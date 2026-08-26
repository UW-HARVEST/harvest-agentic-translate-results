#!/usr/bin/env bash
# Full verification driver: builds the C and Rust shared libraries and runs the
# whole differential suite for EVERY cargo feature combination, in both the
# debug and the release profile.
#
# Usage: ./run_all.sh
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$PWD"
LOG="${TMPDIR:-/tmp}/hatch_verify.log"
: >"$LOG"

say() { printf '\n\033[1m%s\033[0m\n' "$*"; }
fail=0

# ---------------------------------------------------------------------------
# 0. Enumerate every valid feature combination (powerset of [features]).
# ---------------------------------------------------------------------------
mapfile -t COMBOS < <(python3 - <<'PY'
import re, itertools
t = open("Cargo.toml").read()
m = re.search(r'^\[features\](.*?)(?=^\[|\Z)', t, re.S | re.M)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if '=' in line:
            k = line.split('=')[0].strip().strip('"')
            if k and k != 'default':
                feats.append(k)
# Always test the empty (no-default-features) combination.
combos = []
for r in range(len(feats) + 1):
    for c in itertools.combinations(feats, r):
        combos.append(",".join(c))
for c in combos:
    print(c)
PY
)
say "Feature combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  --no-default-features --features '${c}'"; done
echo "  (plus the default feature set)"

# ---------------------------------------------------------------------------
# 1. Build the C shared library.
# ---------------------------------------------------------------------------
say "[1/6] Building the C shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build . ) >>"$LOG" 2>&1 \
  || { echo "C build FAILED (see $LOG)"; exit 1; }
C_SO="$ROOT/c_src/build/libtranslated_rust.so"
ls -l "$C_SO"

# ---------------------------------------------------------------------------
# 2. cargo check for every feature combination (compile-error gate).
# ---------------------------------------------------------------------------
say "[2/6] cargo check for every feature combination"
for c in "${COMBOS[@]}"; do
  for extra in "" "--all-targets"; do
    printf '  check --no-default-features --features "%s" %s ... ' "$c" "$extra"
    if timeout 600 cargo check --quiet --no-default-features --features "$c" $extra \
         >>"$LOG" 2>&1; then echo ok; else echo FAILED; fail=1; fi
  done
done
for extra in "" "--all-targets"; do
  printf '  check (default features) %s ... ' "$extra"
  if timeout 600 cargo check --quiet $extra >>"$LOG" 2>&1; then echo ok; else echo FAILED; fail=1; fi
done
printf '  check --all-features --all-targets ... '
if timeout 600 cargo check --quiet --all-features --all-targets >>"$LOG" 2>&1; then
  echo ok; else echo FAILED; fail=1; fi

# ---------------------------------------------------------------------------
# 3. Build the Rust cdylib in both profiles.
# ---------------------------------------------------------------------------
say "[3/6] Building the Rust cdylib (debug + release)"
timeout 600 cargo build --quiet >>"$LOG" 2>&1 || { echo "debug build FAILED"; exit 1; }
timeout 600 cargo build --quiet --release >>"$LOG" 2>&1 || { echo "release build FAILED"; exit 1; }
ls -l target/debug/libhatch_lib.so target/release/libhatch_lib.so

# ---------------------------------------------------------------------------
# 4. Symbol parity gate (must reach an EMPTY diff).
# ---------------------------------------------------------------------------
say "[4/6] Symbol parity: nm -D diff must be empty"
for so in target/debug/libhatch_lib.so target/release/libhatch_lib.so; do
  nm -D --defined-only "$C_SO" | awk '$2~/^[TDBWRtdbwr]$/{print $3}' | sort -u \
    >"${TMPDIR:-/tmp}/c_syms.txt"
  nm -D --defined-only "$so" | awk '$2~/^[TDBWRtdbwr]$/{print $3}' | sort -u \
    >"${TMPDIR:-/tmp}/r_syms.txt"
  missing=$(comm -23 "${TMPDIR:-/tmp}/c_syms.txt" "${TMPDIR:-/tmp}/r_syms.txt")
  n_c=$(wc -l <"${TMPDIR:-/tmp}/c_syms.txt")
  if [[ -z "$missing" ]]; then
    echo "  $so: EMPTY diff ($n_c/$n_c C symbols present)"
  else
    echo "  $so: MISSING symbols:"; echo "$missing" | sed 's/^/    /'; fail=1
  fi
done

# ---------------------------------------------------------------------------
# 5. Phase B + C + D differential tests, per feature combination & profile.
# ---------------------------------------------------------------------------
run_suite() {
  local label="$1"; shift
  printf '  %-52s ... ' "$label"
  if timeout 600 "$@" >>"$LOG" 2>&1; then echo ok; else echo FAILED; fail=1; fi
}

say "[5/6] Differential test suite (Phases B, C, D)"
for c in "${COMBOS[@]}"; do
  run_suite "debug   --no-default-features --features '$c'" \
    cargo test --quiet --no-default-features --features "$c" -- --test-threads=1
  # Release: point the crash tests at the release .so so the SIGSEGV parity
  # assertions are strict (debug builds abort in UB precondition checks first).
  HATCH_RUST_SO="$ROOT/target/release/libhatch_lib.so" \
    run_suite "release --no-default-features --features '$c'" \
      cargo test --quiet --release --no-default-features --features "$c" -- --test-threads=1
done
run_suite "debug   (default features)" cargo test --quiet -- --test-threads=1
HATCH_RUST_SO="$ROOT/target/release/libhatch_lib.so" \
  run_suite "release (default features)" cargo test --quiet --release -- --test-threads=1

# Strict fatal-signal parity explicitly against the release .so.
say "  strict SIGSEGV parity against the release .so"
HATCH_RUST_SO="$ROOT/target/release/libhatch_lib.so" \
  run_suite "crash_parity (release .so, strict)" \
    cargo test --quiet --test crash_parity -- --test-threads=1

# ---------------------------------------------------------------------------
# 6. Mutation sanity check: the suite must catch injected divergences.
# ---------------------------------------------------------------------------
say "[6/6] Mutation sanity check"
if timeout 600 ./mutation_check.sh; then echo "  mutation check ok"; else
  echo "  mutation check FAILED"; fail=1; fi

say "==================== RESULT ===================="
if [[ $fail -eq 0 ]]; then
  echo "ALL CHECKS PASSED   (full log: $LOG)"
else
  echo "FAILURES PRESENT    (full log: $LOG)"
fi
exit $fail
