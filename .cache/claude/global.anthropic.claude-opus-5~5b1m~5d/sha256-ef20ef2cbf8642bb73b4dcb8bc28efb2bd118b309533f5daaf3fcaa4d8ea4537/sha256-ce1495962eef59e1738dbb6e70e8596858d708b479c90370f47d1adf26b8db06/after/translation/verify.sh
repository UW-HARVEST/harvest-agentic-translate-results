#!/usr/bin/env bash
# Phase D driver: symbol parity + the whole differential suite under every
# feature combination and both build profiles.
#
# Usage: ./verify.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
C_SO="$ROOT/c_src/build/libdriver.so"
FAIL=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
bad() { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAIL=1; }
ok()  { printf '  \033[32mok\033[0m   %s\n' "$*"; }

# ---------------------------------------------------------------------------
say "Building the C shared library"
# ---------------------------------------------------------------------------
( mkdir -p "$ROOT/c_src/build" && cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
[ -f "$C_SO" ] && ok "$C_SO" || { bad "missing $C_SO"; exit 1; }

# ---------------------------------------------------------------------------
say "Enumerating feature combinations from Cargo.toml"
# ---------------------------------------------------------------------------
# Every feature declared in [features] (excluding "default").
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' Cargo.toml | sort -u)

if [ -z "$FEATURES" ]; then
  echo "  no [features] declared -> the only configurations are the default"
  echo "  build and --no-default-features (identical code, both exercised)"
  COMBOS=("" "--no-default-features")
else
  # Power set of the declared features, plus the default build.
  mapfile -t FARR <<< "$FEATURES"
  n=${#FARR[@]}
  COMBOS=("")
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=""
    for ((i = 0; i < n; i++)); do
      (( mask & (1 << i) )) && sel="${sel:+$sel,}${FARR[$i]}"
    done
    COMBOS+=("--no-default-features${sel:+ --features $sel}")
  done
fi
printf '  %d combination(s)\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
# Symbol parity: every symbol the C .so exports must be exported by the Rust .so
# ---------------------------------------------------------------------------
symbol_parity() {
  local rust_so="$1" label="$2"
  local c_syms rust_syms missing

  # Defined, global, dynamic symbols. Drop the toolchain-generated ones that
  # every ELF shared object carries (_init/_fini/_edata/__bss_start/_end).
  c_syms=$(nm -D --defined-only "$C_SO" \
           | awk '$2 ~ /^[TtWwDdBbRrVvGgSs]$/ {print $3}' \
           | grep -vE '^(_init|_fini|_edata|__bss_start|_end|__data_start|data_start)$' \
           | sort -u)
  rust_syms=$(nm -D --defined-only "$rust_so" \
              | awk '$2 ~ /^[TtWwDdBbRrVvGgSs]$/ {print $3}' | sort -u)

  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [ -n "$missing" ]; then
    bad "$label: symbols exported by C but MISSING from Rust:"
    echo "$missing" | sed 's/^/         /'
  else
    ok "$label: symbol diff empty ($(echo "$c_syms" | wc -l) C symbol(s), all present)"
  fi

  # Unresolvable (non-libc) undefined symbols in the Rust .so.
  local unresolved
  unresolved=$(ldd -r "$rust_so" 2>&1 | grep -i 'undefined symbol' || true)
  if [ -n "$unresolved" ]; then
    bad "$label: unresolved symbols at load time:"
    echo "$unresolved" | sed 's/^/         /'
  else
    ok "$label: no unresolved symbols (ldd -r clean)"
  fi
}

# ---------------------------------------------------------------------------
# Run everything for each combination, in both debug and release.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<default features>}"

  for profile in debug release; do
    say "combo: $label   profile: $profile"

    relflag=""
    [ "$profile" = release ] && relflag="--release"

    # shellcheck disable=SC2086
    if ! cargo build --offline $relflag $combo >/dev/null 2>&1; then
      bad "cargo build ($label, $profile)"
      continue
    fi
    RUST_SO="target/$profile/libdriver.so"
    [ -f "$RUST_SO" ] || { bad "missing $RUST_SO"; continue; }

    symbol_parity "$RUST_SO" "$label/$profile"

    # Point the tests at THIS .so, so the release cdylib is differentially
    # tested too, not just the debug one the test harness would find by default.
    # shellcheck disable=SC2086
    if DRIVER_RUST_SO="$(pwd)/$RUST_SO" DRIVER_C_SO="$C_SO" \
         cargo test --offline $combo -- --test-threads=1 >"${TMPDIR:-/tmp}/driver-test-$$.log" 2>&1; then
      ok "$label/$profile: $(grep -c '^test .* ok$' "${TMPDIR:-/tmp}/driver-test-$$.log") test(s) passed"
    else
      bad "$label/$profile: differential suite failed"
      tail -n 40 "${TMPDIR:-/tmp}/driver-test-$$.log" | sed 's/^/         /'
    fi
    rm -f "${TMPDIR:-/tmp}/driver-test-$$.log"
  done
done

# ---------------------------------------------------------------------------
say "Suite self-validation (mutation testing)"
# ---------------------------------------------------------------------------
# A suite that passes against a deliberately-broken library proves nothing.
# Each mutant must make the suite FAIL, in the phase named below.
MUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/driver-mutants-XXXXXX")"
trap 'rm -rf "$MUT_DIR"' EXIT

check_mutant() {
  local src="$1" target="$2" desc="$3"
  local so="$MUT_DIR/lib$(basename "$src" .rs).so"

  if ! rustc --crate-type cdylib -O -o "$so" "tests/mutants/$src" >/dev/null 2>&1; then
    bad "could not compile mutant $src"
    return
  fi

  # Expect a NON-zero exit: the suite must reject the mutant.
  if DRIVER_RUST_SO="$so" DRIVER_C_SO="$C_SO" \
       cargo test --offline --test "$target" -- --test-threads=1 >"$MUT_DIR/log" 2>&1; then
    bad "$src: suite PASSED against a known-broken library ($desc) — tests lack detection power"
  else
    local n
    n=$(grep -c '^test .* FAILED$' "$MUT_DIR/log")
    ok "$src: correctly rejected by $n test(s) in '$target' ($desc)"
  fi
}

check_mutant mutant_floor.rs   differential \
  "floor division instead of truncate-toward-zero"
check_mutant mutant_nontrap.rs trap \
  "checked_div swallows the divide-error instead of raising SIGFPE"

# And the converse, which is the whole reason Phase C is a separate phase:
# the non-trapping mutant is INDISTINGUISHABLE on the happy path.
NT_SO="$MUT_DIR/libmutant_nontrap.so"
if [ -f "$NT_SO" ]; then
  if DRIVER_RUST_SO="$NT_SO" DRIVER_C_SO="$C_SO" \
       cargo test --offline --test differential -- --test-threads=1 >"$MUT_DIR/log2" 2>&1; then
    ok "mutant_nontrap.rs passes all of Phase B — confirms Phase C is not redundant"
  else
    echo "  note: mutant_nontrap.rs also failed Phase B (harmless, but unexpected)"
  fi
fi

say "RESULT"
if [ "$FAIL" -eq 0 ]; then
  printf '  \033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '  \033[31mSOME CHECKS FAILED\033[0m\n'
fi
exit "$FAIL"
