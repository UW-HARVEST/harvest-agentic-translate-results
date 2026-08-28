#!/usr/bin/env bash
# Enumerates every build-time configuration of the crate and runs
# `cargo check` + `cargo test` for each, in both dev and release profiles.
#
# The crate declares no [features] section, so the only valid configurations
# are the (empty) default feature set and --no-default-features. The script
# derives that from Cargo.toml rather than hard-coding it, so it keeps working
# if features are added later.
set -uo pipefail

cd "$(dirname "$0")" || exit 1

# --- enumerate features declared in Cargo.toml -----------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]);
                      if (a[1] != "default" && a[1] != "") print a[1] }
  ' Cargo.toml
)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  COMBOS+=("<default>" "<none>")
else
  # Power set of all declared features, plus the default feature set.
  n=${#FEATURES[@]}
  total=$((1 << n))
  COMBOS+=("<default>")
  for ((mask = 0; mask < total; mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then
        combo="${combo:+$combo,}${FEATURES[b]}"
      fi
    done
    if [ -z "$combo" ]; then COMBOS+=("<none>"); else COMBOS+=("$combo"); fi
  done
fi

echo "Declared features: ${FEATURES[*]:-(none)}"
echo "Configurations to verify: ${#COMBOS[@]}"
printf '  - %s\n' "${COMBOS[@]}"
echo

flags_for() {
  case "$1" in
    "<default>") echo "" ;;
    "<none>")    echo "--no-default-features" ;;
    *)           echo "--no-default-features --features $1" ;;
  esac
}

FAIL=0
for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2046
  read -r -a FL <<<"$(flags_for "$combo")"
  for profile in dev release; do
    PF=()
    [ "$profile" = release ] && PF=(--release)

    printf '=== %-12s profile=%-7s cargo check ... ' "$combo" "$profile"
    if timeout 600 cargo check "${FL[@]}" "${PF[@]}" --all-targets \
        >"/tmp/check_${combo//[^a-zA-Z0-9]/_}_$profile.log" 2>&1; then
      echo "OK"
    else
      echo "FAILED"; FAIL=1
      tail -25 "/tmp/check_${combo//[^a-zA-Z0-9]/_}_$profile.log"
      continue
    fi

    # The differential tests dlopen the cdylib, so make sure it exists for
    # this exact profile/feature set before the harness looks for it.
    timeout 600 cargo build --lib "${FL[@]}" "${PF[@]}" \
      >"/tmp/build_${combo//[^a-zA-Z0-9]/_}_$profile.log" 2>&1 || { echo "  build --lib FAILED"; FAIL=1; continue; }

    printf '=== %-12s profile=%-7s cargo test  ... ' "$combo" "$profile"
    LOG="/tmp/test_${combo//[^a-zA-Z0-9]/_}_$profile.log"
    if timeout 600 cargo test "${FL[@]}" "${PF[@]}" >"$LOG" 2>&1; then
      echo "OK ($(grep -c '\.\.\. ok' "$LOG") assertions/tests passed)"
    else
      echo "FAILED"; FAIL=1
      tail -40 "$LOG"
    fi
  done
done

echo
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAIL"
