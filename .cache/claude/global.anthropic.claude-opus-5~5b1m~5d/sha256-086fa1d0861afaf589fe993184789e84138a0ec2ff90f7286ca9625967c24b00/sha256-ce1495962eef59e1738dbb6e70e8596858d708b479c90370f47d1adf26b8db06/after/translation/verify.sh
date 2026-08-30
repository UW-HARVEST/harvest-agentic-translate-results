#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Full verification driver: Phase D symbol parity + Phases B/C under every
# Cargo feature combination and every build profile.
#
#   ./verify.sh            # everything
#   ./verify.sh symbols    # symbol parity only
# ---------------------------------------------------------------------------
set -uo pipefail
cd "$(dirname "$0")"
CRATE_DIR=$PWD
C_DIR=$CRATE_DIR/../c_src
C_SO=$C_DIR/build/libdriver.so

fail=0
step() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; fail=1; }

# ---------------------------------------------------------------------------
# Build the C shared library
# ---------------------------------------------------------------------------
step "Building the C shared library"
mkdir -p "$C_DIR/build"
( cd "$C_DIR/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
[ -f "$C_SO" ] && ok "$C_SO" || { bad "missing $C_SO"; exit 1; }

# ---------------------------------------------------------------------------
# Enumerate feature combinations straight out of Cargo.toml
# ---------------------------------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re, sys
txt = open("Cargo.toml").read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            n = line.split('=')[0].strip().strip('"')
            if n != 'default':
                names.append(n)
print(' '.join(names))
PY
)

COMBOS=()
COMBOS+=("default:")                       # label:flags
COMBOS+=("no-default-features:--no-default-features")
if [ -n "$FEATURES" ]; then
  # every non-empty subset of the declared features, on top of --no-default-features
  python3 - "$FEATURES" <<'PY' > /tmp/.combos.$$
import itertools, sys
feats = sys.argv[1].split()
for r in range(1, len(feats) + 1):
    for c in itertools.combinations(feats, r):
        print("%s:--no-default-features --features %s" % ('+'.join(c), ','.join(c)))
PY
  while IFS= read -r l; do COMBOS+=("$l"); done < /tmp/.combos.$$
  rm -f /tmp/.combos.$$
  echo "declared features: $FEATURES"
else
  echo "Cargo.toml declares no [features] table -> 2 combinations (default, --no-default-features)"
fi

# ---------------------------------------------------------------------------
# Phase D — symbol parity, per feature combination and per profile
# ---------------------------------------------------------------------------
symbol_parity() {
  local label=$1 flags=$2 profile=$3 pflag=$4 outdir=$5
  # shellcheck disable=SC2086
  cargo build --offline $pflag $flags >/dev/null 2>&1 \
    || { bad "cargo build ($label/$profile)"; return; }
  local rust_so="$CRATE_DIR/target/$outdir/libdriver.so"
  if [ ! -f "$rust_so" ]; then bad "no cdylib at $rust_so ($label/$profile)"; return; fi

  local c_syms rust_syms missing extra
  c_syms=$(nm -D --defined-only "$C_SO"   | awk '{print $NF}' | sort -u)
  rust_syms=$(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [ -n "$missing" ]; then
    bad "$label/$profile: symbols exported by C but MISSING from Rust:"
    echo "$missing" | sed 's/^/         /'
  else
    ok "$label/$profile: symbol diff empty ($(echo "$c_syms" | wc -l) C symbols all present)"
  fi
  extra=$(comm -13 <(echo "$c_syms") <(echo "$rust_syms"))
  [ -n "$extra" ] && printf '         note: Rust additionally exports: %s\n' "$(echo "$extra" | tr '\n' ' ')"

  # No dangling non-libc imports.
  local undef
  undef=$(nm -D --undefined-only "$rust_so" \
          | grep -v 'GLIBC\|GCC_\|_ITM_\|__gmon_start__' || true)
  if [ -n "$undef" ]; then
    bad "$label/$profile: undefined non-libc symbols:"; echo "$undef" | sed 's/^/         /'
  else
    ok "$label/$profile: 0 undefined non-libc symbols"
  fi

  # The C's `static` functions/objects must NOT be exported by either side.
  local leaked=""
  for s in the_house parse_val add_floor add_bedrooms add_floor_to_the_house print_the_house; do
    if nm -D --defined-only "$rust_so" | awk '{print $NF}' | grep -qx "$s"; then
      leaked="$leaked $s"
    fi
  done
  [ -n "$leaked" ] && bad "$label/$profile: Rust leaks C-static symbols:$leaked" \
                   || ok "$label/$profile: no C-static symbols leaked"
}

step "Phase D — symbol parity"
for combo in "${COMBOS[@]}"; do
  label=${combo%%:*}; flags=${combo#*:}
  symbol_parity "$label" "$flags" debug   ""          debug
  symbol_parity "$label" "$flags" release "--release" release
done

# ---------------------------------------------------------------------------
# Phases B & C — differential tests, per feature combination.
#
# Also run each combination's test suite against the RELEASE cdylib (via
# RUST_DRIVER_SO) so an optimised build is differentially tested too.
# ---------------------------------------------------------------------------
step "Phases B & C — differential tests"
for combo in "${COMBOS[@]}"; do
  label=${combo%%:*}; flags=${combo#*:}

  # debug cdylib
  # shellcheck disable=SC2086
  cargo build --offline $flags >/dev/null 2>&1 || { bad "build $label (debug)"; continue; }
  # shellcheck disable=SC2086
  if timeout 600 cargo test --offline $flags >/dev/null 2>&1; then
    ok "cargo test $flags  (against target/debug/libdriver.so)"
  else
    bad "cargo test $flags  (against target/debug/libdriver.so)"
    # shellcheck disable=SC2086
    timeout 600 cargo test --offline $flags 2>&1 | tail -30 | sed 's/^/         /'
  fi

  # release cdylib, tests still built in the test profile
  # shellcheck disable=SC2086
  cargo build --offline --release $flags >/dev/null 2>&1 || { bad "build $label (release)"; continue; }
  # shellcheck disable=SC2086
  if RUST_DRIVER_SO="$CRATE_DIR/target/release/libdriver.so" \
     timeout 600 cargo test --offline $flags >/dev/null 2>&1; then
    ok "cargo test $flags  (against target/release/libdriver.so)"
  else
    bad "cargo test $flags  (against target/release/libdriver.so)"
  fi
done

step "Summary"
if [ "$fail" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$fail"
