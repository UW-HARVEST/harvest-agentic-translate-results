#!/bin/bash
# ---------------------------------------------------------------------------
# Phase D driver: enumerate every build-time configuration, check it compiles,
# verify dynamic-symbol parity against the C .so, and run the whole
# differential test suite for each one.
#
#   ./check_all_configs.sh
# ---------------------------------------------------------------------------
set -u
cd "$(dirname "$(readlink -f "$0")")" || exit 1
ROOT=$PWD
LOG=${TMPDIR:-/tmp}/allcfg.$$
mkdir -p "$LOG"
rc_all=0
fail() { echo "!! FAIL: $*"; rc_all=1; }

# ---------------------------------------------------------------------------
# 0. Build the C reference shared library
# ---------------------------------------------------------------------------
echo "=== [0] building the C reference .so ==="
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "$LOG/cbuild.log" 2>&1 \
  || { fail "C build"; tail -20 "$LOG/cbuild.log"; exit 1; }
C_SO=$ROOT/c_src/build/libtranslated_rust.so
[ -f "$C_SO" ] || { fail "missing $C_SO"; exit 1; }
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort > "$LOG/c.syms"
echo "    C .so exports $(wc -l < "$LOG/c.syms") symbols"

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination (power set of [features])
# ---------------------------------------------------------------------------
FEATS=$(awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1]!="default" && a[1]!="") print a[1]}' Cargo.toml)
COMBOS=()
if [ -z "$FEATS" ]; then
  echo "=== [1] Cargo.toml declares no [features] ==="
  echo "    -> the only valid feature configurations are the empty set:"
  echo "         (a) default features"
  echo "         (b) --no-default-features"
  COMBOS=("__default__" "__none__")
else
  arr=($FEATS); n=${#arr[@]}
  echo "=== [1] features: ${arr[*]} -> $((1<<n)) combinations ==="
  for ((m=0; m<(1<<n); m++)); do
    ccc=""
    for ((k=0; k<n; k++)); do (( (m>>k) & 1 )) && ccc="$ccc,${arr[k]}"; done
    COMBOS+=("${ccc#,}")
  done
  COMBOS+=("__default__")
fi

flags_for() {
  case "$1" in
    __default__) echo "" ;;
    __none__)    echo "--no-default-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

# ---------------------------------------------------------------------------
# 2. cargo check every combination (lib + all targets)
# ---------------------------------------------------------------------------
echo
echo "=== [2] cargo check for every feature combination ==="
for combo in "${COMBOS[@]}"; do
  f=$(flags_for "$combo")
  timeout 600 cargo check --offline $f --all-targets > "$LOG/check.log" 2>&1
  rc=$?
  printf '    %-24s rc=%s\n' "$combo" "$rc"
  [ $rc -ne 0 ] && { fail "cargo check $combo"; tail -25 "$LOG/check.log"; }
done

# ---------------------------------------------------------------------------
# 3. For every (profile x feature-combo): build, diff symbols, run the suite
# ---------------------------------------------------------------------------
for profile in dev release; do
  case $profile in
    dev)     pflag="";          pdir=debug ;;
    release) pflag="--release"; pdir=release ;;
  esac
  for combo in "${COMBOS[@]}"; do
    f=$(flags_for "$combo")
    echo
    echo "=== [3] profile=$profile features=$combo ==="

    timeout 600 cargo build --offline $pflag $f > "$LOG/build.log" 2>&1 \
      || { fail "build $profile/$combo"; tail -25 "$LOG/build.log"; continue; }
    R_SO=$ROOT/target/$pdir/libhm_geti_lib.so
    [ -f "$R_SO" ] || { fail "missing $R_SO"; continue; }

    # --- symbol parity -----------------------------------------------------
    nm -D --defined-only "$R_SO" | awk '{print $3}' | sort > "$LOG/r.syms"
    missing=$(comm -23 "$LOG/c.syms" "$LOG/r.syms")
    if [ -n "$missing" ]; then
      fail "symbols missing from the Rust .so ($profile/$combo):"
      echo "$missing" | sed 's/^/        /'
    else
      echo "    symbol parity: OK (0 missing of $(wc -l < "$LOG/c.syms"))"
    fi
    # --- undefined non-libc symbols ---------------------------------------
    undef=$(nm -D --undefined-only "$R_SO" | awk '{print $2}' \
            | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^_Unwind_' )
    if [ -n "$undef" ]; then
      fail "unresolved non-libc symbols ($profile/$combo): $undef"
    else
      echo "    undefined non-libc symbols: 0"
    fi

    # --- the whole differential suite -------------------------------------
    DIFF_C_SO=$C_SO timeout 600 cargo test --offline $pflag $f \
        > "$LOG/test.log" 2>&1
    rc=$?
    grep -E '^test result:' "$LOG/test.log" | sed 's/^/    /'
    if [ $rc -ne 0 ]; then
      fail "tests $profile/$combo"
      grep -E '^(test .* FAILED|failures:|thread .* panicked)' -A3 "$LOG/test.log" | head -60
    fi

    # --- cross-check: run the dev-profile tests against THIS .so ----------
    if [ "$profile" = release ]; then
      echo "    cross-check: dev-profile tests against the release .so"
      DIFF_C_SO=$C_SO DIFF_RUST_SO=$R_SO timeout 600 cargo test --offline $f \
          > "$LOG/xtest.log" 2>&1
      rc=$?
      grep -E '^test result:' "$LOG/xtest.log" | sed 's/^/      /'
      [ $rc -ne 0 ] && { fail "cross tests $combo"; \
        grep -E '^(test .* FAILED|failures:)' -A3 "$LOG/xtest.log" | head -40; }
    fi
  done
done

echo
if [ $rc_all -eq 0 ]; then
  echo "############ ALL CONFIGURATIONS PASSED ############"
else
  echo "############ FAILURES PRESENT (see above) ############"
fi
echo "logs in $LOG"
exit $rc_all
