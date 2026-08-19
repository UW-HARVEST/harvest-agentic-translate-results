#!/usr/bin/env bash
# Phase D — run the complete differential suite for every feature combination
# and both cargo profiles.
#
# `[profile.release] panic = "abort"` makes debug and release genuinely
# different builds, and each feature combination produces a different symbol
# set, so both axes are swept rather than assumed equivalent.
set -uo pipefail
cd "$(dirname "$0")"

mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z_][A-Za-z0-9_-]*[[:space:]]*=/ {
      split($0, a, "=");
      gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml | sort -u
)
n=${#FEATURES[@]}

fail=0
summary=()

for ((mask = 0; mask < (1 << n); mask++)); do
  combo=()
  for ((i = 0; i < n; i++)); do
    if (( (mask >> i) & 1 )); then combo+=("${FEATURES[$i]}"); fi
  done
  spec=$(IFS=,; echo "${combo[*]:-}")
  label=${spec:-<none>}

  for profile in dev release; do
    flags=(--no-default-features)
    [[ -n "$spec" ]] && flags+=(--features "$spec")
    [[ "$profile" == release ]] && flags+=(--release)

    echo "==============================================================="
    echo "features=$label profile=$profile"
    echo "==============================================================="
    # The C artifacts are cached per profile directory, so wipe them to be
    # sure each run rebuilds and re-verifies against a fresh C build.
    rm -rf "target/$([[ $profile == release ]] && echo release || echo debug)/ctest"

    out=$(timeout 600 cargo test "${flags[@]}" 2>&1)
    echo "$out" | grep -E '^(test |test result|running |     Running)' | tail -70

    if echo "$out" | grep -q 'test result: FAILED\|error\[E\|^error: '; then
      echo "$out" | grep -E 'FAILED|^error|panicked' | head -30
      summary+=("FAIL  features=$label profile=$profile")
      fail=1
    else
      counts=$(echo "$out" | grep -oE '^test result: ok\. [0-9]+ passed' |
               grep -oE '[0-9]+' | awk '{s += $1} END {print s + 0}')
      summary+=("ok    features=$label profile=$profile  (${counts:-0} tests passed)")
    fi
  done
done

echo
echo "=============================== SUMMARY ==============================="
for line in "${summary[@]}"; do echo "$line"; done
echo
if (( fail )); then
  echo "RESULT: FAILURES PRESENT"
  exit 1
fi
echo "RESULT: all feature combinations and profiles pass"
