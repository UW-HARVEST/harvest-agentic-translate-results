#!/usr/bin/env bash
# Verify the Rust translation against the C ground truth for every build
# configuration.
#
# `translation/Cargo.toml` declares no `[features]` table and `c_src/CMakeLists.txt`
# declares no options, `#define`s or conditional sources, so the complete set of
# valid configurations is:
#
#   1. default features            (`cargo test`)
#   2. no default features         (`cargo test --no-default-features`)
#   3. all features                (`cargo test --all-features`)
#
# With an empty feature set these are equivalent by construction, but they are
# all exercised so that adding a feature later cannot silently go untested.
# Each configuration is additionally run in both dev and release profiles, since
# `[profile.release]` sets `panic = "abort"`.
set -uo pipefail

cd "$(dirname "$0")/.." || exit 1
repo_root="$(cd .. && pwd)"

echo "== building the C ground truth =="
mkdir -p "$repo_root/c_src/build"
(
  cd "$repo_root/c_src/build" \
    && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && timeout 600 cmake --build . >/dev/null
) || { echo "FAIL: could not build c_src"; exit 1; }

# Guard against a stale Cargo.toml assumption: if features ever appear, this
# script must be extended to enumerate their combinations.
if grep -qE '^\s*\[features\]' Cargo.toml; then
  echo "NOTE: [features] is now present in Cargo.toml - enumerate combinations explicitly."
fi

feature_flags=(
  ""
  "--no-default-features"
  "--all-features"
)
profiles=("" "--release")

status=0
for flags in "${feature_flags[@]}"; do
  for profile in "${profiles[@]}"; do
    label="cargo test ${flags:-<default features>} ${profile:-<dev profile>}"
    echo
    echo "== $label =="
    # The cdylib must exist before the tests dlopen it; `cargo test` builds the
    # lib target first, but building explicitly keeps the failure mode obvious.
    if ! timeout 600 cargo build $flags $profile >/dev/null 2>&1; then
      echo "FAIL (build): $label"
      status=1
      continue
    fi
    if timeout 600 cargo test $flags $profile 2>&1 | tail -n 25; then
      echo "PASS: $label"
    else
      echo "FAIL: $label"
      status=1
    fi
  done
done

echo
if [ "$status" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASS"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$status"
