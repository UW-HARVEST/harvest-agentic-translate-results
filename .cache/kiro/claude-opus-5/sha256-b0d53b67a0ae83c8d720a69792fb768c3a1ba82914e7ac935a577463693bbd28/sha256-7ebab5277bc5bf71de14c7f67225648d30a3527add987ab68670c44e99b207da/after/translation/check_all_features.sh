#!/usr/bin/env bash
# Enumerate every feature combination declared in Cargo.toml and run
# cargo check + cargo test for each one.
set -uo pipefail
cd "$(dirname "$0")"

# Extract feature names from the [features] table (ignoring the implicit
# "default" key, which cargo handles via --no-default-features/--features).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

n=${#FEATURES[@]}
echo "declared features (${n}): ${FEATURES[*]:-<none>}"

combos=()
if [ "$n" -eq 0 ]; then
  combos+=("")           # only the (empty) default configuration exists
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[i]}"
      fi
    done
    combos+=("$combo")
  done
fi
# Always cover the crate's own default feature set as well.
combos+=("__DEFAULT__")

rc=0
for combo in "${combos[@]}"; do
  if [ "$combo" = "__DEFAULT__" ]; then
    args=()
    label="<default features>"
  else
    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")
    label="--no-default-features${combo:+ --features $combo}"
  fi

  echo "=============================================================="
  echo "### $label"
  for prof in "" "--release"; do
    if ! timeout 600 cargo check ${prof:+$prof} --all-targets "${args[@]}" \
        >/tmp/fc-check.log 2>&1; then
      echo "CHECK FAILED ($label ${prof:-debug})"; tail -30 /tmp/fc-check.log; rc=1; continue
    fi
    # `cargo test` will not rebuild the cdylib (no test target links it), so it
    # must be built explicitly for this profile/feature set first.
    if ! timeout 600 cargo build ${prof:+$prof} "${args[@]}" \
        >/tmp/fc-build.log 2>&1; then
      echo "BUILD FAILED ($label ${prof:-debug})"; tail -30 /tmp/fc-build.log; rc=1; continue
    fi
    if ! timeout 600 cargo test ${prof:+$prof} "${args[@]}" \
        >/tmp/fc-test.log 2>&1; then
      echo "TEST FAILED ($label ${prof:-debug})"; tail -40 /tmp/fc-test.log; rc=1; continue
    fi
    echo "  ok: ${prof:-debug}  $(grep -c '^test .* ok$' /tmp/fc-test.log) tests passed"
  done
done

echo "=============================================================="
exit $rc
