#!/usr/bin/env bash
# Runs the complete verification matrix:
#   1. enumerates every valid feature combination from Cargo.toml,
#   2. `cargo check` for each combination,
#   3. builds the C artifacts (executable + shared library) from the pristine
#      c_src/ and the Rust artifacts (executable + cdylib),
#   4. diffs the exported symbols of the C .so against the Rust .so,
#   5. runs the whole differential test suite for every combination, in the
#      debug and the release profile.
#
# Usage: ./check_all.sh
set -uo pipefail

cd "$(dirname "$0")"
ROOT=$(pwd)
LOG=${TMPDIR:-/tmp}/check_all.$$
mkdir -p "$LOG"
FAILED=0

step() { printf '\n\033[1m=== %s ===\033[0m\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAILED=1; }

# ---------------------------------------------------------------------------
step "1. feature combinations declared in Cargo.toml"
# every key of the [features] table (excluding "default")
mapfile -t FEATURES < <(awk '
  /^\[features\]/ {inside=1; next}
  /^\[/ {inside=0}
  inside && /^[A-Za-z0-9_-]+[ \t]*=/ {split($0,a,"="); gsub(/[ \t]/,"",a[1]); if (a[1] != "default") print a[1]}
' Cargo.toml)
if [ ${#FEATURES[@]} -eq 0 ]; then
  echo "  no [features] table -> exactly one configuration (the empty/default one)"
else
  echo "  features: ${FEATURES[*]}"
fi

# power set of FEATURES (the empty set included)
COMBOS=("")
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  new=()
  for c in "${COMBOS[@]}"; do
    if [ -z "$c" ]; then new+=("$f"); else new+=("$c,$f"); fi
  done
  COMBOS+=("${new[@]}")
done
echo "  ${#COMBOS[@]} combination(s): $(printf '[%s] ' "${COMBOS[@]}")"

# ---------------------------------------------------------------------------
step "2. cargo check for every combination"
for c in "${COMBOS[@]}"; do
  label="--no-default-features --features '$c'"
  if [ -z "$c" ]; then
    args=(--no-default-features)
    label="--no-default-features"
  else
    args=(--no-default-features --features "$c")
  fi
  if cargo check --offline --all-targets "${args[@]}" >"$LOG/check.log" 2>&1; then
    ok "cargo check $label"
  else
    bad "cargo check $label"; tail -20 "$LOG/check.log"
  fi
done
for extra in "" "--all-features"; do
  if cargo check --offline --all-targets $extra >"$LOG/check.log" 2>&1; then
    ok "cargo check ${extra:-<default features>}"
  else
    bad "cargo check ${extra:-<default features>}"; tail -20 "$LOG/check.log"
  fi
done

# ---------------------------------------------------------------------------
step "3. build the C and the Rust artifacts"
mkdir -p c_src/build
(cd c_src/build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$LOG/cmake.log" 2>&1 \
  && cmake --build . >>"$LOG/cmake.log" 2>&1) \
  && ok "C executable  c_src/build/driver" || { bad "cmake build"; tail -20 "$LOG/cmake.log"; }
mkdir -p cbuild
gcc -shared -fPIC -O0 -Dmain=luggage_main -o cbuild/libluggage.so c_src/src/luggage.c \
  && ok "C shared lib  cbuild/libluggage.so" || bad "gcc -shared"
cargo build --offline >"$LOG/build.log" 2>&1 \
  && ok "Rust debug    target/debug/{driver,libdriver.so}" || { bad "cargo build"; tail -20 "$LOG/build.log"; }
cargo build --offline --release >"$LOG/build_rel.log" 2>&1 \
  && ok "Rust release  target/release/{driver,libdriver.so}" || { bad "cargo build --release"; tail -20 "$LOG/build_rel.log"; }

# ---------------------------------------------------------------------------
step "4. symbol parity (nm -D --defined-only)"
strip_noise() {
  nm -D --defined-only "$1" 2>/dev/null \
    | awk '$2=="T" || $2=="i" {print $3}' \
    | sed 's/@.*//' \
    | grep -v -E '^(_ITM_|__cxa|_Unwind|__gmon|_init$|_fini$|rust_eh_personality$)' \
    | sort -u
}
strip_noise cbuild/libluggage.so >"$LOG/c.syms"
strip_noise target/debug/libdriver.so >"$LOG/rust.syms"
echo "  C .so exports:    $(tr '\n' ' ' <"$LOG/c.syms")"
echo "  Rust .so exports: $(tr '\n' ' ' <"$LOG/rust.syms")"
MISSING=$(comm -23 "$LOG/c.syms" "$LOG/rust.syms")
if [ -z "$MISSING" ]; then
  ok "0 symbols missing from the Rust .so"
else
  bad "missing from the Rust .so: $(echo "$MISSING" | tr '\n' ' ')"
fi

# ---------------------------------------------------------------------------
step "5. differential test suite for every combination"
for c in "${COMBOS[@]}"; do
  if [ -z "$c" ]; then
    args=(--no-default-features); label="--no-default-features"
  else
    args=(--no-default-features --features "$c"); label="--features $c"
  fi
  export DIFF_TEST_FEATURES="$c"
  if timeout 900 cargo test --offline "${args[@]}" >"$LOG/test.log" 2>&1; then
    ok "cargo test $label   ($(grep -c '^test .* \.\.\. ok' "$LOG/test.log") tests)"
  else
    bad "cargo test $label"; grep -E "^(test .*FAILED|failures:|thread)" "$LOG/test.log" | head -20
  fi
done
unset DIFF_TEST_FEATURES
if timeout 900 cargo test --offline >"$LOG/test_def.log" 2>&1; then
  ok "cargo test <default features>   ($(grep -c '^test .* \.\.\. ok' "$LOG/test_def.log") tests)"
else
  bad "cargo test <default features>"; grep -E "^(test .*FAILED|failures:|thread)" "$LOG/test_def.log" | head -20
fi
if timeout 900 cargo test --offline --all-features >"$LOG/test_all.log" 2>&1; then
  ok "cargo test --all-features   ($(grep -c '^test .* \.\.\. ok' "$LOG/test_all.log") tests)"
else
  bad "cargo test --all-features"; grep -E "^(test .*FAILED|failures:|thread)" "$LOG/test_all.log" | head -20
fi
if timeout 900 cargo test --offline --release >"$LOG/test_rel.log" 2>&1; then
  ok "cargo test --release   ($(grep -c '^test .* \.\.\. ok' "$LOG/test_rel.log") tests)"
else
  bad "cargo test --release"; grep -E "^(test .*FAILED|failures:|thread)" "$LOG/test_rel.log" | head -20
fi

# ---------------------------------------------------------------------------
step "summary"
if [ $FAILED -eq 0 ]; then
  echo "  ALL CHECKS PASSED"
else
  echo "  THERE WERE FAILURES (logs in $LOG)"
fi
exit $FAILED
