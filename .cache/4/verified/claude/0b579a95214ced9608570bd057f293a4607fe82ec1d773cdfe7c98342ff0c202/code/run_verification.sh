#!/usr/bin/env bash
# Full C-vs-Rust differential verification, for EVERY Cargo feature combination.
#
#   ./run_verification.sh
#
# Phases:
#   A  build the C .so (default configuration) + enumerate feature combinations
#   D1 cargo check for every feature combination
#   B  valid-path differential tests   (tests/valid_paths.rs   <- CONFIGS.md)
#   C  error-path differential tests   (tests/error_paths.rs   <- ERRORS.md)
#   D2 exported-symbol parity          (tests/symbol_parity.rs <- SYMBOLS.md)
set -uo pipefail
cd "$(dirname "$0")"

fail=0
say() { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }

# ---------------------------------------------------------------- Phase A ----
say "Phase A: building the C shared library (default configuration)"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . ) || { echo "C build FAILED"; exit 1; }
C_SO=c_src/build/libtranslated_rust.so
test -f "$C_SO" || { echo "missing $C_SO"; exit 1; }
echo "C .so: $C_SO"

# Enumerate every valid feature combination = powerset of [features] in
# Cargo.toml. With no [features] section this yields exactly one combination
# (the empty one, i.e. --no-default-features == default).
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+ *=/ { sub(/ *=.*/, ""); print }
  ' Cargo.toml
)
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  for existing in "${COMBOS[@]}"; do
    COMBOS+=("${existing:+$existing,}$f")
  done
done
echo "features declared: ${#FEATURES[@]} -> ${#COMBOS[@]} combination(s)"
for c in "${COMBOS[@]}"; do echo "  * --no-default-features --features '${c}'"; done

# ------------------------------------------------------- Phases D1 / B / C ---
for combo in "${COMBOS[@]}"; do
  label="${combo:-<none>}"

  say "Phase D1: cargo check  (features: $label)"
  timeout 600 cargo check --no-default-features --features "$combo" \
    || { echo "cargo check FAILED for '$label'"; fail=1; continue; }
  timeout 600 cargo check --no-default-features --features "$combo" --tests \
    || { echo "cargo check --tests FAILED for '$label'"; fail=1; continue; }

  # Build both cdylib profiles for this feature set; the tests dlopen both.
  timeout 600 cargo build --lib --no-default-features --features "$combo" \
    || { echo "debug cdylib build FAILED for '$label'"; fail=1; continue; }
  timeout 600 cargo build --lib --release --no-default-features --features "$combo" \
    || { echo "release cdylib build FAILED for '$label'"; fail=1; continue; }

  say "Phases B + C + D2: differential tests  (features: $label)"
  timeout 600 cargo test --no-default-features --features "$combo" \
    || { echo "TESTS FAILED for '$label'"; fail=1; }
done

# ------------------------------------------------------------- Phase D2 ------
say "Phase D2: nm -D symbol diff (C .so -> Rust .so)"
for rs in target/release/libldexp_q2_lib.so target/debug/libldexp_q2_lib.so; do
  test -f "$rs" || continue
  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $NF}' | sort -u) \
    <(nm -D --defined-only "$rs"   | awk '{print $NF}' | sort -u))
  if [ -n "$missing" ]; then
    echo "MISSING from $rs:"; echo "$missing"; fail=1
  else
    echo "OK: $rs exports every symbol the C .so exports"
  fi
done

say "RESULT"
if [ "$fail" -eq 0 ]; then
  echo "ALL PHASES PASSED for every feature combination"
else
  echo "FAILURES DETECTED"
fi
exit "$fail"
