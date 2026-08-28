#!/usr/bin/env bash
# Enumerates every valid cargo feature combination for the translation crate and
# runs `cargo check` (and optionally `cargo test`) for each one.
#
#   ./verify_all_features.sh check   # cargo check per combination (default)
#   ./verify_all_features.sh test    # cargo check + cargo test per combination
set -uo pipefail

cd "$(dirname "$0")/translation" || exit 1

MODE="${1:-check}"

# --- Enumerate features declared in Cargo.toml -------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { in_f = 1; next }
    /^\[/           { in_f = 0 }
    in_f && /=/     {
      split($0, parts, "=")
      gsub(/[ \t"]/, "", parts[1])
      if (parts[1] != "" && parts[1] !~ /^#/ && parts[1] != "default")
        print parts[1]
    }
  ' Cargo.toml
)

HAS_DEFAULT=$(awk '
  /^\[features\]/ { in_f = 1; next }
  /^\[/           { in_f = 0 }
  in_f && /^[ \t]*default[ \t]*=/ { print "yes"; exit }
' Cargo.toml)

N=${#FEATURES[@]}
echo "Declared non-default features: $N ${FEATURES[*]:-(none)}"
echo "Has explicit [features].default: ${HAS_DEFAULT:-no}"

# --- Build the combination list ---------------------------------------------
COMBOS=()
if [ "$N" -eq 0 ]; then
  # No features declared: the only build-time configuration is the default one.
  COMBOS+=("")
else
  for ((mask = 0; mask < (1 << N); mask++)); do
    combo=""
    for ((i = 0; i < N; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Combinations to verify: ${#COMBOS[@]}"
echo

FAILED=0
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"
  log="/tmp/verify_$(echo "${combo:-none}" | tr ',' '_').log"

  printf '=== %-40s ' "$label"
  if timeout 600 cargo check --no-default-features --features "$combo" --all-targets \
      > "$log" 2>&1; then
    printf 'check OK'
  else
    printf 'check FAILED (%s)' "$log"
    FAILED=1
    echo
    continue
  fi

  if [ "$MODE" = "test" ]; then
    if timeout 600 cargo test --no-default-features --features "$combo" \
        >> "$log" 2>&1; then
      printf '  |  test OK'
    else
      printf '  |  test FAILED (%s)' "$log"
      FAILED=1
    fi
  fi
  echo
done

# --- Also verify the default configuration explicitly -----------------------
printf '=== %-40s ' "default features"
if timeout 600 cargo check --all-targets > /tmp/verify_default.log 2>&1; then
  printf 'check OK'
else
  printf 'check FAILED (/tmp/verify_default.log)'
  FAILED=1
fi
if [ "$MODE" = "test" ]; then
  if timeout 600 cargo test >> /tmp/verify_default.log 2>&1; then
    printf '  |  test OK'
  else
    printf '  |  test FAILED (/tmp/verify_default.log)'
    FAILED=1
  fi
fi
echo

echo
if [ "$FAILED" -eq 0 ]; then
  echo "ALL FEATURE COMBINATIONS PASSED"
else
  echo "SOME FEATURE COMBINATIONS FAILED"
fi
exit "$FAILED"
