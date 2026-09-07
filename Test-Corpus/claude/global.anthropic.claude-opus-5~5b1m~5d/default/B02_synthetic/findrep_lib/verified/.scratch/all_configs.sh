#!/usr/bin/env bash
# Run the full differential suite under EVERY build configuration:
#   * every feature combination declared in Cargo.toml (this crate declares
#     none, so the set is {default} == {no-default-features} == {all-features})
#   * both cargo profiles (debug and release), because the Rust `.so` under
#     test differs between them (release sets panic = "abort", debug enables
#     integer-overflow checks).
set -u
cd "$(dirname "$0")/.."

# Mechanically extract feature names rather than assuming there are none.
FEATURES=$(python3 - <<'PY'
import re
s=open("Cargo.toml").read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M|re.S)
if not m: print("", end="")
else:
    names=[l.split('=')[0].strip() for l in m.group(1).splitlines()
           if '=' in l and not l.strip().startswith('#')]
    print(" ".join(n for n in names if n!="default"), end="")
PY
)
echo "declared features: [${FEATURES:-none}]"

# Build the combination list.
COMBOS=("--features=")                      # default feature set
COMBOS+=("--no-default-features")
if [ -n "$FEATURES" ]; then
  COMBOS+=("--all-features")
  for f in $FEATURES; do
    COMBOS+=("--no-default-features --features $f")
  done
fi

FAIL=0
for profile in debug release; do
  if [ "$profile" = "release" ]; then PF="--release"; else PF=""; fi
  for combo in "${COMBOS[@]}"; do
    echo "=============================================================="
    echo ">>> profile=$profile combo='$combo'"
    # Rebuild the cdylib for this exact configuration and point the harness at
    # it, so the tests load the .so belonging to this configuration.
    if ! cargo build $PF --offline $combo -q 2>&1 | tail -5; then
      echo "BUILD FAILED"; FAIL=1; continue
    fi
    SO="$(pwd)/target/$profile/libfindrep_lib.so"
    if [ ! -f "$SO" ]; then echo "MISSING $SO"; FAIL=1; continue; fi
    if RUST_SO_PATH="$SO" timeout 600 cargo test $PF --offline $combo 2>&1 \
         | grep -E 'test result|FAILED|^error'; then :; fi
    if ! RUST_SO_PATH="$SO" timeout 600 cargo test $PF --offline $combo >/dev/null 2>&1; then
      echo "*** TESTS FAILED for profile=$profile combo='$combo' ***"
      FAIL=1
    fi
  done
done

echo "=============================================================="
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit $FAIL
