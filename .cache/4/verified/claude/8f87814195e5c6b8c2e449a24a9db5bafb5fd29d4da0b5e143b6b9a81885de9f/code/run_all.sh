#!/usr/bin/env bash
# Full verification driver: builds the C .so, enumerates every Cargo feature
# combination, cargo-checks each, builds the Rust cdylib for each profile,
# diffs exported symbols, and runs the Phase B + Phase C differential suites.
#
#   ./run_all.sh              # full run
#   ITERS=200 ./run_all.sh    # quick run
set -uo pipefail

cd "$(dirname "$0")"
ROOT="$PWD"
LOGDIR="${TMPDIR:-/tmp}/harvest-verify"
mkdir -p "$LOGDIR"
CARGO_FLAGS="--offline"
ITERS="${ITERS:-}"
FAILURES=0

say() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()  { printf '  [ok]   %s\n' "$*"; }
bad() { printf '  [FAIL] %s\n' "$*"; FAILURES=$((FAILURES + 1)); }

# ---------------------------------------------------------------------------
# 0. Enumerate feature combinations from Cargo.toml
# ---------------------------------------------------------------------------
say "Feature combinations declared in Cargo.toml"
mapfile -t FEATURES < <(
  awk '/^\[features\]/{f=1;next} /^\[/{f=0} f && /=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default" && a[1] != "") print a[1]}' Cargo.toml
)
echo "  optional features: ${FEATURES[*]:-<none>}"

# Every subset of the optional features (plus the empty set).
COMBOS=()
n=${#FEATURES[@]}
for ((mask = 0; mask < (1 << n); mask++)); do
  sel=()
  for ((i = 0; i < n; i++)); do
    (((mask >> i) & 1)) && sel+=("${FEATURES[$i]}")
  done
  COMBOS+=("$(IFS=,; echo "${sel[*]}")")
done
echo "  combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "    - '--no-default-features --features ${c:-<empty>}'"; done

# ---------------------------------------------------------------------------
# 1. Build the C shared library
# ---------------------------------------------------------------------------
say "Building the C shared library"
(
  mkdir -p c_src/build && cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
    && cmake --build .
) >"$LOGDIR/c_build.log" 2>&1
C_SO="$ROOT/c_src/build/libtranslated_rust.so"
if [[ -f "$C_SO" ]]; then ok "$C_SO"; else bad "C build failed, see $LOGDIR/c_build.log"; exit 1; fi

nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u >"$LOGDIR/c_syms.txt"
echo "  C exports: $(wc -l <"$LOGDIR/c_syms.txt") symbol(s)"

say "cargo check with the aggregate feature flags"
for extra in "--no-default-features" "--all-features" ""; do
  if cargo check $CARGO_FLAGS $extra --all-targets >"$LOGDIR/check_agg.log" 2>&1; then
    ok "cargo check ${extra:-<default features>}"
  else
    bad "cargo check ${extra:-<default features>} failed"; tail -20 "$LOGDIR/check_agg.log"
  fi
done

# Symbols that are toolchain artifacts, not library API.
IGNORE_RE='^(_ITM_(de)?registerTMCloneTable|__gmon_start__|__cxa_finalize.*|_init|_fini|__bss_start|_edata|_end)$'

# ---------------------------------------------------------------------------
# 2..4  For each combination x profile: check, build, symbol-diff, test
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  label="${combo:-<empty>}"
  featflag=(--no-default-features)
  [[ -n "$combo" ]] && featflag+=(--features "$combo")

  say "cargo check  [features: $label]"
  if cargo check $CARGO_FLAGS "${featflag[@]}" --all-targets >"$LOGDIR/check_$label.log" 2>&1; then
    ok "cargo check clean"
  else
    bad "cargo check failed (see $LOGDIR/check_$label.log)"
    tail -30 "$LOGDIR/check_$label.log"
    continue
  fi

  for profile in debug release; do
    pflag=()
    [[ $profile == release ]] && pflag=(--release)

    say "build + symbol parity + tests  [features: $label, profile: $profile]"
    if ! cargo build $CARGO_FLAGS "${featflag[@]}" "${pflag[@]}" \
        >"$LOGDIR/build_${label}_$profile.log" 2>&1; then
      bad "cargo build failed (see $LOGDIR/build_${label}_$profile.log)"
      tail -30 "$LOGDIR/build_${label}_$profile.log"
      continue
    fi
    RUST_SO="$ROOT/target/$profile/libhsv_to_rgb_lib.so"
    if [[ ! -f "$RUST_SO" ]]; then bad "missing $RUST_SO"; continue; fi
    ok "built $RUST_SO"

    # --- symbol parity (Phase D) ---
    nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u >"$LOGDIR/rust_syms.txt"
    missing=$(comm -23 "$LOGDIR/c_syms.txt" "$LOGDIR/rust_syms.txt" | grep -Ev "$IGNORE_RE" || true)
    if [[ -z "$missing" ]]; then
      ok "symbol parity: every C export is exported by Rust"
    else
      bad "symbols exported by C but MISSING from Rust:"
      echo "$missing" | sed 's/^/         /'
    fi
    undef=$(nm -D --undefined-only "$RUST_SO" | awk '{print $NF}' \
            | grep -Ev "$IGNORE_RE|@GLIBC|@GCC|^__" | sort -u || true)
    if [[ -z "$undef" ]]; then
      ok "no unresolved non-libc imports in the Rust .so"
    else
      bad "unresolved non-libc imports:"; echo "$undef" | sed 's/^/         /'
    fi

    # --- differential tests (Phase B + C) ---
    export HARVEST_C_LIB="$C_SO"
    export HARVEST_RUST_LIB="$RUST_SO"
    [[ -n "$ITERS" ]] && export HARVEST_ITERS="$ITERS"
    tlog="$LOGDIR/test_${label}_$profile.log"
    if cargo test $CARGO_FLAGS "${featflag[@]}" "${pflag[@]}" \
        -- --test-threads="$(nproc)" >"$tlog" 2>&1; then
      ok "differential suites passed ($(grep -c '^test .* \.\.\. ok' "$tlog") tests)"
    else
      bad "differential suites FAILED (see $tlog)"
      grep -E "^test .* FAILED|DIVERGENCE|panicked at|test result" "$tlog" | head -40 | sed 's/^/         /'
    fi
    unset HARVEST_C_LIB HARVEST_RUST_LIB
  done
done

say "SUMMARY"
if ((FAILURES == 0)); then
  echo "  ALL CHECKS PASSED"
else
  echo "  $FAILURES check(s) failed"
fi
exit $((FAILURES > 0))
