#!/usr/bin/env bash
# Phase D driver: run the whole differential suite under every build
# configuration, and check symbol parity for each resulting .so.
#
# Feature combinations are ENUMERATED FROM Cargo.toml rather than hard-coded, so
# this keeps working if features are ever added.
set -u
cd "$(dirname "$0")"

C_SO=../c_src/build/libdriver.so
if [[ ! -f $C_SO ]]; then
  echo "building C reference library..."
  (cd ../c_src && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null)
fi

# --- enumerate features declared in Cargo.toml ------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re,sys
txt=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', txt, re.M|re.S)
if not m:
    sys.exit(0)
for line in m.group(1).splitlines():
    line=line.split('#')[0].strip()
    if not line or '=' not in line: continue
    name=line.split('=')[0].strip()
    if name and name!='default':
        print(name)
PY
)

echo "features declared in Cargo.toml: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of feature flag sets to test.
COMBOS=("" "--no-default-features" "--all-features")
if ((${#FEATURES[@]})); then
  # every individual feature, and the full powerset if it is small enough
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  n=${#FEATURES[@]}
  if ((n <= 8)); then
    for ((mask=1; mask < (1<<n); mask++)); do
      sel=()
      for ((b=0; b<n; b++)); do (((mask>>b)&1)) && sel+=("${FEATURES[b]}"); done
      joined=$(IFS=,; echo "${sel[*]}")
      COMBOS+=("--no-default-features --features $joined")
    done
  fi
fi

# de-duplicate
mapfile -t COMBOS < <(printf '%s\n' "${COMBOS[@]}" | awk '!seen[$0]++')

fail=0
total=0

for profile in debug release; do
  PROF_FLAG=""; [[ $profile == release ]] && PROF_FLAG="--release"
  for combo in "${COMBOS[@]}"; do
    total=$((total+1))
    label="profile=$profile features='${combo:-<default>}'"
    echo
    echo "=============================================================="
    echo ">>> $label"
    echo "=============================================================="

    # shellcheck disable=SC2086
    if ! cargo build $PROF_FLAG $combo --offline >/dev/null 2>&1; then
      echo "BUILD FAILED: $label"; fail=$((fail+1)); continue
    fi

    RS_SO="target/$profile/libdriver.so"
    if [[ ! -f $RS_SO ]]; then
      echo "MISSING .so for $label"; fail=$((fail+1)); continue
    fi

    # --- symbol parity for this configuration -------------------------------
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBWR]$/ {print $3}' | sort -u) \
      <(nm -D --defined-only "$RS_SO" | awk '$2 ~ /^[TDBWR]$/ {print $3}' | sort -u))
    if [[ -n $missing ]]; then
      echo "SYMBOL PARITY FAILED ($label). Missing from Rust .so:"; echo "$missing"
      fail=$((fail+1)); continue
    fi
    echo "symbol parity: ok (0 missing)"

    # --- full differential suite -------------------------------------------
    # shellcheck disable=SC2086
    if cargo test $PROF_FLAG $combo --offline --test differential 2>&1 | tee /dev/stderr \
        | grep -q 'differential result: ok'; then
      echo "differential: ok ($label)"
    else
      echo "DIFFERENTIAL FAILED: $label"; fail=$((fail+1))
    fi
  done
done

echo
echo "=============================================================="
echo "configurations tested: $total ; failures: $fail"
echo "=============================================================="
[[ $fail -eq 0 ]] || exit 1
