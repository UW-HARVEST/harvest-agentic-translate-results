#!/usr/bin/env bash
# Full verification driver: builds every artifact, enumerates the feature
# combinations mechanically, and runs the differential suites under each one.
#
# Usage: scripts/verify.sh
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
FAIL=0
# Scratch space. /tmp is not writable in every sandbox, so honor TMPDIR and prove
# the directory works before relying on it -- a silently failing redirect would
# otherwise turn a real check into a vacuous PASS.
TMP=${TMPDIR:-/tmp}/verify-$$
mkdir -p "$TMP" || { echo "cannot create scratch dir $TMP" >&2; exit 2; }
if ! : > "$TMP/.probe" 2>/dev/null; then
  echo "scratch dir $TMP is not writable; set TMPDIR to a writable path" >&2; exit 2
fi
trap 'rm -rf "$TMP"' EXIT
step() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "Build the C artifacts (executable via CMake, plus a shared object)"
mkdir -p c_src/build
( cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "c_src/build/driver" || bad "cmake build"

mkdir -p capi_build
gcc -shared -fPIC -O0 -o capi_build/libdriver_c.so c_src/src/main.c \
  && ok "capi_build/libdriver_c.so" || bad "gcc -shared"

# ---------------------------------------------------------------------------
step "Enumerate build configurations"
FEATURES=$(python3 - <<'PY'
import re
t=open("Cargo.toml").read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', t, re.M|re.S)
body=m.group(1) if m else ""
f=[l.split('=')[0].strip() for l in body.splitlines()
   if l.strip() and not l.strip().startswith('#')]
print(" ".join(f))
PY
)
if [ -z "$FEATURES" ]; then
  ok "no cargo features declared -> exactly 1 configuration"
else
  ok "features: $FEATURES"
fi
if grep -qE '#\s*(if|ifdef|ifndef|elif)' c_src/src/main.c; then
  bad "c_src has preprocessor conditionals not reflected in CONFIGS.md"
else
  ok "c_src/src/main.c has no #ifdef -> no C-side configuration axis"
fi
if grep -qE '^\s*(option|if)\(' c_src/CMakeLists.txt; then
  bad "CMakeLists.txt has options not reflected in CONFIGS.md"
else
  ok "CMakeLists.txt has no options"
fi

# Build the full combination list (powerset of declared features; empty if none).
COMBOS=$(python3 - "$FEATURES" <<'PY'
import sys, itertools
f=[x for x in sys.argv[1].split() if x]
if not f:
    print("")   # the single default configuration
else:
    for r in range(len(f)+1):
        for c in itertools.combinations(f, r):
            print(",".join(c))
PY
)

# ---------------------------------------------------------------------------
step "cargo check every configuration"
while IFS= read -r combo; do
  label=${combo:-<none>}
  if cargo check --no-default-features --features "$combo" --all-targets >/dev/null 2>&1; then
    ok "cargo check --features '$label'"
  else
    bad "cargo check --features '$label'"
  fi
done <<< "$COMBOS"

# ---------------------------------------------------------------------------
step "Symbol parity (nm -D)"
cargo build --release >/dev/null 2>&1 || bad "cargo build --release"
CSYMS=$TMP/c_syms; RSYMS=$TMP/r_syms
nm -D --defined-only capi_build/libdriver_c.so | awk '$2 ~ /^[TtDBRWi]$/ {print $3}' \
  | sort -u > "$CSYMS"
nm -D --defined-only target/release/libdriver.so | awk '$2 ~ /^[TtDBRWi]$/ {print $3}' \
  | sort -u > "$RSYMS"
NC=$(wc -l < "$CSYMS"); NR=$(wc -l < "$RSYMS")
if [ "$NC" -eq 0 ]; then
  bad "could not read any exported symbols from the C .so (nm failed?)"
elif [ "$NR" -eq 0 ]; then
  bad "could not read any exported symbols from the Rust .so (nm failed?)"
else
  MISSING=$(comm -23 "$CSYMS" "$RSYMS")
  if [ -z "$MISSING" ]; then
    ok "0 of $NC C symbols missing from the Rust .so"
  else
    bad "missing from Rust .so: $(echo "$MISSING" | tr '\n' ' ')"
  fi
  # Sanity: the C .so must export exactly the five external functions.
  EXPECT="bad good main printIntLine printLine"
  GOT=$(tr '\n' ' ' < "$CSYMS" | sed 's/ *$//')
  [ "$GOT" = "$EXPECT" ] && ok "C .so exports exactly: $EXPECT" \
                         || bad "C .so exports '$GOT', expected '$EXPECT'"
  for hidden in goodG2B goodB2G; do
    if grep -qx "$hidden" "$CSYMS" || grep -qx "$hidden" "$RSYMS"; then
      bad "$hidden is static in C and must not be exported"
    else
      ok "$hidden correctly not exported"
    fi
  done
fi
# Both objects must load with every relocation resolved.
for lib in capi_build/libdriver_c.so target/release/libdriver.so; do
  python3 - "$lib" <<'PY' >/dev/null 2>&1 && ok "dlopen(RTLD_NOW) $lib" || bad "dlopen $lib"
import ctypes, sys
h=ctypes.CDLL(sys.argv[1], mode=ctypes.RTLD_LOCAL)
for s in ("printLine","printIntLine","bad","good","main"):
    getattr(h, s)
PY
done

# ---------------------------------------------------------------------------
step "Differential test suites, per configuration"
while IFS= read -r combo; do
  label=${combo:-<none>}
  printf '  --- features: %s ---\n' "$label"
  cargo build --release --no-default-features --features "$combo" >/dev/null 2>&1 \
    || bad "cargo build --features '$label'"
  LOG=$TMP/test-${label//[^a-zA-Z0-9]/_}.log
  timeout 900 cargo test --release --no-default-features --features "$combo" > "$LOG" 2>&1
  RC=$?
  PASSED=$(grep -cE '^test .* \.\.\. ok$' "$LOG")
  if [ "$RC" -ne 0 ] || grep -qE '^test result: FAILED' "$LOG"; then
    bad "cargo test --features '$label' (rc=$RC, $PASSED passed)"
    grep -E '^(test .*FAILED|---- |thread .* panicked)' "$LOG" | head -25
  elif [ "$PASSED" -eq 0 ]; then
    bad "cargo test --features '$label' ran 0 tests -- the suite is vacuous"
  else
    ok "cargo test --features '$label' ($PASSED tests passed)"
  fi
done <<< "$COMBOS"

# ---------------------------------------------------------------------------
step "Result"
if [ "$FAIL" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit $FAIL
