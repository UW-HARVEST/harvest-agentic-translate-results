#!/usr/bin/env bash
# Build the C .so and the Rust .so, then run the full differential test suite
# under every feature combination declared in Cargo.toml.
#
# Usage: translation/scripts/check_features.sh
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname "$here")"
root="$(dirname "$crate")"
offline="${CARGO_OFFLINE:---offline}"

fail=0

echo "== building the C shared library =="
mkdir -p "$root/c_src/build"
( cd "$root/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
ls -l "$root/c_src/build/libdriver.so"

# Extract the feature names from the [features] table, if any.
mapfile -t features < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { sub(/ *=.*/, ""); gsub(/[ \t"]/, ""); if ($0 != "" && $0 !~ /^#/) print }
  ' "$crate/Cargo.toml"
)

# Feature combinations to verify: default, no-default-features, then every
# declared feature on its own and (if any) all of them together.
combos=("default:")
combos+=("no-default:--no-default-features")
if ((${#features[@]})); then
  for f in "${features[@]}"; do
    combos+=("$f:--no-default-features --features $f")
  done
  all="$(IFS=,; echo "${features[*]}")"
  combos+=("all:--no-default-features --features $all")
  combos+=("default+all:--features $all")
else
  echo "note: Cargo.toml declares no [features]; default and --no-default-features are the only configurations"
fi

for combo in "${combos[@]}"; do
  name="${combo%%:*}"
  flags="${combo#*:}"
  echo
  echo "=================================================================="
  echo "== feature combination: $name   (cargo flags: ${flags:-<none>})"
  echo "=================================================================="
  # shellcheck disable=SC2086
  ( cd "$crate" && cargo build --release $offline $flags ) || { echo "[$name] BUILD FAILED"; fail=1; continue; }

  echo "-- exported symbols --"
  cdefs=$(nm -D --defined-only --format=posix "$root/c_src/build/libdriver.so" | awk '{print $1}' | sort -u)
  rdefs=$(nm -D --defined-only --format=posix "$crate/target/release/libdriver.so" | awk '{print $1}' | sort -u)
  missing=$(comm -23 <(echo "$cdefs") <(echo "$rdefs"))
  if [[ -n "$missing" ]]; then
    echo "[$name] MISSING SYMBOLS in the Rust .so:"; echo "$missing"; fail=1
  else
    echo "[$name] symbol diff empty ($(echo "$cdefs" | wc -l) C exports all present)"
  fi

  # shellcheck disable=SC2086
  ( cd "$crate" && cargo test --release $offline $flags ) || { echo "[$name] TESTS FAILED"; fail=1; }
done

echo
if ((fail)); then
  echo "RESULT: FAILURES (see above)"
else
  echo "RESULT: all feature combinations passed"
fi
exit "$fail"
