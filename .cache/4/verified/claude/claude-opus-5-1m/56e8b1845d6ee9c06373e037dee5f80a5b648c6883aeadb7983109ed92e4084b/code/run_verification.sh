#!/bin/bash
# Full differential verification of the C -> Rust translation.
#
#   Phase A : build both .so's, enumerate every feature combination
#   Phase B : valid-path differential tests   (tests/phase_b_configs.rs)
#   Phase C : error-path differential tests   (tests/phase_c_errors.rs)
#   Phase D : symbol parity + repeat B/C for every feature combination
#
# The feature powerset is derived MECHANICALLY from Cargo.toml, never hard-coded.
set -uo pipefail
cd "$(dirname "$0")"
FAIL=0
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }

# --------------------------------------------------------------------------
step "Phase A.1 - build the C shared library"
# --------------------------------------------------------------------------
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) > "${TMPDIR:-/tmp}/c_build.log" 2>&1 \
  && ok "C .so built" || { bad "C build failed (see \$TMPDIR/c_build.log)"; tail -20 "${TMPDIR:-/tmp}/c_build.log"; exit 1; }
C_SO=$(find c_src/build -name '*.so' | head -1)
echo "  C_SO=$C_SO"

# --------------------------------------------------------------------------
step "Phase A.2 - enumerate every feature combination"
# --------------------------------------------------------------------------
FEATURES=$(python3 - <<'PY'
import re
t = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', t, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip()
        if k and k != 'default':
            names.append(k)
print(" ".join(names))
PY
)
echo "  declared features: [${FEATURES}]"
combos=("")
for f in $FEATURES; do
  new=()
  for c in "${combos[@]}"; do new+=("$c" "${c:+$c,}$f"); done
  combos=("${new[@]}")
done
echo "  ${#combos[@]} feature combination(s): $(printf '{%s} ' "${combos[@]}")"

# --------------------------------------------------------------------------
step "Phase A.3 - cargo check for EVERY feature combination"
# --------------------------------------------------------------------------
for c in "${combos[@]}"; do
  if timeout 600 cargo check --all-targets --no-default-features --features "$c" \
       > "${TMPDIR:-/tmp}/check.log" 2>&1; then
    ok "cargo check --no-default-features --features '$c'"
  else
    bad "cargo check --features '$c'"; tail -30 "${TMPDIR:-/tmp}/check.log"
  fi
done
if timeout 600 cargo check --all-targets > "${TMPDIR:-/tmp}/check.log" 2>&1; then
  ok "cargo check (default features)"
else
  bad "cargo check (default features)"; tail -30 "${TMPDIR:-/tmp}/check.log"
fi

# --------------------------------------------------------------------------
step "Phases B+C+D - differential tests for EVERY combination x profile"
# --------------------------------------------------------------------------
for c in "${combos[@]}"; do
  for profile in debug release; do
    flag=""; [ "$profile" = release ] && flag="--release"
    # Build the cdylib for this profile/combo, then point the tests at it so the
    # tests always load the .so that matches what was just built.
    if ! timeout 600 cargo build $flag --no-default-features --features "$c" \
         > "${TMPDIR:-/tmp}/build.log" 2>&1; then
      bad "cargo build $profile features='$c'"; tail -30 "${TMPDIR:-/tmp}/build.log"; continue
    fi
    RS="target/$profile/libinreftree_lib.so"
    if [ ! -f "$RS" ]; then bad "missing $RS"; continue; fi
    if C_SO="$C_SO" RUST_SO="$RS" timeout 600 cargo test $flag \
         --no-default-features --features "$c" -- --test-threads=4 \
         > "${TMPDIR:-/tmp}/test.log" 2>&1; then
      n=$(grep -c '^test .* \.\.\. ok$' "${TMPDIR:-/tmp}/test.log")
      ok "cargo test [$profile] features='$c'  ($n tests passed)"
    else
      bad "cargo test [$profile] features='$c'"
      grep -E "^(test .* FAILED|failures:|thread|assertion|error)" "${TMPDIR:-/tmp}/test.log" | head -40
    fi
  done
done

# --------------------------------------------------------------------------
step "Phases B+C - high-volume randomized sweep across several seeds"
# --------------------------------------------------------------------------
for c in "${combos[@]}"; do
  for seed in 1 42 999 123456789 3141592653589793238; do
    if C_SO="$C_SO" RUST_SO="target/release/libinreftree_lib.so" \
       FUZZ_SEED="$seed" FUZZ_ITERS=300000 FUZZ_ROUNDS=1500 \
       timeout 600 cargo test --release --no-default-features --features "$c" \
         --test fuzz_sweep -- --ignored --test-threads=1 \
         > "${TMPDIR:-/tmp}/fuzz.log" 2>&1; then
      ok "fuzz sweep seed=$seed features='$c'"
    else
      bad "fuzz sweep seed=$seed features='$c'"
      grep -E "assertion|mismatch|panicked" "${TMPDIR:-/tmp}/fuzz.log" | head -20
    fi
  done
done

# --------------------------------------------------------------------------
step "Phase D - nm -D symbol diff (must be empty)"
# --------------------------------------------------------------------------
for profile in debug release; do
  RS="target/$profile/libinreftree_lib.so"
  [ -f "$RS" ] || continue
  nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u > "${TMPDIR:-/tmp}/c.syms"
  nm -D --defined-only "$RS"   | awk '{print $NF}' | sort -u > "${TMPDIR:-/tmp}/r.syms"
  miss=$(comm -23 "${TMPDIR:-/tmp}/c.syms" "${TMPDIR:-/tmp}/r.syms")
  if [ -z "$miss" ]; then
    ok "[$profile] every C symbol ($(wc -l < "${TMPDIR:-/tmp}/c.syms")) is exported by the Rust .so"
  else
    bad "[$profile] Rust .so is missing: $(echo $miss)"
  fi
  # exported data object sizes
  csz=$(nm -D -S --defined-only "$C_SO" | awk '$NF=="node_table"{print $2}')
  rsz=$(nm -D -S --defined-only "$RS"   | awk '$NF=="node_table"{print $2}')
  [ "$csz" = "$rsz" ] && ok "[$profile] node_table size 0x$csz matches" \
                      || bad "[$profile] node_table size C=0x$csz Rust=0x$rsz"
  csz=$(nm -D -S --defined-only "$C_SO" | awk '$NF=="node_count"{print $2}')
  rsz=$(nm -D -S --defined-only "$RS"   | awk '$NF=="node_count"{print $2}')
  [ "$csz" = "$rsz" ] && ok "[$profile] node_count size 0x$csz matches" \
                      || bad "[$profile] node_count size C=0x$csz Rust=0x$rsz"
done

# --------------------------------------------------------------------------
step "Robustness - the same Rust .so vs the C built at every optimization level"
# --------------------------------------------------------------------------
# `inreftree` reads op_string[negative] (ERRORS.md row 26), which is UB, so the
# bytes it sees depend on how the C was compiled. Rebuild the C with several
# optimization levels (into $TMPDIR - c_src/ is never modified) and re-run the
# whole suite against each, to prove the translation is not tied to one build.
ALT="${TMPDIR:-/tmp}/altc"; mkdir -p "$ALT"
for opt in -O0 -O1 -O2 -O3 -Os; do
  if ! gcc "$opt" -fPIC -shared -Ic_src/include -o "$ALT/lib$opt.so" c_src/src/lib.c 2>/dev/null; then
    bad "could not build the C at $opt"; continue
  fi
  if C_SO="$ALT/lib$opt.so" RUST_SO="target/release/libinreftree_lib.so" NO_AUTO_BUILD=1 \
     timeout 600 cargo test --release -- --test-threads=4 > "${TMPDIR:-/tmp}/alt.log" 2>&1; then
    ok "suite passes against the C built at $opt"
  else
    bad "suite FAILS against the C built at $opt"
    grep -oE "^test [a-z_0-9]+ \.\.\. FAILED" "${TMPDIR:-/tmp}/alt.log" | head -10
  fi
done

# --------------------------------------------------------------------------
step "Suite adequacy - mutation testing (the suite must be able to FAIL)"
# --------------------------------------------------------------------------
if timeout 600 ./mutation_test.sh > "${TMPDIR:-/tmp}/mutation.out" 2>&1; then
  ok "$(grep -oE 'mutation score: .*' "${TMPDIR:-/tmp}/mutation.out")"
else
  bad "mutation testing found blind spots:"
  grep -E "ESCAPED|mutation score" "${TMPDIR:-/tmp}/mutation.out" | head -20
fi

# --------------------------------------------------------------------------
step "Completion gate"
# --------------------------------------------------------------------------
for f in SYMBOLS.md ERRORS.md CONFIGS.md; do
  [ -s "$f" ] && ok "$f present" || bad "$f missing"
done
unchecked=$(grep -c '^| *[0-9]* *|.*\[ \]' ERRORS.md CONFIGS.md 2>/dev/null | awk -F: '{s+=$2} END{print s+0}')
[ "$unchecked" = 0 ] && ok "no unchecked rows in ERRORS.md / CONFIGS.md" \
                     || bad "$unchecked unchecked row(s) remain"

if [ "$FAIL" = 0 ]; then
  printf '\n\033[1;32mVERIFICATION COMPLETE - all phases passed\033[0m\n'
else
  printf '\n\033[1;31mVERIFICATION FAILED\033[0m\n'
fi
exit $FAIL
