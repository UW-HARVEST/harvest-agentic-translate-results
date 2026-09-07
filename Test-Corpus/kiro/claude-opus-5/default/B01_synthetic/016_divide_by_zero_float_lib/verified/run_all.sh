#!/usr/bin/env bash
# Runs the full differential suite across every Cargo feature combination and
# both profiles, rebuilding the C reference .so first.
#
# Usage:  ./run_all.sh
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
FAILED=0

echo "=== building the C reference shared library ==="
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null
) || { echo "C build FAILED"; exit 1; }
ls -l "$ROOT/c_src/build/libdriver.so"

cd "$CRATE_DIR"

# ---------------------------------------------------------------------------
# Enumerate feature combinations from Cargo.toml.
# ---------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /=/      { split($0, a, "="); gsub(/[ \t]/, "", a[1]); if (a[1] != "default") print a[1] }
  ' Cargo.toml
)

if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo
  echo "=== Cargo.toml declares no [features]; the only configurations are"
  echo "    default and --no-default-features ==="
  COMBOS=("" "--no-default-features" "--all-features")
else
  COMBOS=("" "--no-default-features" "--all-features")
  for f in "${FEATURES[@]}"; do
    COMBOS+=("--no-default-features --features $f")
  done
  # Pairwise combinations.
  n=${#FEATURES[@]}
  for ((i = 0; i < n; i++)); do
    for ((j = i + 1; j < n; j++)); do
      COMBOS+=("--no-default-features --features ${FEATURES[i]},${FEATURES[j]}")
    done
  done
fi

for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    label="cargo test ${profile:-<debug>} ${combo:-<default features>}"
    echo
    echo "################################################################"
    echo "# $label"
    echo "################################################################"
    # `cargo test` does not build cdylib artifacts, so build the .so under test
    # explicitly for THIS profile/feature combination first.
    # shellcheck disable=SC2086
    if ! timeout 600 cargo build $profile $combo 2>&1 | tail -n 10; then
      echo "RESULT: FAIL (build)  [$label]"; FAILED=1; continue
    fi
    so_profile=$([ -n "$profile" ] && echo release || echo debug)
    if [ ! -f "target/$so_profile/libdriver.so" ]; then
      echo "RESULT: FAIL (no target/$so_profile/libdriver.so)  [$label]"; FAILED=1; continue
    fi
    # shellcheck disable=SC2086
    if timeout 600 cargo test $profile $combo -- --test-threads=1 2>&1 | tail -n 45; then
      echo "RESULT: pass  [$label]"
    else
      echo "RESULT: FAIL  [$label]"
      FAILED=1
    fi
  done
done

echo
echo "=== symbol diff (C .so -> Rust .so), both profiles ==="
for profile in debug release; do
  rso="target/$profile/libdriver.so"
  [ -f "$rso" ] || { echo "$profile: no cdylib built, skipping"; continue; }
  diff_out=$(comm -23 \
    <(nm -D --defined-only --format=posix "$ROOT/c_src/build/libdriver.so" | awk '{print $1}' | sort) \
    <(nm -D --defined-only --format=posix "$rso" | awk '{print $1}' | sort))
  if [ -z "$diff_out" ]; then
    echo "$profile: symbol diff EMPTY (0 missing)"
  else
    echo "$profile: MISSING symbols:"; echo "$diff_out"; FAILED=1
  fi
done

echo
if [ "$FAILED" -eq 0 ]; then
  echo "ALL CONFIGURATIONS PASSED"
else
  echo "SOME CONFIGURATIONS FAILED"
fi
exit "$FAILED"
