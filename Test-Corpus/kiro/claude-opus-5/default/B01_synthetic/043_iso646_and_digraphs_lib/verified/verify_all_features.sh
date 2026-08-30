#!/usr/bin/env bash
# Verify the translation against the C ground truth for every valid
# build-time feature combination.
#
# Feature combinations are read out of Cargo.toml rather than hard-coded, so the
# loop stays correct if features are added later. `driver` currently declares no
# [features] section at all, which leaves exactly one configuration: the empty
# set (`--no-default-features`, identical to the default build).
set -uo pipefail

cd "$(dirname "$0")" || exit 1
root="$(cd .. && pwd)"

# --- C ground truth ---------------------------------------------------------
(
  cd "$root/c_src" &&
  mkdir -p build && cd build &&
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null &&
  cmake --build . >/dev/null
) || { echo "FAIL: could not build the C shared library"; exit 1; }

# --- enumerate feature combinations ----------------------------------------
mapfile -t features < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /=/   {
      split($0, kv, "=")
      gsub(/[ \t"]/, "", kv[1])
      if (kv[1] != "" && kv[1] != "default") print kv[1]
    }
  ' Cargo.toml
)

combos=("")   # the empty combination, i.e. --no-default-features
n=${#features[@]}
if (( n > 0 && n <= 12 )); then
  for (( mask = 1; mask < (1 << n); mask++ )); do
    combo=""
    for (( i = 0; i < n; i++ )); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${features[i]}"
      fi
    done
    combos+=("$combo")
  done
elif (( n > 12 )); then
  echo "refusing to enumerate 2^$n combinations; narrow the list first" >&2
  exit 1
fi

echo "feature combinations to verify: ${#combos[@]}"
for combo in "${combos[@]}"; do
  echo "  - '${combo:-<none>}'"
done

status=0
for combo in "${combos[@]}"; do
  label="${combo:-<none>}"
  args=(--no-default-features)
  [[ -n "$combo" ]] && args+=(--features "$combo")

  echo
  echo "=== $label : cargo check ==="
  if ! timeout 600 cargo check "${args[@]}" 2>&1 | tail -n 5; then
    echo "FAIL: cargo check failed for '$label'"; status=1; continue
  fi

  echo "=== $label : cargo build --release ==="
  if ! timeout 600 cargo build --release "${args[@]}" 2>&1 | tail -n 5; then
    echo "FAIL: cargo build failed for '$label'"; status=1; continue
  fi

  echo "=== $label : nm -D symbol parity ==="
  c_syms=$(nm -D --defined-only "$root/c_src/build/libdriver.so" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u)
  rust_syms=$(nm -D --defined-only target/release/libdriver.so | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [[ -n "$missing" ]]; then
    echo "FAIL: Rust .so is missing C symbols for '$label':"; echo "$missing"; status=1
  else
    echo "ok: Rust .so exports every C symbol ($(echo "$c_syms" | wc -w) checked)"
  fi

  echo "=== $label : cargo test ==="
  if ! timeout 600 cargo test "${args[@]}" 2>&1 | grep -E '^(test |test result|error|running)' ; then
    echo "FAIL: cargo test failed for '$label'"; status=1; continue
  fi
done

echo
if (( status == 0 )); then
  echo "ALL FEATURE COMBINATIONS PASS"
else
  echo "FAILURES PRESENT"
fi
exit $status
