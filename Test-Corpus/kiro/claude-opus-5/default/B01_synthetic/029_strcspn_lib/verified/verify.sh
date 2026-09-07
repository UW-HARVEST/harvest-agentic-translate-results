#!/usr/bin/env bash
# Phase D driver: symbol parity + the full feature-combination sweep.
#
# Run from the repository root or from translation/; it locates both itself.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
CRATE="$HERE"
C_SO="$ROOT/c_src/build/libdriver.so"
RUST_SO="$CRATE/target/release/libdriver.so"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# ---------------------------------------------------------------------------
note "build C shared library"
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l "$C_SO"

# ---------------------------------------------------------------------------
note "enumerate feature combinations from Cargo.toml"
# Mechanically derive the feature list; the powerset is enumerated, plus the
# default build and the bare --no-default-features build.
mapfile -t FEATURES < <(
  cd "$CRATE" && cargo metadata --no-deps --format-version 1 2>/dev/null \
    | python3 -c 'import json,sys; print("\n".join(json.load(sys.stdin)["packages"][0]["features"]))' \
    | grep -v '^$'
)
echo "declared features: ${FEATURES[*]:-<none>}"

COMBOS=()
COMBOS+=("--DEFAULT--")           # plain `cargo test --release`
COMBOS+=("--NODEFAULT--")         # `--no-default-features`
n=${#FEATURES[@]}
if (( n > 0 )); then
  for (( mask=1; mask < (1<<n); mask++ )); do
    combo=""
    for (( i=0; i<n; i++ )); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    --DEFAULT--)    flags=();                                              label="default" ;;
    --NODEFAULT--)  flags=(--no-default-features);                         label="no-default-features" ;;
    *)              flags=(--no-default-features --features "$combo");     label="features=$combo" ;;
  esac

  note "combination: $label — build release cdylib"
  ( cd "$CRATE" && timeout 600 cargo build --release "${flags[@]}" ) || { echo "BUILD FAILED ($label)"; fail=1; continue; }

  note "combination: $label — symbol parity (nm -D)"
  c_syms=$(nm -D --defined-only "$C_SO"    | awk '$2=="T"{print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  echo "C exports:    $(echo "$c_syms" | grep -c . )"
  echo "Rust exports: $(echo "$r_syms" | grep -c . )"
  if [[ -n "$missing" ]]; then
    echo "MISSING FROM RUST .so ($label):"; echo "$missing"; fail=1
  else
    echo "symbol diff: EMPTY (0 missing)"
  fi

  # No dangling non-libc/non-unwinder imports in the Rust .so.
  bad=$(nm -D --undefined-only "$RUST_SO" \
        | awk '{print $NF}' \
        | grep -vE '^(_ITM_|_Unwind_|__cxa_|__gmon_start__|__tls_get_addr|__errno_location)' \
        | grep -vE '@GLIBC|@GCC' \
        | grep -vE '^(statx|gettid)$' \
        | grep -v '^$')
  if [[ -n "$bad" ]]; then
    echo "UNRESOLVED NON-LIBC IMPORTS ($label):"; echo "$bad"; fail=1
  else
    echo "undefined symbols: all libc/unwinder"
  fi

  note "combination: $label — Phase B + Phase C differential tests"
  # --test-threads=1: the harness swaps the process-global stdout descriptor to
  # capture what each .so prints, so libtest must not write concurrently.
  ( cd "$CRATE" && timeout 600 cargo test --release "${flags[@]}" -- --test-threads=1 ) \
    || { echo "TESTS FAILED ($label)"; fail=1; }
done

# ---------------------------------------------------------------------------
note "binary executable check"
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null; then
  echo "C CMakeLists declares an executable — stdout comparison required"; fail=1
else
  echo "c_src/CMakeLists.txt declares no add_executable"
fi
if grep -q '^\[\[bin\]\]' "$CRATE/Cargo.toml" || [[ -f "$CRATE/src/main.rs" ]]; then
  echo "Rust crate declares a binary — stdout comparison required"; fail=1
else
  echo "translation/ declares no [[bin]] and has no src/main.rs"
fi
echo "=> no driver executable on either side; stdout equivalence is covered by the .so tests"

note "RESULT"
if (( fail )); then echo "VERIFICATION FAILED"; exit 1; fi
echo "ALL CHECKS PASSED"
