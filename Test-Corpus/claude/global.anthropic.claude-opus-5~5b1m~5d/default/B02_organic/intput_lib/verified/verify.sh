#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination + both profiles.
# Usage: bash verify.sh      (from the translation/ directory)
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
TMP=${TMPDIR:-/tmp}
mkdir -p "$TMP"
FAIL=0
note() { printf '\n== %s\n' "$*"; }
ok()   { printf '   PASS %s\n' "$*"; }
bad()  { printf '   FAIL %s\n' "$*"; FAIL=1; }

CSO=$(ls "$ROOT"/c_src/build/*.so 2>/dev/null | head -1)
if [ -z "${CSO:-}" ]; then
  note "building the C shared library"
  ( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null ) || { bad "cmake build"; exit 1; }
  CSO=$(ls "$ROOT"/c_src/build/*.so | head -1)
fi
echo "C   .so: $CSO"

# ---------------------------------------------------------------------------
# 1. enumerate the feature combinations declared in Cargo.toml
# ---------------------------------------------------------------------------
FEATLIST=$(python3 ./list_features.py)
FEATURES=()
while IFS= read -r line; do
  [ -n "$line" ] && FEATURES+=("$line")
done <<< "$FEATLIST"
NFEAT=${#FEATURES[@]}
echo "declared features ($NFEAT): ${FEATURES[*]:-<none>}"

COMBOS=("" "--no-default-features" "--all-features")
if [ "$NFEAT" -gt 0 ]; then
  total=$((1 << NFEAT))
  for ((mask = 0; mask < total; mask++)); do
    sel=""
    for ((i = 0; i < NFEAT; i++)); do
      if (( (mask >> i) & 1 )); then sel="$sel,${FEATURES[$i]}"; fi
    done
    if [ -z "$sel" ]; then
      COMBOS+=("--no-default-features")
    else
      COMBOS+=("--no-default-features --features ${sel#,}")
    fi
  done
fi
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')
echo "feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. per (profile x combination): check, build, nm -D diff, differential tests
# ---------------------------------------------------------------------------
for PROFILE in dev release; do
  if [ "$PROFILE" = "release" ]; then PROF_FLAG="--release"; OUTDIR="target/release";
  else PROF_FLAG=""; OUTDIR="target/debug"; fi
  for COMBO in "${COMBOS[@]}"; do
    LABEL="profile=$PROFILE features='${COMBO:-<default>}'"
    note "$LABEL"

    if ! timeout 600 cargo check $PROF_FLAG $COMBO > "$TMP/chk" 2>&1; then
      bad "cargo check ($LABEL)"; tail -20 "$TMP/chk"; continue
    fi
    ok "cargo check"

    if ! timeout 600 cargo build $PROF_FLAG $COMBO > "$TMP/bld" 2>&1; then
      bad "cargo build ($LABEL)"; tail -20 "$TMP/bld"; continue
    fi
    RSO="$(pwd)/$OUTDIR/libintput_lib.so"
    [ -f "$RSO" ] || { bad "missing $RSO"; continue; }
    ok "cargo build -> $OUTDIR/libintput_lib.so"

    DIFF=$(diff <(nm -D --defined-only "$CSO" | awk '{print $3}' | sort) \
                <(nm -D --defined-only "$RSO" | awk '{print $3}' | sort))
    if [ -n "$DIFF" ]; then
      bad "nm -D symbol diff is NOT empty ($LABEL)"; echo "$DIFF"
    else
      NSYM=$(nm -D --defined-only "$CSO" | wc -l | tr -d ' ')
      ok "nm -D parity: $NSYM/$NSYM symbols, diff empty"
    fi

    UNDEF=$(nm -D -u "$RSO" | awk '{print $2}' \
            | grep -v '@GLIBC' | grep -v '^_Unwind_' | grep -v '^_ITM_' \
            | grep -v '^__gmon_start__$' | grep -v '^__cxa_' | grep -v '^$' || true)
    if [ -n "$UNDEF" ]; then
      bad "undefined non-libc symbols: $(echo "$UNDEF" | tr '\n' ' ')"
    else
      ok "0 undefined non-libc symbols"
    fi

    if RUST_DS_SO="$RSO" C_DS_SO="$CSO" \
         timeout 600 cargo test $COMBO --tests -- --test-threads=1 > "$TMP/tst" 2>&1; then
      PASSED=$(grep -c '\.\.\. ok$' "$TMP/tst")
      ok "cargo test against $OUTDIR .so: $PASSED tests passed"
    else
      bad "cargo test ($LABEL, .so=$OUTDIR)"; tail -40 "$TMP/tst"
    fi
  done
done

# ---------------------------------------------------------------------------
# 3. extra robustness matrix: the same suite against differently-optimised C
#    builds.  `lib.c` contains signed-overflow UB (`d[3] << 24` in the siphash
#    tail) and a shift-count-overflow UB (`512u << (block>>1)`), so a different
#    -O level is a genuinely different oracle.  Built outside c_src/ (never
#    modified) into translation/target/.
# ---------------------------------------------------------------------------
declare -A CFLAGS_VARIANTS=(
  [cbuild_O0]=""
  [cbuild_O1]="-O1"
  [cbuild_O2]="-O2"
  [cbuild_O3]="-O3"
  [cbuild_Os]="-Os"
  [cbuild_O2_strict]="-O2 -fstrict-aliasing -fstrict-overflow"
  [cbuild_O0g]="-O0 -g -fno-strict-aliasing"
)
for VAR in "${!CFLAGS_VARIANTS[@]}"; do
  DIR="target/$VAR"
  if [ ! -f "$DIR/libharvest-work-uAHqBm.so" ]; then
    cmake -S ../c_src -B "$DIR" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
          -DCMAKE_C_FLAGS="${CFLAGS_VARIANTS[$VAR]}" >/dev/null 2>&1
    cmake --build "$DIR" >/dev/null 2>&1
  fi
  CSO_VAR=$(ls "$DIR"/*.so 2>/dev/null | head -1)
  if [ -z "${CSO_VAR:-}" ]; then bad "could not build C variant $VAR"; continue; fi
  for prof in debug release; do
    note "C variant '$VAR' (CFLAGS='${CFLAGS_VARIANTS[$VAR]}') vs rust/$prof"
    RSO="$(pwd)/target/$prof/libintput_lib.so"
    [ -f "$RSO" ] || { bad "missing $RSO"; continue; }
    DIFF=$(diff <(nm -D --defined-only "$CSO_VAR" | awk '{print $3}' | sort) \
                <(nm -D --defined-only "$RSO" | awk '{print $3}' | sort))
    if [ -n "$DIFF" ]; then bad "nm -D diff vs $VAR"; echo "$DIFF"; else ok "nm -D parity"; fi
    if C_DS_SO="$CSO_VAR" RUST_DS_SO="$RSO" \
         timeout 600 cargo test --tests -- --test-threads=1 > "$TMP/tst" 2>&1; then
      ok "cargo test: $(grep -c '\.\.\. ok$' "$TMP/tst") tests passed"
    else
      bad "cargo test (C=$VAR rust=$prof)"; grep -E 'FAILED|panicked' "$TMP/tst" | head -20
    fi
  done
done

note "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit $FAIL
