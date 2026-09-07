#!/usr/bin/env bash
# Full verification sweep: build the C .so, then run the whole differential
# suite for EVERY feature combination x EVERY build profile, and diff the
# exported symbol sets.
set -uo pipefail
cd "$(dirname "$0")/.."
CRATE=$PWD
CSRC=$CRATE/../c_src
CARGO="cargo --offline"
fail=0

step() { printf '\n=== %s ===\n' "$1"; }

# ---------------------------------------------------------------- C library ---
step "building the C shared library"
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
CSO=$(find "$CSRC/build" -maxdepth 1 -name '*.so' | sort | head -1)
echo "C .so: $CSO"

# ------------------------------------------------- feature combination list ---
# Enumerate the powerset of the [features] table (excluding "default").
mapfile -t FEATURES < <(python3 - "$CRATE/Cargo.toml" <<'PY'
import re, sys, itertools
src = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
if not names:
    print("__default__")
    print("__nodefault__")
else:
    print("__default__")
    print("__nodefault__")
    for r in range(1, len(names) + 1):
        for combo in itertools.combinations(names, r):
            print(",".join(combo))
PY
)
echo "feature combinations: ${FEATURES[*]}"

# ------------------------------------------------------------- the matrix -----
for profile in debug release; do
  for combo in "${FEATURES[@]}"; do
    case "$combo" in
      __default__)   FLAGS=() ; label="default" ;;
      __nodefault__) FLAGS=(--no-default-features) ; label="no-default-features" ;;
      *)             FLAGS=(--no-default-features --features "$combo") ; label="features=$combo" ;;
    esac
    PFLAG=(); [ "$profile" = release ] && PFLAG=(--release)

    step "cargo check  [$profile / $label]"
    $CARGO check "${PFLAG[@]}" "${FLAGS[@]}" 2>&1 | tail -3 || fail=1

    step "cargo build (cdylib)  [$profile / $label]"
    $CARGO build "${PFLAG[@]}" "${FLAGS[@]}" >/dev/null 2>&1 || { echo "BUILD FAILED"; fail=1; continue; }

    RSO="$CRATE/target/$profile/libdequantize_granule_lib.so"
    if [ ! -f "$RSO" ]; then echo "missing $RSO"; fail=1; continue; fi

    step "symbol diff  [$profile / $label]"
    diff <(nm -D --defined-only "$CSO"  | awk '{print $3}' | sort -u) \
         <(nm -D --defined-only "$RSO" | awk '{print $3}' | sort -u) \
      > "${TMPDIR:-/tmp}/symdiff.$$" 2>/dev/null || true
    # only "<" lines matter: symbols the C exports that Rust does not
    MISSING=$(grep '^<' "${TMPDIR:-/tmp}/symdiff.$$" || true)
    rm -f "${TMPDIR:-/tmp}/symdiff.$$"
    if [ -n "$MISSING" ]; then
      echo "MISSING FROM RUST .so:"; echo "$MISSING"; fail=1
    else
      echo "OK — no C symbol is missing from the Rust .so"
    fi

    step "cargo test  [$profile / $label]"
    LOG="${TMPDIR:-/tmp}/cargotest.$$.log"
    if timeout 600 $CARGO test "${PFLAG[@]}" "${FLAGS[@]}" > "$LOG" 2>&1; then
      grep -E '^(     Running|test result)' "$LOG"
    else
      echo "TESTS FAILED"; grep -E 'panicked|FAILED|^test result|^error' "$LOG" | head -40; fail=1
    fi
    rm -f "$LOG"
  done
done

printf '\n===============================\n'
if [ $fail -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES DETECTED"; fi
exit $fail
