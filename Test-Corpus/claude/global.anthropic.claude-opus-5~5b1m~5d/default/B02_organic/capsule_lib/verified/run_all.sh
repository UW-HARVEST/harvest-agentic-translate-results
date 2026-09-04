#!/usr/bin/env bash
# Phase D driver: symbol parity + the whole differential suite under every
# cargo feature combination and both cargo profiles.
set -uo pipefail
cd "$(dirname "$0")"

ROOT="$(cd .. && pwd)"
C_SO="$(ls "$ROOT"/c_src/build/*.so 2>/dev/null | head -1)"
if [[ -z "$C_SO" ]]; then
  echo "!! C .so missing; build it with:"
  echo "   cd $ROOT/c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi

fail=0

# ---------------------------------------------------------------- features ---
# Enumerate the [features] table (excluding the implicit "default").
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:］ ]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)

COMBOS=()
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  # No [features] table: the default build is the only configuration, but we
  # still verify --no-default-features explicitly.
  COMBOS+=("<default>" "<none>")
else
  n=${#FEATURES[@]}
  COMBOS+=("<default>" "<none>")
  for ((m = 1; m < (1 << n); m++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( m & (1 << i) )); then combo+="${FEATURES[i]},"; fi
    done
    COMBOS+=("${combo%,}")
  done
fi

echo "### feature combinations to verify: ${COMBOS[*]}"

for profile in "" "--release"; do
  for combo in "${COMBOS[@]}"; do
    case "$combo" in
      "<default>") args=() ;;
      "<none>")    args=(--no-default-features) ;;
      *)           args=(--no-default-features --features "$combo") ;;
    esac
    label="profile=${profile:-dev} features=$combo"

    # `cargo test` alone does not emit the cdylib for the dev profile, so build
    # it explicitly: the tests must load the .so of the profile under test.
    echo "=== cargo build $profile ${args[*]:-} ($label) ==="
    if ! timeout 600 cargo build $profile "${args[@]}" 2>&1 | tail -n 10; then
      echo "!! BUILD FAILED: $label"
      fail=1
    fi

    echo "=== cargo test $profile ${args[*]:-} ($label) ==="
    if ! timeout 600 cargo test $profile "${args[@]}" 2>&1 | tail -n 40; then
      echo "!! FAILED: $label"
      fail=1
    fi

    # ---------------------------------------------------------- symbols ---
    if [[ "$profile" == "--release" ]]; then rdir=target/release; else rdir=target/debug; fi
    R_SO="$rdir/libcapsule_lib.so"
    missing=$(comm -23 \
      <(nm -D --defined-only "$C_SO" | awk '$2=="T"{print $3}' | sort) \
      <(nm -D --defined-only "$R_SO" | awk '$2=="T"{print $3}' | sort))
    if [[ -n "$missing" ]]; then
      echo "!! symbols missing from $R_SO ($label):"
      echo "$missing"
      fail=1
    else
      echo "--- symbol parity OK ($label): $(nm -D --defined-only "$C_SO" | awk '$2=="T"' | wc -l) symbols"
    fi
  done
done

if [[ $fail -eq 0 ]]; then
  echo "ALL GREEN"
else
  echo "FAILURES PRESENT"
fi
exit $fail
