#!/usr/bin/env bash
# Phase D driver: rebuild both shared objects, diff their exported symbols, and
# run the full differential suite under every feature combination and every
# build profile. Nothing here is manual per-configuration repetition.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
C_SRC="$ROOT/../c_src"
C_SO="$C_SRC/build/libdriver.so"
rc=0

echo "=== 1. build the C reference .so ==="
( cd "$C_SRC" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >"$ROOT/c_build.log" 2>&1 \
  || { echo "C build FAILED (see c_build.log)"; exit 1; }
echo "ok: $C_SO"

echo
echo "=== 2. enumerate feature combinations from Cargo.toml ==="
# Everything under [features], excluding the `default` key itself.
FEATURES=$(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/           {inside=0}
  inside && /=/   {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}
' "$ROOT/Cargo.toml")

# Combination list is always: default build + --no-default-features, plus the
# powerset of any declared features. With no [features] section the first two
# are the only configurations that exist.
COMBOS=("default" "no-default-features")
if [ -n "$FEATURES" ]; then
  feats=($FEATURES)
  n=${#feats[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${feats[$i]}"; fi
    done
    COMBOS+=("no-default-features --features $combo")
    COMBOS+=("all-plus:$combo")
  done
  COMBOS+=("all-features")
fi
printf 'declared features: %s\n' "${FEATURES:-(none)}"
printf 'configurations to verify: %d\n' "${#COMBOS[@]}"
printf '  - %s\n' "${COMBOS[@]}"

echo
echo "=== 3. verify every (configuration x profile) ==="
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default)             flags=() ;;
    no-default-features) flags=(--no-default-features) ;;
    all-features)        flags=(--all-features) ;;
    all-plus:*)          flags=(--features "${combo#all-plus:}") ;;
    "no-default-features --features "*)
                         flags=(--no-default-features --features "${combo##*--features }") ;;
    *)                   flags=() ;;
  esac

  for profile in debug release; do
    if [ "$profile" = release ]; then pflags=(--release); else pflags=(); fi
    label="[$combo | $profile]"

    if ! (cd "$ROOT" && timeout 600 cargo build "${pflags[@]}" "${flags[@]}") \
         >"$ROOT/build-$profile.log" 2>&1; then
      echo "$label cargo build FAILED (build-$profile.log)"; rc=1; continue
    fi
    RUST_SO="$ROOT/target/$profile/libdriver.so"
    if [ ! -f "$RUST_SO" ]; then
      echo "$label expected $RUST_SO to exist"; rc=1; continue
    fi

    # 3a. symbol diff must be empty
    nm -D --defined-only --format=posix "$C_SO"   | awk '{print $1}' \
      | grep -vE '^(_ITM_|__gmon|_init|_fini|__bss|_edata|_end)' | sort -u >"$ROOT/.c.syms"
    nm -D --defined-only --format=posix "$RUST_SO" | awk '{print $1}' \
      | grep -vE '^(_ITM_|__gmon|_init|_fini|__bss|_edata|_end)' | sort -u >"$ROOT/.r.syms"
    missing=$(comm -23 "$ROOT/.c.syms" "$ROOT/.r.syms")
    if [ -n "$missing" ]; then
      echo "$label SYMBOL DIFF NOT EMPTY - missing from Rust .so:"; echo "$missing" | sed 's/^/    /'
      rc=1
    else
      echo "$label symbol diff empty ($(wc -l <"$ROOT/.c.syms") C symbols all present)"
    fi

    # 3b. full differential suite (Phase B + Phase C + Phase D)
    if (cd "$ROOT" && C_SO="$C_SO" RUST_SO="$RUST_SO" \
        timeout 600 cargo test "${pflags[@]}" "${flags[@]}" -- --test-threads=1) \
        >"$ROOT/test-$profile.log" 2>&1; then
      echo "$label tests PASSED ($(grep -c '\.\.\. ok$' "$ROOT/test-$profile.log") test cases)"
    else
      echo "$label tests FAILED:"; grep -E 'FAILED|panicked' "$ROOT/test-$profile.log" | head -20 | sed 's/^/    /'
      rc=1
    fi
  done
done

rm -f "$ROOT/.c.syms" "$ROOT/.r.syms"
echo
if [ "$rc" -eq 0 ]; then echo "=== ALL CONFIGURATIONS PASSED ==="; else echo "=== FAILURES PRESENT ==="; fi
exit "$rc"
