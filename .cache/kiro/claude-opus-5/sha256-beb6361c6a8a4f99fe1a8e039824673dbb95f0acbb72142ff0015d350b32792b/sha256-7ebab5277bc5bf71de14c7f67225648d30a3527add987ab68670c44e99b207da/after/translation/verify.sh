#!/usr/bin/env bash
# Full verification matrix for the C-to-Rust translation of driver.c.
#
#   1. enumerate every valid Cargo feature combination
#   2. cargo check each combination
#   3. build the C reference shared library
#   4. build the Rust cdylib (debug and release)
#   5. compare exported dynamic symbols: everything the C .so exports, the
#      Rust .so must export under the same name
#   6. run the differential test suite for each combination, in both profiles
#
# Run from the translation/ directory:  ./verify.sh
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT="$(cd .. && pwd)"
C_LIB="$ROOT/c_src/build/libdriver.so"
TIMEOUT=600
FAILURES=0

note() { printf '\n=== %s ===\n' "$*"; }
fail() { printf '!! FAIL: %s\n' "$*"; FAILURES=$((FAILURES + 1)); }
ok()   { printf '   ok: %s\n' "$*"; }

# --- 1. enumerate feature combinations -------------------------------------
note "Feature combinations"
# Feature names from the [features] table, ignoring the `default` entry.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inside = 1; next }
    /^\[/           { inside = 0 }
    inside && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
    }
  ' Cargo.toml
)
HAS_DEFAULT=$(awk '/^\[features\]/{i=1;next} /^\[/{i=0} i && /^default[[:space:]]*=/{print "yes"}' Cargo.toml)

COMBOS=()
if [ "${#FEATURES[@]}" -eq 0 ]; then
  echo "Cargo.toml declares no [features]; the crate has a single configuration."
  COMBOS=("")
else
  echo "features: ${FEATURES[*]}"
  n=${#FEATURES[@]}
  if [ "$n" -gt 12 ]; then
    echo "too many features to enumerate exhaustively ($n)" >&2
    exit 1
  fi
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((b = 0; b < n; b++)); do
      if (((mask >> b) & 1)); then combo="${combo:+$combo,}${FEATURES[b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"

# --- 2. cargo check every combination --------------------------------------
note "cargo check, every feature combination"
for combo in "${COMBOS[@]}"; do
  label="--no-default-features${combo:+ --features $combo}"
  if timeout "$TIMEOUT" cargo check --no-default-features \
       ${combo:+--features "$combo"} --all-targets >/tmp/check.log 2>&1; then
    ok "cargo check $label"
  else
    fail "cargo check $label"; tail -30 /tmp/check.log
  fi
done
if [ -n "$HAS_DEFAULT" ] || [ "${#FEATURES[@]}" -gt 0 ]; then
  if timeout "$TIMEOUT" cargo check --all-targets >/tmp/check.log 2>&1; then
    ok "cargo check (default features)"
  else
    fail "cargo check (default features)"; tail -30 /tmp/check.log
  fi
fi

# --- 3. build the C reference ----------------------------------------------
note "Build the C reference library"
(
  cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/tmp/cmake.log 2>&1 \
    && cmake --build . >>/tmp/cmake.log 2>&1
) && ok "libdriver.so (C)" || { fail "C build"; tail -20 /tmp/cmake.log; }

# --- 4/5/6. per-profile build, symbol comparison, tests ---------------------
compare_symbols() {
  local rust_lib="$1" profile="$2"
  [ -f "$rust_lib" ] || { fail "missing $rust_lib"; return; }

  # Defined dynamic symbols, name only.
  nm -D --defined-only "$C_LIB"   | awk '{print $NF}' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$rust_lib" | awk '{print $NF}' | sort -u > /tmp/r_syms.txt
  local missing
  missing="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
  if [ -n "$missing" ]; then
    fail "[$profile] Rust .so is missing symbols the C .so exports:"
    echo "$missing" | sed 's/^/       /'
  else
    ok "[$profile] all $(wc -l < /tmp/c_syms.txt) C-exported symbols present in the Rust .so"
  fi

  # Symbol type and binding must line up too, not just the name.
  while read -r sym; do
    local ct rt
    ct="$(nm -D --defined-only "$C_LIB"    | awk -v s="$sym" '$NF==s {print $(NF-1)}' | head -1)"
    rt="$(nm -D --defined-only "$rust_lib" | awk -v s="$sym" '$NF==s {print $(NF-1)}' | head -1)"
    if [ "$ct" != "$rt" ]; then
      fail "[$profile] symbol '$sym' is type '$ct' in C but '$rt' in Rust"
    fi
  done < /tmp/c_syms.txt

  # Same again from the ELF side, which also covers type (FUNC/OBJECT),
  # binding (GLOBAL/WEAK) and visibility (DEFAULT/PROTECTED).
  dynsyms() {
    readelf --wide --dyn-syms "$1" \
      | awk '$1 ~ /^[0-9]+:$/ && NF >= 8 && $4 != "NOTYPE" && $7 != "UND" { print $8, $4, $5, $6 }' \
      | sed 's/@@.*//' | sort -u
  }
  dynsyms "$C_LIB" > /tmp/c_dyn.txt
  dynsyms "$rust_lib" > /tmp/r_dyn.txt
  while read -r name type bind vis; do
    if ! grep -qx "$name $type $bind $vis" /tmp/r_dyn.txt; then
      if grep -q "^$name " /tmp/r_dyn.txt; then
        fail "[$profile] '$name' differs: C has '$type $bind $vis', Rust has '$(grep "^$name " /tmp/r_dyn.txt | head -1 | cut -d' ' -f2-)'"
      else
        fail "[$profile] '$name' ($type $bind $vis) is exported by the C .so but not the Rust .so"
      fi
    fi
  done < /tmp/c_dyn.txt
  ok "[$profile] ELF type/binding/visibility match for every C-exported symbol"
}

for profile in debug release; do
  note "Profile: $profile"
  if [ "$profile" = release ]; then
    timeout "$TIMEOUT" cargo build --release >/tmp/build.log 2>&1 || { fail "cargo build --release"; tail -20 /tmp/build.log; }
  else
    timeout "$TIMEOUT" cargo build >/tmp/build.log 2>&1 || { fail "cargo build"; tail -20 /tmp/build.log; }
  fi
  compare_symbols "target/$profile/libdriver.so" "$profile"

  for combo in "${COMBOS[@]}"; do
    label="$profile${combo:+ / $combo}"
    args=(--no-default-features)
    [ -n "$combo" ] && args+=(--features "$combo")
    [ "$profile" = release ] && args+=(--release)
    if timeout "$TIMEOUT" cargo test "${args[@]}" >/tmp/test.log 2>&1; then
      ok "cargo test [$label] - $(grep -c '^test .* ok$' /tmp/test.log) tests passed"
    else
      fail "cargo test [$label]"
      grep -A6 -E '^(failures:|---- )' /tmp/test.log | head -60
    fi
  done
done

note "Summary"
if [ "$FAILURES" -eq 0 ]; then
  echo "All checks passed."
else
  echo "$FAILURES check(s) failed."
fi
exit "$FAILURES"
