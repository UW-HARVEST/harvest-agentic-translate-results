#!/usr/bin/env bash
# Phase D driver: build the C shared object, then run `cargo check` + the full
# differential suite under EVERY valid feature combination, in both profiles.
#
# Feature names are extracted from Cargo.toml rather than hard-coded, so the
# sweep stays correct if features are ever added.
set -u
cd "$(dirname "$0")"

fail=0
note() { printf '\n=== %s ===\n' "$*"; }

# --------------------------------------------------------------------------
# 0. Enumerate feature combinations (powerset of the non-`default` features).
# --------------------------------------------------------------------------
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ {inf=1; next}
    /^\[/           {inf=0}
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
n=${#FEATURES[@]}
note "feature axes discovered in Cargo.toml: $n ${FEATURES[*]:-(none)}"

COMBOS=()
for ((mask = 0; mask < (1 << n); mask++)); do
  combo=""
  for ((i = 0; i < n; i++)); do
    if (((mask >> i) & 1)); then combo="${combo:+$combo,}${FEATURES[i]}"; fi
  done
  COMBOS+=("$combo")
done
echo "valid feature combinations: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  --no-default-features --features '${c}'"; done

# --------------------------------------------------------------------------
# 1. Build the C artifacts (shared object + the cmake executable).
# --------------------------------------------------------------------------
note "building the C shared object and cmake executable"
mkdir -p build_c
gcc -fPIC -shared -o build_c/libdriver_c.so c_src/src/main.c -lm || fail=1
( mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) || fail=1
TMP="${TMPDIR:-.}/cfgsweep.$$"
mkdir -p "$TMP"
nm -D --defined-only build_c/libdriver_c.so | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort > "$TMP/c_syms"
echo "C .so exports: $(tr '\n' ' ' < "$TMP/c_syms")"

# --------------------------------------------------------------------------
# 2. cargo check / build / test for every combination x profile.
# --------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<empty>}"
  for profile in debug release; do
    relflag=""; [ "$profile" = release ] && relflag="--release"

    note "check: features='$label' profile=$profile"
    if ! cargo check --offline --all-targets --no-default-features \
           --features "$combo" $relflag 2>&1 | tail -3; then
      echo "CHECK FAILED"; fail=1; continue
    fi

    note "build + symbol diff: features='$label' profile=$profile"
    cargo build --offline --no-default-features --features "$combo" $relflag \
      --lib --bins 2>&1 | tail -2
    so="target/$profile/libdriver.so"
    if [ ! -f "$so" ]; then echo "MISSING $so"; fail=1; continue; fi
    nm -D --defined-only "$so" | awk '$2 ~ /^[A-Z]$/ {print $3}' | sort > "$TMP/r_syms"
    missing=$(comm -23 "$TMP/c_syms" "$TMP/r_syms")
    extra=$(comm -13 "$TMP/c_syms" "$TMP/r_syms")
    if [ -n "$missing" ]; then
      echo "SYMBOLS MISSING FROM RUST: $missing"; fail=1
    else
      echo "symbol diff empty (missing: none, extra: ${extra:-none})"
    fi

    note "test: features='$label' profile=$profile"
    if timeout 600 cargo test --offline --no-default-features \
         --features "$combo" $relflag -- --test-threads=1 2>&1 | grep -E "test result|^error"; then
      :
    fi
    if ! timeout 600 cargo test --offline --no-default-features \
           --features "$combo" $relflag -- --test-threads=1 >/dev/null 2>&1; then
      echo "TESTS FAILED for features='$label' profile=$profile"; fail=1
    fi
  done
done

# --------------------------------------------------------------------------
# 3. --all-features and the default configuration, for completeness.
# --------------------------------------------------------------------------
for extra in "--all-features" ""; do
  note "test: ${extra:-<default features>}"
  if ! timeout 600 cargo test --offline $extra -- --test-threads=1 2>&1 \
        | grep -E "test result"; then
    echo "TESTS FAILED ($extra)"; fail=1
  fi
done

rm -rf "$TMP"
note "RESULT"
if [ "$fail" -eq 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES PRESENT"; fi
exit "$fail"
