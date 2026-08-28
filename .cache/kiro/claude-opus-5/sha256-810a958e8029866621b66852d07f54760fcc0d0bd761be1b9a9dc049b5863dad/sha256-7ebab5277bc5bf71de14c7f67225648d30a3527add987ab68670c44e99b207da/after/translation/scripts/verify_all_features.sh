#!/usr/bin/env bash
# Enumerates every valid feature combination of the crate and, for each one,
# runs `cargo check --all-targets` and the full differential test suite against
# the freshly built C shared library.
#
#   ./scripts/verify_all_features.sh            # check + test every combination
#   ./scripts/verify_all_features.sh --check    # check only
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

CHECK_ONLY=0
[ "${1:-}" = "--check" ] && CHECK_ONLY=1

# --- enumerate the feature powerset ----------------------------------------
mapfile -t FEATURES < <(python3 - Cargo.toml <<'PY'
import re, sys
text = open(sys.argv[1]).read()
m = re.search(r'(?ms)^\[features\]\s*$(.*?)(?=^\[|\Z)', text)
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if '=' in line:
            print(line.split('=', 1)[0].strip().strip('"'))
PY
)

COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the only configuration is the default."
  COMBOS=("")
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

echo "features declared: ${FEATURES[*]:-<none>}"
echo "combinations to verify: ${#COMBOS[@]}"
echo

# --- build the C reference library once ------------------------------------
echo "=== building C reference library ==="
(
  mkdir -p ../c_src/build &&
    cd ../c_src/build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
    cmake --build .
) >/tmp/verify-c-build.log 2>&1 || {
  echo "C build FAILED (see /tmp/verify-c-build.log)"
  tail -20 /tmp/verify-c-build.log
  exit 1
}
ls -l ../c_src/build/libdriver.so
echo

rc=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default/no features>}"
  echo "################################################################"
  echo "# combination: $label"
  echo "################################################################"

  args=(--no-default-features)
  [ -n "$combo" ] && args+=(--features "$combo")

  slug=$(echo "${combo:-default}" | tr ',' '_')

  echo "--- cargo check --all-targets ---"
  if timeout 600 cargo check "${args[@]}" --all-targets >"/tmp/verify-check-$slug.log" 2>&1; then
    echo "check OK"
  else
    echo "check FAILED (/tmp/verify-check-$slug.log)"
    tail -40 "/tmp/verify-check-$slug.log"
    rc=1
    continue
  fi

  [ "$CHECK_ONLY" = 1 ] && { echo; continue; }

  # The tests rebuild the cdylib themselves; tell them which flags to use.
  export DIFFTEST_NO_DEFAULT_FEATURES=1
  export DIFFTEST_FEATURES="$combo"
  rm -f target/release/libdriver.so

  echo "--- cargo build --release (cdylib under test) ---"
  if ! timeout 600 cargo build "${args[@]}" --release --lib >"/tmp/verify-build-$slug.log" 2>&1; then
    echo "release build FAILED (/tmp/verify-build-$slug.log)"
    tail -40 "/tmp/verify-build-$slug.log"
    rc=1
    continue
  fi

  echo "--- nm -D symbol parity ---"
  nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $3}' | sort >/tmp/verify-c-syms.txt
  nm -D --defined-only target/release/libdriver.so | awk '{print $3}' | sort >/tmp/verify-r-syms.txt
  if missing=$(comm -23 /tmp/verify-c-syms.txt /tmp/verify-r-syms.txt) && [ -z "$missing" ]; then
    echo "all $(wc -l </tmp/verify-c-syms.txt) C symbols present in the Rust .so"
  else
    echo "MISSING from the Rust .so:"
    echo "$missing"
    rc=1
  fi

  echo "--- cargo test ---"
  if timeout 600 cargo test "${args[@]}" >"/tmp/verify-test-$slug.log" 2>&1; then
    grep -E "^test result:" "/tmp/verify-test-$slug.log"
    echo "tests OK"
  else
    echo "tests FAILED (/tmp/verify-test-$slug.log)"
    grep -nE "^(test result:|---- |thread |assertion)" "/tmp/verify-test-$slug.log" | head -60
    rc=1
  fi
  echo
done

if [ "$rc" = 0 ]; then
  echo "ALL COMBINATIONS VERIFIED"
else
  echo "SOME COMBINATIONS FAILED"
fi
exit "$rc"
