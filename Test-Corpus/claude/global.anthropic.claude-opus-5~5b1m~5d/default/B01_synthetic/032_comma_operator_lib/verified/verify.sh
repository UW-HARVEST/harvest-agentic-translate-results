#!/usr/bin/env bash
# Full verification sweep: builds the C .so and the Rust .so (both profiles),
# then runs the differential suite under every Cargo feature combination.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
cargo_flags="--offline"
fail=0

echo "###############################################################"
echo "# 1. build the C shared library (ground truth)"
echo "###############################################################"
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
ls -l "$root/c_src/build/libdriver.so"

echo
echo "###############################################################"
echo "# 2. enumerate Cargo feature combinations"
echo "###############################################################"
# Extract feature names from the [features] table, if any.
features=$(python3 - "$here/Cargo.toml" <<'PY'
import sys, re
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\](.*?)(?=^\[|\Z)', text, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            name = line.split('=')[0].strip().strip('"')
            if name and name != 'default':
                names.append(name)
print(' '.join(names))
PY
)
if [ -z "$features" ]; then
  echo "Cargo.toml declares NO [features] -> exactly one configuration: default."
  combos=("default")
else
  echo "features found: $features"
  # Power set of the declared features, plus default and no-default-features.
  combos=("default" "none")
  # shellcheck disable=SC2206
  arr=($features)
  n=${#arr[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (( mask & (1 << b) )); then combo="$combo,${arr[b]}"; fi
    done
    combos+=("${combo#,}")
  done
fi
printf 'combinations to test: %s\n' "${combos[*]}"

echo
echo "###############################################################"
echo "# 3. cargo check / build / test per combination"
echo "###############################################################"
for combo in "${combos[@]}"; do
  case "$combo" in
    default) featflags=() ;;
    none)    featflags=(--no-default-features) ;;
    *)       featflags=(--no-default-features --features "$combo") ;;
  esac

  echo
  echo "--------------------------------------------------------------"
  echo "### combination: $combo   (${featflags[*]:-<default>})"
  echo "--------------------------------------------------------------"

  ( cd "$here" && timeout 300 cargo check $cargo_flags "${featflags[@]}" 2>&1 | tail -3 )

  # Build BOTH profiles so the harness compares release and debug .so files
  # (panic/overflow-check settings differ between them).
  ( cd "$here" && timeout 300 cargo build --release $cargo_flags "${featflags[@]}" >/dev/null 2>&1 ) \
    || { echo "release build FAILED for [$combo]"; fail=1; continue; }
  ( cd "$here" && timeout 300 cargo build $cargo_flags "${featflags[@]}" >/dev/null 2>&1 ) \
    || { echo "debug build FAILED for [$combo]"; fail=1; continue; }

  nm -D --defined-only "$here/target/release/libdriver.so" | awk '{print "  release exports: "$NF}'
  nm -D --defined-only "$here/target/debug/libdriver.so"   | awk '{print "  debug   exports: "$NF}'

  # Symbol parity check, independent of the test binary.
  missing=$(comm -23 \
    <(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$here/target/release/libdriver.so" | awk '{print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    echo "  SYMBOL PARITY FAILED for [$combo]; missing from Rust: $missing"
    fail=1
  else
    echo "  symbol parity: OK (0 missing)"
  fi

  out=$( cd "$here" && timeout 600 cargo test --release $cargo_flags "${featflags[@]}" 2>&1 )
  echo "$out" | grep -E "^test result:|^error" || true
  if echo "$out" | grep -q "test result: FAILED"; then
    echo "  TESTS FAILED for [$combo]"
    echo "$out" | grep -A3 "^failures:" | head -40
    fail=1
  else
    echo "  tests: OK for [$combo]"
  fi
done

echo
echo "###############################################################"
if [ "$fail" -eq 0 ]; then
  echo "# ALL COMBINATIONS PASSED"
else
  echo "# FAILURES PRESENT (see above)"
fi
echo "###############################################################"
exit "$fail"
