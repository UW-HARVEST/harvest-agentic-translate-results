#!/usr/bin/env bash
# Full verification driver: symbol parity, every feature combination, and every
# phase's test suite. Long overflow-region tests get one cargo invocation each so
# no single command exceeds the 600 s budget.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
FAIL=0

hdr() { printf '\n===== %s =====\n' "$*"; }
ok()  { printf 'PASS  %s\n' "$*"; }
bad() { printf 'FAIL  %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
hdr "Build C shared library"
( cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null ) && ok "C .so built" || bad "C build"
C_SO="$ROOT/c_src/build/libdriver.so"

hdr "Build Rust cdylib"
( cd "$CRATE" && timeout 600 cargo build --release >/dev/null 2>&1 ) \
  && ok "Rust .so built" || bad "Rust build"
R_SO="$CRATE/target/release/libdriver.so"

# ---------------------------------------------------------------------------
hdr "Executable targets (binary stdout comparison applicability)"
C_EXES=$(grep -c 'add_executable' "$ROOT/c_src/CMakeLists.txt" || true)
R_BINS=$(grep -c '^\[\[bin\]\]' "$CRATE/Cargo.toml" || true)
R_BINDIR=$(ls "$CRATE/src/bin" 2>/dev/null | wc -l)
R_MAIN=$([ -f "$CRATE/src/main.rs" ] && echo 1 || echo 0)
echo "c_src add_executable: $C_EXES   rust [[bin]]: $R_BINS   src/bin entries: $R_BINDIR   src/main.rs: $R_MAIN"
if [ "$C_EXES" -eq 0 ] && [ "$R_BINS" -eq 0 ] && [ "$R_BINDIR" -eq 0 ] && [ "$R_MAIN" -eq 0 ]; then
  ok "no binary in either tree -> stdout-comparison item is N/A"
else
  bad "a binary exists; the C/Rust stdout comparison must be implemented"
fi

# ---------------------------------------------------------------------------
hdr "Symbol parity (nm -D)"
c_syms=$(nm -D --defined-only --format=posix "$C_SO" | awk '$2 ~ /^[TDBRWVGSiu]$/ {print $1}' | sort -u)
r_syms=$(nm -D --defined-only --format=posix "$R_SO" | awk '$2 ~ /^[TDBRWVGSiu]$/ {print $1}' | sort -u)
missing=$(comm -23 <(printf '%s\n' "$c_syms") <(printf '%s\n' "$r_syms"))
echo "C exports:    $(printf '%s\n' "$c_syms" | grep -c . )"
echo "Rust exports: $(printf '%s\n' "$r_syms" | grep -c . )"
if [ -z "$missing" ]; then ok "0 symbols missing from the Rust .so"
else bad "missing symbols:"; printf '%s\n' "$missing"; fi

if ldd -r "$R_SO" 2>&1 | grep -q 'undefined symbol'; then
  bad "ldd -r found unresolved symbols in the Rust .so"
else
  ok "ldd -r: no unresolved symbols in the Rust .so"
fi

# ---------------------------------------------------------------------------
hdr "Feature combinations"
# Mechanically enumerate every feature declared in Cargo.toml.
FEATURES=$(awk '
  /^\[features\]/ {inf=1; next}
  /^\[/           {inf=0}
  inf && /=/      {gsub(/[[:space:]]/,""); split($0,a,"="); if (a[1] != "default") print a[1]}
' "$CRATE/Cargo.toml")
NFEAT=$(printf '%s\n' "$FEATURES" | grep -c . || true)
echo "declared non-default features: ${NFEAT:-0}  [${FEATURES:-none}]"

COMBOS=()
if [ "${NFEAT:-0}" -eq 0 ]; then
  # No [features] section at all -> exactly one configuration exists.
  COMBOS+=("__default__")
else
  COMBOS+=("__default__" "__none__")
  # Full power set of the declared features.
  arr=($FEATURES)
  n=${#arr[@]}
  for ((mask=1; mask<(1<<n); mask++)); do
    combo=""
    for ((b=0; b<n; b++)); do
      if (( mask & (1<<b) )); then combo="${combo:+$combo,}${arr[b]}"; fi
    done
    COMBOS+=("$combo")
  done
fi
echo "combinations to verify: ${#COMBOS[@]}"

FAST_ARGS=(--release --test smoke --test symbols --test valid_paths --test error_paths)

for combo in "${COMBOS[@]}"; do
  case "$combo" in
    __default__) flags=() ; label="default" ;;
    __none__)    flags=(--no-default-features) ; label="no-default-features" ;;
    *)           flags=(--no-default-features --features "$combo") ; label="features=$combo" ;;
  esac
  hdr "cargo check [$label]"
  ( cd "$CRATE" && timeout 600 cargo check "${flags[@]}" >/dev/null 2>&1 ) \
    && ok "check $label" || bad "check $label"

  hdr "Phases B+C+D fast suites [$label]"
  ( cd "$CRATE" && timeout 600 cargo test "${FAST_ARGS[@]}" "${flags[@]}" -- --test-threads=1 2>&1 \
      | tail -n 30 )
  if [ "${PIPESTATUS[0]}" -eq 0 ]; then ok "fast suites $label"; else bad "fast suites $label"; fi
done

# ---------------------------------------------------------------------------
hdr "Long overflow-region tests (CONFIGS C9/C13, ERRORS E9)"
if [ "${SKIP_LONG:-0}" = "1" ]; then
  echo "SKIP_LONG=1 -> skipping (run without it to execute the ~14 min suite)"
else
for t in c9a c9b c9c c13; do
  ( cd "$CRATE" && timeout 600 cargo test --release --test overflow -- \
      --ignored --nocapture --test-threads=1 "$t" 2>&1 | tail -n 8 )
  if [ "${PIPESTATUS[0]}" -eq 0 ]; then ok "overflow $t"; else bad "overflow $t"; fi
done
fi

# ---------------------------------------------------------------------------
hdr "Result"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit "$FAIL"
