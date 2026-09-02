#!/bin/bash
# Phase D: enumerate every cargo feature combination declared in Cargo.toml and
# run the full differential suite under each one. Features are extracted from
# Cargo.toml rather than hard-coded, so new features are picked up automatically.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT" || exit 1

# --- extract feature names from the [features] table -------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { infeat=1; next }
    /^\[/           { infeat=0 }
    infeat && /=/   { split($0, a, "="); gsub(/[ \t"]/, "", a[1]);
                      if (a[1] != "" && a[1] !~ /^#/ && a[1] != "default") print a[1] }
  ' Cargo.toml | sort -u
)

# default features (whatever `default = [...]` expands to) is always combo #1
COMBOS=("<default>")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  COMBOS+=("<none>")
  # full power set of the non-default features
  total=$((1 << n))
  for ((mask = 1; mask < total; mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then
        combo="${combo:+$combo,}${FEATURES[$i]}"
      fi
    done
    COMBOS+=("$combo")
  done
fi

echo "declared features : ${FEATURES[*]:-<none declared>}"
echo "combinations      : ${#COMBOS[@]}"
echo

rc_all=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    "<default>") args=() ;;
    "<none>")    args=(--no-default-features) ;;
    *)           args=(--no-default-features --features "$combo") ;;
  esac

  # Build the cdylib under this combination and test against THAT .so, so the
  # feature-gated code paths are the ones actually exercised through the FFI.
  if ! timeout 600 cargo build --release "${args[@]}" >/tmp/pd_build.log 2>&1; then
    echo "FAIL  build   [$combo]"; tail -5 /tmp/pd_build.log; rc_all=1; continue
  fi
  if ! timeout 600 cargo check "${args[@]}" >/tmp/pd_check.log 2>&1; then
    echo "FAIL  check   [$combo]"; tail -5 /tmp/pd_check.log; rc_all=1; continue
  fi
  out=$(RUST_DRIVER_SO="$ROOT/target/release/libdriver.so" \
        timeout 600 cargo test --release "${args[@]}" 2>&1)
  if [ $? -eq 0 ]; then
    echo "PASS  [$combo]  $(echo "$out" | grep -E '^test result:' | tail -1)"
  else
    echo "FAIL  test    [$combo]"; echo "$out" | tail -25; rc_all=1
  fi

  # Symbol parity must hold under every combination too.
  missing=$(comm -23 \
    <(nm -D --defined-only ../c_src/build/libdriver.so | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only target/release/libdriver.so | awk '{print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    echo "FAIL  symbols [$combo] missing: $missing"; rc_all=1
  else
    echo "      symbols [$combo] parity OK (0 missing)"
  fi
done

echo
if [ "$rc_all" -eq 0 ]; then echo "ALL COMBINATIONS PASS"; else echo "SOME COMBINATIONS FAILED"; fi
exit "$rc_all"
