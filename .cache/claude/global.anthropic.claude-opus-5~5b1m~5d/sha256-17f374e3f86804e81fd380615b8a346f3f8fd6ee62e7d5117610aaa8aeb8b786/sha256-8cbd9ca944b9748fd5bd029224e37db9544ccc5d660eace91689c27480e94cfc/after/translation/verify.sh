#!/usr/bin/env bash
# Full C-vs-Rust verification driver (Phases A-D).
#
#   ./verify.sh
#
# 1. builds the C shared library
# 2. builds the Rust cdylib in every profile
# 3. Phase D symbol-parity diff (must be empty)
# 4. runs the whole differential suite against BOTH the release and the debug
#    Rust .so, for EVERY cargo feature combination
set -uo pipefail

cd "$(dirname "$0")"
CRATE="$PWD"
CSRC="$CRATE/../c_src"
fail=0
LOGDIR="$PWD/target/verify-logs"
mkdir -p "$LOGDIR"
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok] %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=1; }

# --------------------------------------------------------------------------
step "1. build the C shared library"
mkdir -p "$CSRC/build"
( cd "$CSRC/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$LOGDIR/c-build.log" 2>&1 \
  && ok "C .so built" || { bad "C build failed (see target/verify-logs/c-build.log)"; exit 1; }
C_SO=$(find "$CSRC/build" -maxdepth 1 -name '*.so' | head -n1)
ok "C .so = $C_SO"

# --------------------------------------------------------------------------
step "2. enumerate cargo feature combinations"
# Every subset of the [features] table, plus the default and no-default builds.
mapfile -t FEATURES < <(python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name != 'default':
                feats.append(name)
import itertools
print('__default__')
print('__none__')
for r in range(1, len(feats) + 1):
    for combo in itertools.combinations(feats, r):
        print(','.join(combo))
PY
)
printf '  feature combinations: %s\n' "${FEATURES[*]}"

flags_for() {
  case "$1" in
    __default__) echo "" ;;
    __none__)    echo "--no-default-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

# --------------------------------------------------------------------------
step "3. build the Rust cdylib + symbol parity, per feature combination"
for combo in "${FEATURES[@]}"; do
  read -r -a FL <<<"$(flags_for "$combo")"
  for profile in release debug; do
    PF=(); [ "$profile" = release ] && PF=(--release)
    if cargo build --offline "${PF[@]}" "${FL[@]}" >"$LOGDIR/build-$profile.log" 2>&1; then
      ok "cargo build $profile [$combo]"
    else
      bad "cargo build $profile [$combo] (see target/verify-logs/build-$profile.log)"
      continue
    fi
    RS_SO="$CRATE/target/$profile/libarr_del_lib.so"
    d=$(diff <(nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort) \
             <(nm -D --defined-only "$RS_SO" | awk '{print $3}' | sort))
    if [ -z "$d" ]; then
      n=$(nm -D --defined-only "$C_SO" | wc -l)
      ok "symbol parity $profile [$combo]: $n/$n symbols, diff empty"
    else
      bad "symbol diff $profile [$combo]:"$'\n'"$d"
    fi
    # No undefined non-libc / non-unwind symbols in the Rust .so.
    u=$(nm -D -u "$RS_SO" | awk '{print $NF}' \
        | grep -v '@GLIBC' | grep -v '@GCC' \
        | grep -vE '^(_ITM_|__gmon_start__|__cxa_|_Unwind_|gettid|statx)' || true)
    if [ -z "$u" ]; then
      ok "no undefined non-libc symbols $profile [$combo]"
    else
      bad "undefined non-libc symbols $profile [$combo]: $u"
    fi
  done
done

# --------------------------------------------------------------------------
step "4. differential test suite, per feature combination x per profile .so"
for combo in "${FEATURES[@]}"; do
  read -r -a FL <<<"$(flags_for "$combo")"
  for profile in release debug; do
    export TRANSLATION_SO="$CRATE/target/$profile/libarr_del_lib.so"
    log="$LOGDIR/test-$combo-$profile.log"
    if timeout 600 cargo test --offline "${FL[@]}" >"$log" 2>&1; then
      res=$(grep -c '^test result: ok' "$log")
      cnt=$(grep -oP '^test result: ok\. \K[0-9]+' "$log" | awk '{t+=$1} END {print t}')
      ok "cargo test [$combo] against $profile .so: $cnt tests passed ($res binaries)"
    else
      bad "cargo test [$combo] against $profile .so (see $log)"
      tail -n 30 "$log"
    fi
    unset TRANSLATION_SO
  done
done

# --------------------------------------------------------------------------
step "5. binary executable check"
if grep -q 'add_executable' "$CSRC/CMakeLists.txt"; then
  bad "the C project builds an executable; stdout comparison is required"
else
  ok "no add_executable in c_src/CMakeLists.txt and no [[bin]] in Cargo.toml -> no driver binary to diff"
fi

printf '\n=====================================\n'
if [ "$fail" -eq 0 ]; then
  echo "ALL PHASES PASSED"
else
  echo "FAILURES PRESENT"
fi
exit "$fail"
