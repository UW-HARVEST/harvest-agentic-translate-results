#!/usr/bin/env bash
# Full verification matrix: build the C reference and the Rust cdylib, diff the
# exported symbol tables, then run every differential test under every feature
# combination and both cargo profiles.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
FAIL=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf 'PASS  %s\n' "$*"; }
bad()  { printf 'FAIL  %s\n' "$*"; FAIL=1; }

step "Build C reference shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || bad "C build"
C_SO="$(ls "$ROOT"/c_src/build/*.so)"
echo "C  .so: $C_SO"

step "Enumerate feature combinations from Cargo.toml"
# Cross-product of all declared features, always including the default set and
# --no-default-features. `cargo read-manifest` is the source of truth.
mapfile -t COMBOS < <(cargo read-manifest 2>/dev/null | python3 -c '
import json,sys,itertools
feats=sorted(k for k in json.load(sys.stdin).get("features",{}) if k!="default")
print("--default")                       # default feature set
print("--no-default-features")           # empty feature set
for r in range(1,len(feats)+1):
    for c in itertools.combinations(feats,r):
        print("--no-default-features --features "+",".join(c))
        print("--features "+",".join(c))
')
printf '%s\n' "${COMBOS[@]}"

for PROFILE in debug release; do
  PROF_FLAG=""
  [ "$PROFILE" = release ] && PROF_FLAG="--release"
  for COMBO in "${COMBOS[@]}"; do
    FLAGS="$PROF_FLAG"
    [ "$COMBO" != "--default" ] && FLAGS="$PROF_FLAG $COMBO"
    LABEL="$PROFILE / ${COMBO}"

    step "cargo build [$LABEL]"
    if ! timeout 600 cargo build $FLAGS >/dev/null 2>&1; then
      bad "build [$LABEL]"; continue
    fi
    RUST_SO="target/$PROFILE/libpoly_ray_lib.so"
    [ -f "$RUST_SO" ] || { bad "missing $RUST_SO [$LABEL]"; continue; }

    step "symbol diff [$LABEL]"
    nm -D --defined-only "$C_SO"   | awk '$2=="T"{print $3}' | sort > /tmp/c_syms.txt
    nm -D --defined-only "$RUST_SO" | awk '$2=="T"{print $3}' | sort > /tmp/r_syms.txt
    MISSING="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
    if [ -n "$MISSING" ]; then
      bad "symbols missing from Rust .so [$LABEL]:"; echo "$MISSING"
    else
      ok "symbol parity ($(wc -l < /tmp/c_syms.txt) symbols) [$LABEL]"
    fi

    step "cargo test [$LABEL] DIFF_ITERS=${DIFF_ITERS:-default}"
    if C_SO="$C_SO" RUST_SO="$(pwd)/$RUST_SO" timeout 600 cargo test $FLAGS 2>&1 | tail -25; then
      ok "tests [$LABEL]"
    else
      bad "tests [$LABEL]"
    fi
  done
done

step "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
