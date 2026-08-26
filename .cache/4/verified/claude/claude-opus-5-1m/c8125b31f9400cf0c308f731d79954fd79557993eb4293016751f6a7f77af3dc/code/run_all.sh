#!/usr/bin/env bash
# Build both shared libraries, then run the full differential suite for EVERY
# feature combination declared in Cargo.toml.
#
# `cargo build` is mandatory before `cargo test`: cargo does not emit a
# cdylib-only lib target during `cargo test`, so testing without building first
# would silently exercise a stale .so (the test harness now hard-fails on that).
set -euo pipefail
cd "$(dirname "$0")"

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations from Cargo.toml (power set of [features],
#    excluding the "default" meta-feature).
# ---------------------------------------------------------------------------
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

COMBOS=()
n=${#FEATURES[@]}
if [ "$n" -eq 0 ]; then
  COMBOS=("")            # no features exist -> exactly one configuration
else
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "Discovered ${#FEATURES[@]} feature(s): ${FEATURES[*]:-<none>}"
echo "Feature combinations to verify: ${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# 2. Build the C reference library.
# ---------------------------------------------------------------------------
echo
echo "=== Building C reference .so ==="
mkdir -p c_src/build
(cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > cmake.log 2>&1 \
  && cmake --build . >> cmake.log 2>&1) \
  || { echo "C build FAILED"; tail -30 c_src/build/cmake.log; exit 1; }
ls -l c_src/build/*.so

# ---------------------------------------------------------------------------
# 3. For each combination: check, build the .so, run every test binary.
# ---------------------------------------------------------------------------
rc=0
for combo in "${COMBOS[@]}"; do
 for profile in dev release; do
  if [ -z "$combo" ]; then
    label="<no features>"
    fargs=(--no-default-features)
  else
    label="$combo"
    fargs=(--no-default-features --features "$combo")
  fi
  # The release profile sets panic = "abort" and enables optimisations, which can
  # change float->int codegen, so it is verified as well as the dev profile.
  if [ "$profile" = release ]; then fargs+=(--release); fi
  label="$label / $profile"

  echo
  echo "############################################################"
  echo "# feature combination: $label"
  echo "############################################################"

  echo "--- cargo check ---"
  if ! timeout 600 cargo check "${fargs[@]}" 2>&1 | tail -5; then
    echo "CHECK FAILED for $label"; rc=1; continue
  fi

  echo "--- cargo build (emits the cdylib the tests dlopen) ---"
  if ! timeout 600 cargo build "${fargs[@]}" 2>&1 | tail -5; then
    echo "BUILD FAILED for $label"; rc=1; continue
  fi

  echo "--- cargo test ---"
  log="$(mktemp "${TMPDIR:-/tmp}/dnf-test-XXXXXX.log")"
  if timeout 600 cargo test "${fargs[@]}" > "$log" 2>&1; then
    # Show every test binary and its verdict (never truncate the summary).
    grep -E "^\s*Running |^test result:" "$log" | sed 's/^/    /'
    bins=$(grep -c "^test result:" "$log")
    tests=$(awk '/^test result: ok\./ { s += $4 } END { print s+0 }' "$log")
    echo "    => $bins test binaries, ${tests:-0} test functions, all passed"
  else
    echo "TESTS FAILED for $label"
    grep -E "^\s*Running |^test result:|^failures:|panicked|^error" "$log" | sed 's/^/    /'
    sed -n '/^failures:/,$p' "$log" | head -60
    rc=1
  fi
  rm -f "$log"
 done
done

echo
echo "=== Phase A bookkeeping (rows <-> tests <-> symbols) ==="
if ! python3 check_coverage.py; then rc=1; fi

echo
if [ "$rc" -eq 0 ]; then
  echo "=== ALL feature combinations passed ==="
else
  echo "=== FAILURES present ==="
fi
exit "$rc"
