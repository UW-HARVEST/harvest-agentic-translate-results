#!/usr/bin/env bash
# Runs the differential suite across every feature combination and both
# profiles. Feature combinations are extracted from Cargo.toml, not hardcoded.
set -uo pipefail
cd "$(dirname "$0")"

fail=0

# --- enumerate features declared in Cargo.toml --------------------------------
feats=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {print $1}
' Cargo.toml | grep -v '^default$' | sort -u)

echo "=== declared non-default features ==="
if [ -z "$feats" ]; then
  echo "(none -- Cargo.toml has no [features] table)"
else
  echo "$feats"
fi

# Build the list of combinations to test: always the default build and the
# no-default-features build; plus the powerset of declared features.
combos=("DEFAULT" "NONE")
if [ -n "$feats" ]; then
  mapfile -t arr <<<"$feats"
  n=${#arr[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then sel="${sel:+$sel,}${arr[i]}"; fi
    done
    combos+=("$sel")
  done
fi

for profile in "" "--release"; do
  for combo in "${combos[@]}"; do
    case "$combo" in
      DEFAULT) fargs=() ;;
      NONE)    fargs=(--no-default-features) ;;
      *)       fargs=(--no-default-features --features "$combo") ;;
    esac
    label="profile=${profile:-debug} features=${combo}"
    echo
    echo "=== $label ==="
    case "$combo" in
      DEFAULT) unset HARNESS_FEATURES ;;
      NONE)    export HARNESS_FEATURES="" ;;
      *)       export HARNESS_FEATURES="$combo" ;;
    esac
    if ! timeout 600 cargo test $profile "${fargs[@]}" 2>&1 | tail -5; then
      echo "FAILED: $label"
      fail=1
    fi
    # Symbol parity for the artifact the harness actually loads.
    hdir=target/harness/$([ -n "$profile" ] && echo release || echo debug)
    if ! nm -D --defined-only "$hdir/libcontrast_ratio_lib.so" | grep -q ' T contrast_ratio$'; then
      echo "FAILED symbol parity: $label"
      fail=1
    fi
  done
done

echo
echo "=== symbol diff (C .so vs harness-built Rust .so) ==="
cso=$(ls ../c_src/build/lib*.so | head -1)
rso=target/harness/release/libcontrast_ratio_lib.so
diff <(nm -D --defined-only "$cso" | awk '$2!="a" && $2!="A" {print $3}' | sort -u) \
     <(nm -D --defined-only "$rso" | awk '{print $3}' | sort -u) \
     | grep '^<' && { echo "FAILED: C-only symbols above"; fail=1; } || echo "no C-only symbols: OK"

echo
echo "=== undefined (imported) symbols in the Rust .so ==="
nm -D --undefined-only "$rso" | awk '{print $NF}' | sort -u | tr '\n' ' '; echo

echo
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit "$fail"
