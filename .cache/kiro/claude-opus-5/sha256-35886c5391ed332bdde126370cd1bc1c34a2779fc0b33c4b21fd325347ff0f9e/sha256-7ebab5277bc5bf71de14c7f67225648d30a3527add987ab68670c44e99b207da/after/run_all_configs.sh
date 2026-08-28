#!/usr/bin/env bash
# Enumerate every feature combination declared in translation/Cargo.toml and run
# `cargo check` + `cargo test` for each. Also exercises the release profile,
# which differs from dev (`panic = "abort"`).
set -uo pipefail

cd "$(dirname "$0")/translation" || exit 1

# Feature names from the [features] table, excluding "default".
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f=1; next }
    /^\[/           { in_f=0 }
    in_f && /=/     { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  echo "No [features] declared -> single configuration."
  COMBOS+=("")
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

fail=0
for combo in "${COMBOS[@]}"; do
  for profile in dev release; do
    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")
    [ "$profile" = release ] && args+=(--release)
    label="features=[${combo:-<none>}] profile=$profile"

    echo "=== cargo check $label ==="
    if ! timeout 600 cargo check --all-targets "${args[@]}" > /tmp/check.log 2>&1; then
      echo "CHECK FAILED: $label"; tail -30 /tmp/check.log; fail=1; continue
    fi

    echo "=== cargo test  $label ==="
    if ! timeout 600 cargo test "${args[@]}" > /tmp/test.log 2>&1; then
      echo "TEST FAILED: $label"; tail -40 /tmp/test.log; fail=1; continue
    fi
    grep -E "^test result:" /tmp/test.log | sed 's/^/    /'
  done
done

# Also verify the default configuration (default features on).
for profile in dev release; do
  args=()
  [ "$profile" = release ] && args+=(--release)
  echo "=== cargo test default-features profile=$profile ==="
  if ! timeout 600 cargo test "${args[@]}" > /tmp/test.log 2>&1; then
    echo "TEST FAILED: default profile=$profile"; tail -40 /tmp/test.log; fail=1; continue
  fi
  grep -E "^test result:" /tmp/test.log | sed 's/^/    /'
done

if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$fail"
