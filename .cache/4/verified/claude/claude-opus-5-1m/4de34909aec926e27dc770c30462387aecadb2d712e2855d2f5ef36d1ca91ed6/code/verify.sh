#!/usr/bin/env bash
# Runs the whole differential verification for every valid build configuration.
#
#   ./verify.sh            # debug profile
#   PROFILES="debug release" ./verify.sh
#
# Phase A artefacts: SYMBOLS.md, ERRORS.md, CONFIGS.md
set -u -o pipefail

cd "$(dirname "$0")" || exit 1

CARGO_FLAGS=${CARGO_FLAGS:---offline}
PROFILES=${PROFILES:-debug}
rc=0

# --- enumerate every feature combination declared in Cargo.toml -------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]); print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  # No [features] section: the only configuration is the empty one. Both
  # spellings are exercised because they take different cargo code paths.
  COMBOS+=("" "--no-default-features")
else
  n=${#FEATURES[@]}
  total=$((1 << n))
  COMBOS+=("")
  for ((mask = 0; mask < total; mask++)); do
    list=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then
        list="${list:+$list,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("--no-default-features${list:+ --features $list}")
  done
fi

echo "=============================================================="
echo "features declared : ${FEATURES[*]:-<none>}"
echo "combinations      : ${#COMBOS[@]}"
echo "profiles          : $PROFILES"
echo "=============================================================="

for profile in $PROFILES; do
  prof_flag=""
  [ "$profile" = "release" ] && prof_flag="--release"

  for combo in "${COMBOS[@]}"; do
    label="profile=$profile features=[${combo:-default}]"

    echo
    echo "--- cargo check   $label"
    # shellcheck disable=SC2086
    if ! cargo check $CARGO_FLAGS $prof_flag $combo --all-targets 2>&1 | tail -n 5; then
      echo "FAILED: cargo check ($label)"
      rc=1
      continue
    fi

    echo "--- cargo build   $label   (produces the .so the tests dlopen)"
    # shellcheck disable=SC2086
    if ! cargo build $CARGO_FLAGS $prof_flag $combo 2>&1 | tail -n 5; then
      echo "FAILED: cargo build ($label)"
      rc=1
      continue
    fi

    echo "--- cargo test    $label"
    # shellcheck disable=SC2086
    if ! timeout 600 cargo test $CARGO_FLAGS $prof_flag $combo -- --test-threads=1 2>&1 \
      | grep -E '^(test result|running|error|warning: unused|test [a-z_0-9]+ \.\.\. FAILED)'; then
      echo "FAILED: cargo test ($label)"
      rc=1
      continue
    fi
  done
done

echo
echo "--- symbol parity (nm -D) ---"
c_so=target/c/libdriver_c.so
for profile in $PROFILES; do
  rust_so="target/$profile/libctype_driver.so"
  [ -f "$rust_so" ] || continue
  missing=$(comm -23 \
    <(nm -D --defined-only "$c_so" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    echo "MISSING from $rust_so:"
    echo "$missing"
    rc=1
  else
    echo "$rust_so: exports every C symbol ($(nm -D --defined-only "$c_so" | awk '{print $NF}' | sort -u | tr '\n' ' '))"
  fi
done

echo
if [ "$rc" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "FAILURES PRESENT (rc=$rc)"
fi
exit "$rc"
