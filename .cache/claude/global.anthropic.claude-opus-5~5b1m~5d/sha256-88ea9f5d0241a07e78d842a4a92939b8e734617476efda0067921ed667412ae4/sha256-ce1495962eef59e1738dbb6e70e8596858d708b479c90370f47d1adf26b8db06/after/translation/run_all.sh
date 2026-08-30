#!/usr/bin/env bash
# Runs the complete differential verification across every build configuration.
#
#   ./run_all.sh
#
# 1. (re)builds the C shared library
# 2. enumerates the crate's feature combinations from Cargo.toml
# 3. runs the whole test suite for each combination, in both the dev and the
#    release profile (the release profile is the one that carries
#    `panic = "abort"` and full optimisation, so it is a genuinely different
#    code path for the exported wrappers)
# 4. re-checks the C-vs-Rust `nm -D` symbol diff for each configuration
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
fail=0

echo "== building the C shared library =="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
c_so="$root/c_src/build/libdriver.so"

# ---------------------------------------------------------------------------
# Feature combinations. This crate declares no [features] table, so the set of
# combinations is exactly {default, --no-default-features}; the loop below is
# generated from Cargo.toml so it keeps working if features are ever added.
# ---------------------------------------------------------------------------
features=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/{print $1}' "$here/Cargo.toml" \
           | grep -v '^default$' | tr '\n' ' ')

combos=("" "--no-default-features")
for f in $features; do
  combos+=("--no-default-features --features $f")
  combos+=("--features $f")
done
if [ -n "$features" ]; then
  all=$(echo "$features" | tr ' ' ',' | sed 's/,$//')
  combos+=("--no-default-features --features $all")
  combos+=("--features $all")
fi

for profile in "" "--release"; do
  for combo in "${combos[@]}"; do
    label="cargo test ${profile:-<dev>} ${combo:-<default features>}"
    echo
    echo "== $label =="
    ( cd "$here" && cargo build $profile $combo >/dev/null 2>&1 )
    ( cd "$here" && cargo build --examples $profile $combo >/dev/null 2>&1 )
    log="$(mktemp)"
    ( cd "$here" && cargo test $profile $combo >"$log" 2>&1 ); st=$?
    grep -E "^(running|test result|test .* FAILED)|panicked|mismatch" "$log" | tail -n 30
    if [ "$st" != 0 ]; then echo "FAILED: $label"; tail -n 40 "$log"; fail=1; fi
    rm -f "$log"

    # symbol parity for this configuration
    prof_dir=$([ -n "$profile" ] && echo release || echo debug)
    rs_so="$here/target/$prof_dir/libdriver.so"
    if [ -f "$rs_so" ]; then
      d=$(diff <(nm -D --defined-only "$c_so"  | awk '{print $3}' | sort) \
               <(nm -D --defined-only "$rs_so" | awk '{print $3}' | sort))
      if [ -n "$d" ]; then echo "SYMBOL DIFF for $label:"; echo "$d"; fail=1;
      else echo "symbol diff: empty (OK)"; fi
    else
      echo "WARNING: $rs_so not built"; fail=1
    fi
  done
done

echo
if [ "$fail" = 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
