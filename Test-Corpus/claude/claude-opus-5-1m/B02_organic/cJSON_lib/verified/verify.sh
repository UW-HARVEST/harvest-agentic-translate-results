#!/usr/bin/env bash
# Full verification driver: builds the C reference .so, enumerates every valid
# Cargo feature combination, and runs `cargo check` + the whole differential
# test suite for each of them.
#
#   ./verify.sh            # everything
#   ./verify.sh --quick    # skip the ~10 s, 4 GiB rows 23/25 test
set -uo pipefail
cd "$(dirname "$0")"

QUICK=0
[ "${1:-}" = "--quick" ] && QUICK=1

CARGO_FLAGS="--offline"
FAILED=0

echo "=== 1/4  building the C reference shared objects ==============="
(
  cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
ls -l c_src/build/libcjson.so.1.7.19 c_src/build/libcJSON_test.so

echo
echo "=== 2/4  enumerating feature combinations ====================="
mapfile -t COMBOS < <(python3 - <<'PY'
import itertools, re, sys
text = open("Cargo.toml").read()
m = re.search(r'(?ms)^\[features\]\s*(.*?)(?=^\[|\Z)', text)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip()
        if name and name != "default":
            feats.append(name)
# every subset of the optional features (the empty set is --no-default-features)
for n in range(len(feats) + 1):
    for combo in itertools.combinations(feats, n):
        print(",".join(combo))
PY
)
printf 'found %d combination(s):\n' "${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then echo "  - <none> (--no-default-features)"; else echo "  - $c"; fi
done

echo
echo "=== 3/4  cargo check for every combination ===================="
for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then FEAT=(--no-default-features); else FEAT=(--no-default-features --features "$c"); fi
  echo "--- cargo check ${FEAT[*]}"
  cargo check $CARGO_FLAGS "${FEAT[@]}" --all-targets 2>&1 | tail -3
  [ "${PIPESTATUS[0]}" -ne 0 ] && FAILED=1
done

echo
echo "=== 4/4  differential test suite for every combination ========"
EXCLUDE=()
[ "$QUICK" = 1 ] && EXCLUDE=(--test phase_b_core --test phase_b_hooks --test phase_b_print \
  --test phase_b_parse --test phase_b_api --test phase_b_pipeline --test phase_b_driver \
  --test phase_b_locale --test phase_c_errors --test phase_c_errors2 --test phase_c_alloc \
  --test phase_d_symbols)

for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then FEAT=(--no-default-features); else FEAT=(--no-default-features --features "$c"); fi
  echo "--- building the Rust .so ${FEAT[*]}"
  cargo build $CARGO_FLAGS --release "${FEAT[@]}" 2>&1 | tail -2
  [ "${PIPESTATUS[0]}" -ne 0 ] && { FAILED=1; continue; }
  echo "--- cargo test --release ${FEAT[*]}"
  timeout 600 cargo test $CARGO_FLAGS --release "${FEAT[@]}" "${EXCLUDE[@]}" 2>&1 \
    | grep -E "^(test |running|test result|error|warning: unused)" | grep -v "^test .* ok$"
  [ "${PIPESTATUS[0]}" -ne 0 ] && FAILED=1
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "ALL GREEN"
else
  echo "FAILURES DETECTED"
fi
exit "$FAILED"
