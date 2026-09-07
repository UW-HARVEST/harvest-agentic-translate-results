#!/usr/bin/env bash
# Phase D driver: builds both libraries, diffs their dynamic symbol tables, then
# runs the full Phase B + Phase C differential suites under EVERY feature
# combination and both cargo profiles. Exits non-zero on the first failure.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(dirname "$HERE")"
C_SO="$ROOT/c_src/build/libpow.so"
FAIL=0

step() { printf '\n=========== %s ===========\n' "$*"; }
fail() { printf 'FAIL: %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
step "Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) >/dev/null 2>&1 \
  || { fail "C build"; exit 1; }
[ -f "$C_SO" ] || { fail "missing $C_SO"; exit 1; }
echo "ok: $C_SO"

# ---------------------------------------------------------------------------
step "Enumerate feature combinations"
# ---------------------------------------------------------------------------
# Extract every feature name from the [features] table (if any) and build the
# power set of combinations to test.
FEATURES=$(awk '
  /^\[features\]/       { inf=1; next }
  /^\[/                 { inf=0 }
  inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ {
      split($0, a, "="); gsub(/[[:space:]]/, "", a[1]);
      if (a[1] != "default") print a[1]
  }' "$HERE/Cargo.toml")

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "No [features] table in Cargo.toml -> the only configuration is the default one."
  COMBOS+=("DEFAULT")
  COMBOS+=("NO_DEFAULT")
else
  echo "features: $(echo "$FEATURES" | tr '\n' ' ')"
  COMBOS+=("DEFAULT")
  COMBOS+=("NO_DEFAULT")
  mapfile -t FARR <<<"$FEATURES"
  n=${#FARR[@]}
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (( mask & (1 << i) )); then
        combo="${combo:+$combo,}${FARR[$i]}"
      fi
    done
    COMBOS+=("FEAT:$combo")
  done
fi
printf 'combinations to verify: %s\n' "${COMBOS[*]}"

# ---------------------------------------------------------------------------
run_combo() {
  local combo="$1" profile="$2"
  local -a flags=()
  case "$combo" in
    DEFAULT)    ;;
    NO_DEFAULT) flags+=(--no-default-features) ;;
    FEAT:*)     flags+=(--no-default-features --features "${combo#FEAT:}") ;;
  esac
  [ "$profile" = release ] && flags+=(--release)

  step "combo=$combo profile=$profile"

  cargo build --offline "${flags[@]}" >/dev/null 2>&1 \
    || { fail "cargo build ($combo/$profile)"; return 1; }

  local rust_so="$HERE/target/$profile/libpow.so"
  [ -f "$rust_so" ] || { fail "missing $rust_so ($combo/$profile)"; return 1; }

  # ---- symbol parity ----
  local c_syms rust_syms missing
  c_syms=$(nm -D --defined-only "$C_SO"      | awk '{print $NF}' | sort -u)
  rust_syms=$(nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$rust_syms"))
  if [ -n "$missing" ]; then
    fail "symbols exported by C but MISSING from Rust ($combo/$profile):"
    echo "$missing" | sed 's/^/    /'
  else
    echo "symbol parity: OK ($(echo "$c_syms" | wc -l) C symbol(s), all present in Rust)"
  fi

  # ---- undefined non-libc symbols in the Rust .so ----
  local undef
  undef=$(nm -D --undefined-only "$rust_so" | awk '{print $NF}' \
          | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^statx$|^gettid$' || true)
  if [ -n "$undef" ]; then
    fail "unresolved non-libc symbols in Rust .so ($combo/$profile):"
    echo "$undef" | sed 's/^/    /'
  else
    echo "undefined non-libc symbols: none"
  fi

  # ---- differential test suites (Phase B + Phase C) ----
  local log="$HERE/target/verify-$combo-$profile.log"
  log="${log//:/_}"
  RUST_POW_SO="$rust_so" timeout 600 cargo test --offline "${flags[@]}" >"$log" 2>&1
  local rc=$?
  grep -E '^\s+Running|^test result' "$log" | sed 's/^/    /'
  local total
  total=$(grep -c '^test .* \.\.\. ok$' "$log")
  echo "    -> $total individual differential tests passed"
  if [ "$rc" -ne 0 ]; then
    fail "differential tests ($combo/$profile) — see $log"
    grep -E 'FAILED|panicked|^failures:' -A5 "$log" | head -n 40 | sed 's/^/    /'
  fi
}

for combo in "${COMBOS[@]}"; do
  for profile in debug release; do
    run_combo "$combo" "$profile"
  done
done

# ---------------------------------------------------------------------------
step "Binary/driver stdout comparison"
# ---------------------------------------------------------------------------
if grep -q 'add_executable' "$ROOT/c_src/CMakeLists.txt" 2>/dev/null \
   || grep -q '^\[\[bin\]\]' "$HERE/Cargo.toml" \
   || [ -f "$HERE/src/main.rs" ]; then
  fail "a binary target exists but this script does not compare its stdout"
else
  echo "N/A: c_src/CMakeLists.txt builds only 'add_library(pow SHARED ...)';"
  echo "     translation/Cargo.toml declares only a cdylib (no [[bin]], no src/main.rs)."
fi

step "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAIL"
