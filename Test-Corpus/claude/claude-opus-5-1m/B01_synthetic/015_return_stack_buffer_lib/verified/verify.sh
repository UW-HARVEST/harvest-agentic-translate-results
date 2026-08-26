#!/usr/bin/env bash
# Full differential-verification sweep: Phase A artifacts -> symbol parity ->
# Phase B/C tests, repeated for EVERY feature combination in Cargo.toml.
#
# Usage: ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"

# Sandboxes may make /tmp read-only; honour TMPDIR.
LOG="${TMPDIR:-/tmp}"

RED=$'\033[31m'; GRN=$'\033[32m'; BLD=$'\033[1m'; RST=$'\033[0m'
fails=0
step() { printf '\n%s== %s ==%s\n' "$BLD" "$1" "$RST"; }
ok()   { printf '%s  PASS%s %s\n' "$GRN" "$RST" "$1"; }
bad()  { printf '%s  FAIL%s %s\n' "$RED" "$RST" "$1"; fails=$((fails+1)); }

# ---------------------------------------------------------------------------
# 0. Build the C reference shared library
# ---------------------------------------------------------------------------
step "Building C reference library"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > $LOG/cbuild.log 2>&1 \
  || { cat $LOG/cbuild.log; bad "C build"; exit 1; }
C_SO=c_src/build/libdriver.so
[[ -f $C_SO ]] && ok "C .so at $C_SO" || { bad "C .so missing"; exit 1; }

# ---------------------------------------------------------------------------
# 1. Enumerate every valid feature combination straight out of Cargo.toml
# ---------------------------------------------------------------------------
step "Enumerating feature combinations from Cargo.toml"
mapfile -t COMBOS < <(python3 - <<'PY'
import tomllib, itertools
m = tomllib.load(open("Cargo.toml", "rb"))
feats = [k for k in m.get("features", {}) if k != "default"]
feats += [k for k, v in m.get("dependencies", {}).items()
          if isinstance(v, dict) and v.get("optional")]
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print(",".join(c))
PY
)
printf 'combinations: %s\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do printf '  - %s\n' "${c:-<none>}"; done

# ---------------------------------------------------------------------------
# 2. Per-combination: check -> build -> symbol diff -> Phase B/C tests
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"
  step "Feature combination: $label"

  if timeout 600 cargo check --no-default-features --features "$combo" \
       > $LOG/check.log 2>&1; then
    warns=$(grep -c '^warning' $LOG/check.log)
    ok "cargo check ($warns warnings)"
  else
    tail -30 $LOG/check.log; bad "cargo check [$label]"; continue
  fi

  for prof in dev release; do
    if [[ $prof == release ]]; then flag=--release; dir=target/release
    else flag=; dir=target/debug; fi

    if timeout 600 cargo build --no-default-features --features "$combo" $flag \
         > $LOG/build.log 2>&1; then
      ok "cargo build [$prof]"
    else
      tail -30 $LOG/build.log; bad "cargo build [$label/$prof]"; continue
    fi

    # --- symbol parity (Phase D) -----------------------------------------
    R_SO=$dir/libdriver.so
    nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtWwDdBb]$/ {print $3}' \
      | grep -vE '^(_|__)' | sort -u > $LOG/c.syms
    nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TtWwDdBb]$/ {print $3}' \
      | grep -vE '^(_|__)' | sort -u > $LOG/r.syms
    if [[ -s $LOG/c.syms ]] && missing=$(comm -23 $LOG/c.syms $LOG/r.syms) \
       && [[ -z $missing ]]; then
      ok "symbol parity [$prof]: $(wc -l < $LOG/c.syms) C symbols, 0 missing"
    else
      printf 'missing from Rust .so:\n%s\n' "$missing"; bad "symbol parity [$label/$prof]"
    fi

    # --- Phase B + C + D differential tests ------------------------------
    if RUST_TEST_THREADS=1 timeout 600 cargo test \
         --no-default-features --features "$combo" $flag \
         > $LOG/test.log 2>&1; then
      ok "differential tests [$prof]: $(grep -c '^test .* ok$' $LOG/test.log) passed"
    else
      grep -E '^(test .*FAILED|failures:|DIVERGENCE|thread)' $LOG/test.log | head -30
      bad "differential tests [$label/$prof]"
    fi
  done
done

# ---------------------------------------------------------------------------
# 3. Phase A artifacts must exist
# ---------------------------------------------------------------------------
step "Phase A artifacts"
for f in SYMBOLS.md ERRORS.md CONFIGS.md; do
  [[ -s $f ]] && ok "$f ($(wc -l < "$f") lines)" || bad "$f missing/empty"
done

step "Summary"
if (( fails == 0 )); then
  printf '%sALL CHECKS PASSED%s\n' "$GRN" "$RST"
else
  printf '%s%d CHECK(S) FAILED%s\n' "$RED" "$fails" "$RST"
fi
exit $(( fails > 0 ))
