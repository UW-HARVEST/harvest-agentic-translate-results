#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every build-time
# configuration.
#
# Enumerates the powerset of the features declared in translation/Cargo.toml
# (excluding the "default" meta-feature) and runs `cargo check` and `cargo test`
# for each combination, in both the dev and the release profile.

set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"
log_dir="${TMPDIR:-/tmp}/hsv-verify"
mkdir -p "$log_dir"

timeout_secs=600
failures=0

# --- build the C reference -------------------------------------------------
echo "== building C reference =="
(
  cd "$root/c_src" &&
    mkdir -p build &&
    cd build &&
    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON &&
    cmake --build .
) >"$log_dir/c-build.log" 2>&1 || {
  echo "FAIL: C build (see $log_dir/c-build.log)"
  exit 1
}
echo "ok: $(ls "$root"/c_src/build/lib*.so)"

# --- enumerate feature combinations ---------------------------------------
# Feature names are the keys of the [features] table, minus "default".
mapfile -t features < <(
  awk '
    /^\[features\]/ { in_features = 1; next }
    /^\[/           { in_features = 0 }
    in_features && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, parts, "=")
      gsub(/[[:space:]]/, "", parts[1])
      if (parts[1] != "default") print parts[1]
    }
  ' "$root/translation/Cargo.toml"
)

n=${#features[@]}
combos=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=()
  for ((bit = 0; bit < n; bit++)); do
    if ((mask & (1 << bit))); then combo+=("${features[bit]}"); fi
  done
  combos+=("$(
    IFS=,
    echo "${combo[*]}"
  )")
done

echo "== ${#combos[@]} feature combination(s) from ${n} declared feature(s) =="
for combo in "${combos[@]}"; do
  echo "  - [${combo:-<none>}]"
done

# --- check and test each combination in each profile ----------------------
run() {
  local label="$1" logfile="$2"
  shift 2
  if timeout "$timeout_secs" "$@" >"$logfile" 2>&1; then
    echo "  PASS  $label"
  else
    echo "  FAIL  $label  (log: $logfile)"
    tail -n 25 "$logfile" | sed 's/^/        /'
    failures=$((failures + 1))
  fi
}

cd "$root/translation" || exit 1

for combo in "${combos[@]}"; do
  slug="${combo:-none}"
  slug="${slug//,/_}"
  feature_args=(--no-default-features)
  if [[ -n "$combo" ]]; then feature_args+=(--features "$combo"); fi

  echo "== features: [${combo:-<none>}] =="
  run "cargo check" "$log_dir/check-$slug.log" \
    cargo check --all-targets "${feature_args[@]}"

  for profile in dev release; do
    profile_args=()
    if [[ "$profile" == release ]]; then profile_args+=(--release); fi
    run "cargo test ($profile)" "$log_dir/test-$slug-$profile.log" \
      cargo test "${profile_args[@]}" "${feature_args[@]}"
  done
done

echo
if ((failures == 0)); then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "$failures step(s) FAILED"
fi
exit $((failures > 0))
