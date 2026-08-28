#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under EVERY feature
# combination and EVERY build profile of the Rust cdylib.
#
# Feature combinations are extracted mechanically from Cargo.toml rather than
# hard-coded, so adding a feature later cannot silently go untested.
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
C_BUILD="$ROOT/../c_src/build"

# --------------------------------------------------------------------------
# 1. Build the C shared library (the ground truth).
# --------------------------------------------------------------------------
echo "=== Building the C .so ==="
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=$(find "$C_BUILD" -maxdepth 1 -name '*.so' | head -1)
echo "C .so: $C_SO"
test -n "$C_SO" || { echo "no C .so produced"; exit 1; }

# --------------------------------------------------------------------------
# 2. Enumerate feature combinations from Cargo.toml.
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  python3 - <<'PY'
import re, sys
txt = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name and name != 'default':
            names.append(name)
print('\n'.join(names))
PY
)
# `mapfile` on empty output yields one EMPTY element, not zero elements, so
# filter explicitly — otherwise we would synthesise `--features <nothing>`.
_clean=()
for f in "${FEATURES[@]+"${FEATURES[@]}"}"; do
  [ -n "$f" ] && _clean+=("$f")
done
FEATURES=("${_clean[@]+"${_clean[@]}"}")

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] section: the default (empty) set is the only combination,
  # but exercise the equivalent explicit flags too so the claim is verified
  # rather than assumed.
  COMBOS=("<default>" "--no-default-features" "--all-features")
else
  COMBOS=("<default>" "--no-default-features" "--all-features")
  n=${#FEATURES[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
    done
    COMBOS+=("--no-default-features --features $(
      IFS=,
      echo "${sel[*]}"
    )")
  done
fi

echo
echo "=== Feature combinations to verify (${#COMBOS[@]}) ==="
for c in "${COMBOS[@]}"; do echo "  $c"; done

# --------------------------------------------------------------------------
# 3. cargo check every combination first (fast failure).
# --------------------------------------------------------------------------
echo
echo "=== cargo check across all combinations ==="
FAIL=0
for combo in "${COMBOS[@]}"; do
  flags=""
  [ "$combo" != "<default>" ] && flags="$combo"
  # shellcheck disable=SC2086
  if timeout 300 cargo check --all-targets $flags >/dev/null 2>&1; then
    echo "  OK    cargo check $combo"
  else
    echo "  FAIL  cargo check $combo"
    # shellcheck disable=SC2086
    timeout 300 cargo check --all-targets $flags 2>&1 | tail -20
    FAIL=1
  fi
done

# --------------------------------------------------------------------------
# 4. Run the full suite for every (combination x profile) pair.
# --------------------------------------------------------------------------
SCRATCH="${TMPDIR:-$ROOT/.scratch}"
mkdir -p "$SCRATCH"

echo
echo "=== Differential test suite ==="
for combo in "${COMBOS[@]}"; do
  flags=""
  feat_env=""
  case "$combo" in
    "<default>") ;;
    "--no-default-features") flags="--no-default-features"; feat_env="--no-default-features" ;;
    "--all-features") flags="--all-features" ;;
    *) flags="$combo"; feat_env="${combo#--no-default-features --features }" ;;
  esac
  for profile in debug release; do
    echo
    echo "--- combo: $combo | rust .so profile: $profile ---"
    # Capture the whole run once, then report per-binary results AND the exit
    # code. (Piping into `tail` would mask the exit status via the pipeline and
    # hide most of the test binaries.)
    log="$SCRATCH/run.$$.log"
    # shellcheck disable=SC2086
    C2_C_SO="$C_SO" C2_SO_PROFILE="$profile" C2_FEATURES="$feat_env" \
      timeout 600 cargo test $flags >"$log" 2>&1
    rc=$?
    paste -d' ' \
      <(grep -oE 'Running [^ ]+' "$log" | sed 's|Running ||; s|.*/||') \
      <(grep -E '^test result:' "$log") 2>/dev/null \
      | sed 's/^/    /'
    total=$(grep -cE '^test result: ok' "$log")
    bad=$(grep -cE '^test result: FAILED' "$log")
    echo "    => $total binaries ok, $bad failed, cargo exit=$rc"
    if [ "$rc" -ne 0 ] || [ "$bad" -ne 0 ]; then
      echo "  *** FAILED: combo=$combo profile=$profile ***"
      grep -E 'DIVERGENCE|panicked|^test .* FAILED' "$log" | head -20
      FAIL=1
    fi
    rm -f "$log"
  done
done

# --------------------------------------------------------------------------
# 5. Symbol parity, printed explicitly.
# --------------------------------------------------------------------------
echo
echo "=== Symbol parity (nm -D --defined-only) ==="
RUST_SO=$(find "$ROOT/target/so-under-test" -name 'libpoly_ray_lib.so' -not -path '*/deps/*' | head -1)
[ -n "$RUST_SO" ] || RUST_SO=$(find "$ROOT/target/so-under-test" -name 'libpoly_ray_lib.so' | head -1)
echo "rust .so: $RUST_SO"
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort >"$SCRATCH/c2_c_syms.$$"
nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort >"$SCRATCH/c2_r_syms.$$"
echo "C symbols:    $(wc -l <"$SCRATCH/c2_c_syms.$$")"
echo "Rust symbols: $(wc -l <"$SCRATCH/c2_r_syms.$$")"
echo "--- in C but MISSING from Rust ---"
comm -23 "$SCRATCH/c2_c_syms.$$" "$SCRATCH/c2_r_syms.$$" | tee "$SCRATCH/c2_missing.$$"
echo "--- in Rust but not in C ---"
comm -13 "$SCRATCH/c2_c_syms.$$" "$SCRATCH/c2_r_syms.$$"
if [ -s "$SCRATCH/c2_missing.$$" ]; then
  echo "SYMBOL PARITY FAILED"
  FAIL=1
else
  echo "SYMBOL PARITY OK (0 missing)"
fi
rm -f "$SCRATCH/c2_c_syms.$$" "$SCRATCH/c2_r_syms.$$" "$SCRATCH/c2_missing.$$"

echo
if [ "$FAIL" -eq 0 ]; then
  echo "############ ALL CONFIGURATIONS PASSED ############"
else
  echo "############ FAILURES PRESENT ############"
fi
exit "$FAIL"
