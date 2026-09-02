#!/usr/bin/env bash
# Build BOTH shared objects, then run the differential suite over every
# feature combination.
#
# `cargo test` alone is not sufficient: it links the test binary but not the
# `cdylib`, so the suite would load a stale `libdriver.so`. Always go through
# this script (or `cargo build --release && cargo test --release`).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

# --- 1. the C reference library ------------------------------------------
if [[ ! -f "$root/c_src/build/libdriver.so" ]]; then
  echo "== building the C reference library =="
  mkdir -p "$root/c_src/build"
  ( cd "$root/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . )
fi

# --- 2. enumerate feature combinations from Cargo.toml -------------------
# The crate declares no [features] table, so this yields the single `default`
# configuration; the loop is written generically so it keeps working if
# features are ever added.
mapfile -t features < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' "$here/Cargo.toml" \
    | grep -v '^default$' || true
)

combos=("")                       # default features
combos+=("--no-default-features")  # nothing enabled
for f in "${features[@]:-}"; do
  [[ -n "$f" ]] && combos+=("--no-default-features --features $f")
done
if (( ${#features[@]} > 1 )); then
  combos+=("--no-default-features --features $(IFS=,; echo "${features[*]}")")
fi

# --- 3. build + test each combination -----------------------------------
rc=0
for combo in "${combos[@]}"; do
  label="${combo:-<default features>}"
  echo
  echo "================================================================"
  echo "== feature combination: $label"
  echo "================================================================"
  ( cd "$here"
    # shellcheck disable=SC2086
    timeout 600 cargo build --release $combo
    # shellcheck disable=SC2086
    timeout 600 cargo test  --release $combo
  ) || rc=1
done

# --- 4. symbol parity, straight from nm ---------------------------------
echo
echo "== nm -D symbol diff (C vs Rust) =="
c_syms=$(nm -D --defined-only "$root/c_src/build/libdriver.so" \
           | awk '$2 ~ /^[TtWDBRi]$/ {print $3}' | sort -u)
r_syms=$(nm -D --defined-only "$here/target/release/libdriver.so" \
           | awk '$2 ~ /^[TtWDBRi]$/ && $3 !~ /^_/ {print $3}' | sort -u)
if diff <(echo "$c_syms") <(echo "$r_syms"); then
  echo "symbol diff is EMPTY"
else
  echo "SYMBOL DIFF NON-EMPTY"
  rc=1
fi

echo
echo "== unresolved relocations in the Rust .so (ldd -r) =="
if ldd -r "$here/target/release/libdriver.so" 2>&1 | grep -i "undefined symbol"; then
  echo "UNRESOLVED SYMBOLS PRESENT"
  rc=1
else
  echo "none"
fi

# --- 5. cross-checks: other build profiles on both sides -----------------
# The Rust must match the C regardless of how either side was compiled. A
# divergence that only shows up at one optimisation level would mean the
# translation is modelling a compiler artefact rather than the C semantics.
bin=$(ls -t "$here"/target/release/deps/differential-* 2>/dev/null \
        | grep -v '\.d$' | head -1)
if [[ -n "$bin" ]]; then
  echo
  echo "== cross-check: Rust DEBUG cdylib vs the C reference =="
  ( cd "$here" && timeout 600 cargo build >/dev/null )
  DRIVER_RUST_SO="$here/target/debug/libdriver.so" timeout 600 "$bin" \
    | grep -E '^result:|^FAIL' || rc=1

  for opt in O2 O3; do
    alt="$(mktemp -d)/libdriver_$opt.so"
    # Built outside c_src/, which is never modified.
    if gcc "-$opt" -fPIC -shared -I"$root/c_src/include" \
           "$root/c_src/src/driver.c" -o "$alt" 2>/dev/null; then
      echo
      echo "== cross-check: C built with -$opt vs the Rust release cdylib =="
      DRIVER_C_SO="$alt" timeout 600 "$bin" | grep -E '^result:|^FAIL' || rc=1
    fi
  done
fi

exit $rc
