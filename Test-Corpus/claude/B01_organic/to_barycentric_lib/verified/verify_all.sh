#!/usr/bin/env bash
# Full verification driver: enumerates every Cargo feature combination, runs
# `cargo check` on each, builds the C reference `.so`, diffs the exported symbol
# tables, and runs the whole differential suite (Phases B and C) for every
# feature combination against BOTH the debug and release Rust `.so`.
#
# Usage:  ./verify_all.sh [DIFF_SCALE]   e.g. `./verify_all.sh 10` for a soak run
set -uo pipefail

cd "$(dirname "$0")" || exit 1
ROOT=$PWD
SCALE=${1:-}
FAILED=0
CARGO_FLAGS=(--offline)

say() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
ok()  { printf '  \033[32mPASS\033[0m %s\n' "$*"; }
bad() { printf '  \033[31mFAIL\033[0m %s\n' "$*"; FAILED=$((FAILED + 1)); }

# ---------------------------------------------------------------------------
# Phase A.1 — enumerate feature combinations (powerset of the declared features,
# excluding `default` itself, plus the explicit --no-default-features case).
# ---------------------------------------------------------------------------
say "Feature combinations"
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf = 1; next }
    /^\[/           { inf = 0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1];
    }
  ' Cargo.toml
)
NF=${#FEATURES[@]}
echo "declared non-default features: ${NF} ${FEATURES[*]:-(none)}"

COMBOS=("")                      # empty feature set (== `default` here)
if [ "$NF" -gt 0 ]; then
  for ((m = 1; m < (1 << NF); m++)); do
    c=""
    for ((i = 0; i < NF; i++)); do
      if (( m & (1 << i) )); then c="${c:+$c,}${FEATURES[$i]}"; fi
    done
    COMBOS+=("$c")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - '--no-default-features${c:+ --features $c}'"; done

# ---------------------------------------------------------------------------
# Phase A.2 — cargo check every combination (default + each explicit combo)
# ---------------------------------------------------------------------------
say "cargo check, every feature combination"
if timeout 600 cargo check "${CARGO_FLAGS[@]}" --all-targets >/dev/null 2>&1; then
  ok "cargo check (default features)"
else
  bad "cargo check (default features)"
fi
for c in "${COMBOS[@]}"; do
  args=(check "${CARGO_FLAGS[@]}" --all-targets --no-default-features)
  [ -n "$c" ] && args+=(--features "$c")
  if timeout 600 cargo "${args[@]}" >/dev/null 2>&1; then
    ok "cargo check --no-default-features${c:+ --features $c}"
  else
    bad "cargo check --no-default-features${c:+ --features $c}"
  fi
done

# ---------------------------------------------------------------------------
# Phase A.3 — build the C reference shared library
# ---------------------------------------------------------------------------
say "Build C reference .so"
mkdir -p c_src/build
if (cd c_src/build \
      && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
      && cmake --build . >/dev/null); then
  ok "cmake build"
else
  bad "cmake build"
fi
C_SO=$(ls -1 c_src/build/*.so 2>/dev/null | head -1)
echo "C .so: ${C_SO:-<none>}"

# ---------------------------------------------------------------------------
# Build the Rust cdylib in both profiles
# ---------------------------------------------------------------------------
say "Build Rust cdylib (debug + release)"
timeout 600 cargo build "${CARGO_FLAGS[@]}"           >/dev/null 2>&1 || bad "cargo build (debug)"
timeout 600 cargo build "${CARGO_FLAGS[@]}" --release >/dev/null 2>&1 || bad "cargo build (release)"
RS_DEBUG=$ROOT/target/debug/libto_barycentric_lib.so
RS_RELEASE=$ROOT/target/release/libto_barycentric_lib.so
ls -l "$RS_DEBUG" "$RS_RELEASE" 2>/dev/null | sed 's/^/  /'

# ---------------------------------------------------------------------------
# Phase D.1 — symbol parity
# ---------------------------------------------------------------------------
say "Phase D: symbol parity (nm -D)"
for rs in "$RS_DEBUG" "$RS_RELEASE"; do
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TWiD]$/ {print $3}' | sort -u) \
    <(nm -D --defined-only "$rs"   | awk '$2 ~ /^[TWiD]$/ {print $3}' | sort -u))
  if [ -z "$missing" ]; then
    ok "no C symbol missing from $(basename "$(dirname "$rs")")/$(basename "$rs")"
  else
    bad "missing from $rs: $(echo "$missing" | tr '\n' ' ')"
  fi
done
undef=$(nm -D --undefined-only "$RS_RELEASE" | awk '{print $2}' | sed 's/@.*//' \
        | sort -u | grep -vE '^(memcpy|memmove|memset|memcmp|bcmp|abort|write|getenv|calloc|malloc|free|realloc|posix_memalign|dl_iterate_phdr|pthread_[a-z_]*|__[A-Za-z0-9_]*|_[A-Z][A-Za-z0-9_]*|sysconf|open|close|read|mmap|munmap|mprotect|sigaction|sigaltstack|signal|strlen|syscall|readlink|getcwd|environ|fstat64|stat64|statx|lseek64|mmap64|open64|realpath|writev|gettid)$' || true)
if [ -z "$undef" ]; then
  ok "no unresolved non-libc symbols in the Rust .so"
else
  echo "  note: remaining undefined symbols (verify these are libc/runtime): $(echo "$undef" | tr '\n' ' ')"
fi

# ---------------------------------------------------------------------------
# Phases B + C — differential suite, every feature combination x both profiles
# ---------------------------------------------------------------------------
say "Phases B + C: differential suite"
EXPECTED_TESTS=$(grep -ch '^fn \|^#\[test\]' tests/phase_b_configs.rs tests/phase_c_errors.rs \
                 | paste -sd+ - | bc 2>/dev/null || echo 0)

run_suite() { # $1 = label, $2 = rust .so, rest = extra cargo args
  local label=$1 so=$2; shift 2
  local out rc nok nbad nsuites
  # NOTE: env assignments must be passed via `env`, not as an expanded word
  # prefix — bash only recognises literal `VAR=x cmd` prefixes at parse time.
  local -a envv=(C_SO_PATH="$ROOT/$C_SO" RUST_SO_PATH="$so")
  [ -n "$SCALE" ] && envv+=(DIFF_SCALE="$SCALE")
  out=$(env "${envv[@]}" timeout 600 cargo test "${CARGO_FLAGS[@]}" --release "$@" 2>&1)
  rc=$?
  nok=$(echo "$out" | grep -c '\.\.\. ok$')
  nbad=$(echo "$out" | grep -c 'FAILED')
  nsuites=$(echo "$out" | grep -c '^test result: ok\.')
  if [ "$rc" -ne 0 ] || [ "$nbad" -ne 0 ]; then
    bad "$label (rc=$rc)"
    echo "$out" | grep -E 'FAILED|MISMATCH|panicked|assertion|^error' | head -20 | sed 's/^/      /'
  elif [ "$nsuites" -lt 3 ] || [ "$nok" -lt "$EXPECTED_TESTS" ]; then
    # 3 = lib unittests + phase_b + phase_c; guards against a silently
    # skipped/empty run being reported as a pass.
    bad "$label ran only $nok/$EXPECTED_TESTS tests across $nsuites suites"
    echo "$out" | tail -20 | sed 's/^/      /'
  else
    ok "$label  ($nok tests)"
  fi
}

for c in "${COMBOS[@]}"; do
  featargs=(--no-default-features)
  [ -n "$c" ] && featargs+=(--features "$c")
  lbl="features='${c:-<empty>}'"
  run_suite "$lbl vs release .so" "$RS_RELEASE" "${featargs[@]}"
  run_suite "$lbl vs debug   .so" "$RS_DEBUG"   "${featargs[@]}"
done
run_suite "default features vs release .so" "$RS_RELEASE"
run_suite "default features vs debug   .so" "$RS_DEBUG"

# ---------------------------------------------------------------------------
# Negative control: a deliberately-wrong library MUST be rejected, otherwise the
# suite could be passing vacuously.
# ---------------------------------------------------------------------------
say "Negative control (mutant must FAIL)"
MUT=${TMPDIR:-/tmp}/harness_mutant
mkdir -p "$MUT"
cp c_src/include/lib.h "$MUT/"
sed 's/return a\.x \* b\.x + a\.y \* b\.y;/return a.y * b.y + a.x * b.x;/' \
  c_src/src/lib.c > "$MUT/m1.c"
if gcc -O0 -shared -fPIC -I "$MUT" -o "$MUT/libm1.so" "$MUT/m1.c" 2>/dev/null; then
  out=$(env C_SO_PATH="$ROOT/$C_SO" RUST_SO_PATH="$MUT/libm1.so" \
        timeout 600 cargo test "${CARGO_FLAGS[@]}" --release 2>&1)
  if echo "$out" | grep -q 'FAILED'; then
    ok "mutant (swapped dot-product add order) is detected: $(echo "$out" | grep -c 'FAILED\b') failing rows"
  else
    bad "mutant NOT detected — the differential suite is vacuous!"
  fi
else
  echo "  note: gcc unavailable, negative control skipped"
fi

say "Summary"
if [ "$FAILED" -eq 0 ]; then
  printf '\033[32mALL CHECKS PASSED\033[0m\n'
else
  printf '\033[31m%d CHECK(S) FAILED\033[0m\n' "$FAILED"
fi
exit $((FAILED > 0))
