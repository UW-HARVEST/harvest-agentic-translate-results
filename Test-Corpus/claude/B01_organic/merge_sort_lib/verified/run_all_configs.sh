#!/usr/bin/env bash
# Phase D driver: symbol parity + Phases B/C under every build configuration.
#
# Feature combinations are enumerated MECHANICALLY from Cargo.toml (powerset of
# [features]) rather than hard-coded, so this stays correct if features are ever
# added. Today Cargo.toml has no [features] section, so the powerset is the
# single empty combination.
set -u
cd "$(dirname "$0")" || exit 1

RED=$'\033[31m'; GRN=$'\033[32m'; YEL=$'\033[33m'; NC=$'\033[0m'
fails=0
step() { printf '\n%s=== %s ===%s\n' "$YEL" "$*" "$NC"; }
ok()   { printf '%s  ✓ %s%s\n' "$GRN" "$*" "$NC"; }
bad()  { printf '%s  ✗ %s%s\n' "$RED" "$*" "$NC"; fails=$((fails+1)); }

# ---------------------------------------------------------------------------
step "Enumerating build configurations"
# ---------------------------------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        name = line.split('=')[0].strip()
        if name and name != 'default':
            feats.append(name)
print(' '.join(feats))
PY
)
if [ -z "$FEATURES" ]; then
  echo "  Cargo.toml declares NO [features]  =>  1 combination (empty)"
  COMBOS=("")
else
  echo "  features: $FEATURES"
  mapfile -t COMBOS < <(python3 - "$FEATURES" <<'PY'
import sys, itertools
f = sys.argv[1].split()
for r in range(len(f) + 1):
    for c in itertools.combinations(f, r):
        print(','.join(c))
PY
)
fi
echo "  ${#COMBOS[@]} feature combination(s) to verify"

# ---------------------------------------------------------------------------
step "Building the C shared library"
# ---------------------------------------------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) \
  && ok "C .so built" || { bad "C build failed"; exit 1; }
C_SO=$(ls c_src/build/*.so | head -1)
echo "  C_SO=$C_SO"

# ---------------------------------------------------------------------------
step "cargo check for EVERY feature combination"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then args=(--no-default-features); label="<none>"
  else args=(--no-default-features --features "$combo"); label="$combo"; fi
  if timeout 600 cargo check "${args[@]}" --tests >/dev/null 2>&1; then
    ok "cargo check ${args[*]}  (features: $label)"
  else
    bad "cargo check ${args[*]} FAILED"
    timeout 600 cargo check "${args[@]}" --tests 2>&1 | tail -20
  fi
done

# ---------------------------------------------------------------------------
step "Symbol parity (SYMBOLS.md / Phase D gate)"
# ---------------------------------------------------------------------------
run_symbol_diff() {
  local rust_so="$1" label="$2"
  nm -D --defined-only "$C_SO"   | awk '{print $3}' | grep -v '^$' | sort > "${TMPDIR:-/tmp}/c_syms.txt"
  nm -D --defined-only "$rust_so" | awk '{print $3}' | grep -v '^$' | sort > "${TMPDIR:-/tmp}/r_syms.txt"
  local missing
  missing=$(comm -23 "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/r_syms.txt")
  local nc nr
  nc=$(wc -l < "${TMPDIR:-/tmp}/c_syms.txt"); nr=$(wc -l < "${TMPDIR:-/tmp}/r_syms.txt")
  if [ -z "$missing" ]; then
    ok "$label: 0 missing symbols (C exports $nc, Rust exports $nr)"
  else
    bad "$label: symbols missing from Rust .so:"; echo "$missing" | sed 's/^/      /'
  fi
  # No undefined non-libc symbols (i.e. nothing referencing untranslated code).
  local undef
  undef=$(nm -D --undefined-only "$rust_so" | awk '{print $2}' \
          | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__|^_Unwind|^__cxa|^$' || true)
  if [ -z "$undef" ]; then
    ok "$label: 0 undefined non-libc symbols"
  else
    bad "$label: undefined non-libc symbols:"; echo "$undef" | sed 's/^/      /'
  fi
}

# ---------------------------------------------------------------------------
step "Phases B + C under every configuration"
# ---------------------------------------------------------------------------
run_suite() {
  local rust_so="$1" label="$2"
  for t in phase_b_valid phase_c_errors; do
    if RUST_SO="$rust_so" C_SO="$C_SO" timeout 600 cargo test --test "$t" \
         >"${TMPDIR:-/tmp}/$t.log" 2>&1; then
      ok "$label / $t : $(grep -m1 'test result' "${TMPDIR:-/tmp}/$t.log")"
    else
      bad "$label / $t FAILED"
      tail -30 "${TMPDIR:-/tmp}/$t.log" | sed 's/^/      /'
    fi
  done
}

for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then fargs=(--no-default-features); label="features=<none>"
  else fargs=(--no-default-features --features "$combo"); label="features=$combo"; fi

  # --- configuration 1: dev profile (UB checks off, matches unchecked C) ---
  timeout 600 cargo build "${fargs[@]}" >/dev/null 2>&1 \
    || { bad "$label: dev build failed"; continue; }
  run_symbol_diff target/debug/libmerge_sort_lib.so "$label dev"
  run_suite "$PWD/target/debug/libmerge_sort_lib.so" "$label dev"

  # --- configuration 2: release profile (the shipping artifact) ---
  timeout 600 cargo build --release "${fargs[@]}" >/dev/null 2>&1 \
    || { bad "$label: release build failed"; continue; }
  run_symbol_diff target/release/libmerge_sort_lib.so "$label release"
  run_suite "$PWD/target/release/libmerge_sort_lib.so" "$label release"

  # --- configuration 3: UB checks + overflow checks FORCED ON, valid paths ---
  # In-contract input must never trip a Rust unsafe-precondition check or an
  # arithmetic-overflow check. (Out-of-contract rows are excluded: the C has no
  # such checks, so they would abort where the C faults -- see Cargo.toml.)
  if RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on" \
     timeout 600 cargo build "${fargs[@]}" \
       --target-dir "${TMPDIR:-/tmp}/ubchecks" >/dev/null 2>&1; then
    if RUST_SO="${TMPDIR:-/tmp}/ubchecks/debug/libmerge_sort_lib.so" C_SO="$C_SO" \
       timeout 600 cargo test --test phase_b_valid >"${TMPDIR:-/tmp}/ub.log" 2>&1; then
      ok "$label ub-checks-on / phase_b_valid : $(grep -m1 'test result' "${TMPDIR:-/tmp}/ub.log")"
    else
      bad "$label ub-checks-on / phase_b_valid FAILED"
      tail -30 "${TMPDIR:-/tmp}/ub.log" | sed 's/^/      /'
    fi
  else
    bad "$label: ub-checks build failed"
  fi
done

# ---------------------------------------------------------------------------
step "SUMMARY"
# ---------------------------------------------------------------------------
if [ "$fails" -eq 0 ]; then
  printf '%sALL CONFIGURATIONS PASSED%s\n' "$GRN" "$NC"
else
  printf '%s%d CHECK(S) FAILED%s\n' "$RED" "$fails" "$NC"
  exit 1
fi
