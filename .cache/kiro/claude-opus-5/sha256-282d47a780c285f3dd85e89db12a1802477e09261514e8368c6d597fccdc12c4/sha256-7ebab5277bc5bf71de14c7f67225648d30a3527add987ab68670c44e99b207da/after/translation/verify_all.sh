#!/usr/bin/env bash
# Verify the Rust translation against the C reference for every valid
# build-time feature combination, in both the dev and release profiles, and
# confirm the exported dynamic symbols match.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
CBUILD="$ROOT/c_src/build"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# --- 1. build the C reference -------------------------------------------------
note "building C reference"
mkdir -p "$CBUILD"
( cd "$CBUILD" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
  && cmake --build . >>/tmp/cmake.log 2>&1 ) || { echo "C build FAILED (see /tmp/cmake.log)"; exit 1; }
C_SO="$(find "$CBUILD" -maxdepth 1 -name '*.so' | head -1)"
echo "C .so: $C_SO"

# --- 2. enumerate feature combinations --------------------------------------
# All features declared under [features] in Cargo.toml (excluding "default").
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {sub(/[ \t]*=.*/,""); gsub(/[ \t]/,""); if ($0 != "" && $0 != "default") print}' \
    "$CRATE/Cargo.toml"
)
n=${#FEATURES[@]}
echo "declared features (${n}): ${FEATURES[*]:-<none>}"

COMBOS=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (( mask & (1 << i) )); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
  done
  COMBOS+=("$combo")
done
echo "feature combinations: ${#COMBOS[@]}"

# --- 3. cargo check / test / symbol diff per combination ----------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<no features>}"

  note "cargo check --no-default-features --features '$combo'  [$label]"
  if ! timeout 600 cargo check --manifest-path "$CRATE/Cargo.toml" \
        --no-default-features --features "$combo" 2>&1 | tail -5; then
    echo "CHECK FAILED: $label"; fail=1; continue
  fi

  for profile in dev release; do
    relflag=(); [[ $profile == release ]] && relflag=(--release)

    note "cargo build ($profile) [$label]"
    timeout 600 cargo build --manifest-path "$CRATE/Cargo.toml" \
      --no-default-features --features "$combo" "${relflag[@]}" 2>&1 | tail -3

    R_SO="$(find "$CRATE/target/$([[ $profile == release ]] && echo release || echo debug)" \
            -maxdepth 1 -name '*rgb_to_hsv_lib*.so' | head -1)"
    note "symbol diff ($profile) [$label]"
    if [[ -z "$R_SO" ]]; then
      echo "no Rust .so built"; fail=1
    else
      # Every global symbol the C .so defines must also be defined by the Rust .so.
      csyms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TtDdBbRrWwGgSs]$/ {print $3}' | sort -u)
      rsyms=$(nm -D --defined-only "$R_SO" | awk '{print $NF}' | sort -u)
      missing=$(comm -23 <(echo "$csyms") <(echo "$rsyms"))
      if [[ -n "$missing" ]]; then
        echo "MISSING EXPORTS in Rust .so: $missing"; fail=1
      else
        echo "ok: all C exports present ($(echo "$csyms" | wc -l) symbol(s)): $(echo $csyms)"
      fi
    fi

    note "cargo test ($profile) [$label]"
    if ! timeout 600 cargo test --manifest-path "$CRATE/Cargo.toml" \
          --no-default-features --features "$combo" "${relflag[@]}" 2>&1 | tail -18; then
      echo "TEST FAILED: $label ($profile)"; fail=1
    fi
  done
done

note "RESULT"
if [[ $fail -eq 0 ]]; then echo "ALL COMBINATIONS PASSED"; else echo "FAILURES PRESENT"; fi
exit $fail
