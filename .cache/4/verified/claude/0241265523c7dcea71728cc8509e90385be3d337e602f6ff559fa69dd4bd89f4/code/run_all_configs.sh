#!/usr/bin/env bash
# Phase D: run `cargo check` and the full differential suite under EVERY valid
# build configuration.
#
# The feature list is read out of Cargo.toml rather than hard-coded, so a new
# feature cannot silently escape the sweep.  `c_src/CMakeLists.txt` defines no
# build options and `c_src/src/main.c` contains no preprocessor conditionals, so
# the C side has exactly one configuration; the Rust side declares
# `[features] default = []`, i.e. the power set of the (empty) optional-feature
# list is a single, empty combination.  It is still exercised three ways
# (default / --no-default-features / --all-features) plus the release profile,
# because those select different cargo code paths.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT" || exit 1

CARGO_FLAGS="--offline"

# Sandboxes commonly make /tmp read-only, so keep run logs inside the crate.
LOGDIR="${TMPDIR:-$ROOT/logs}/configruns"
mkdir -p "$LOGDIR" 2>/dev/null || LOGDIR="$ROOT/logs"
mkdir -p "$LOGDIR"

echo "=== enumerating features from Cargo.toml ==="
FEATURES=$(python3 - <<'PY'
import re, pathlib
text = pathlib.Path("Cargo.toml").read_text()
m = re.search(r"^\[features\]\s*$(.*?)(?=^\[|\Z)", text, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split("#", 1)[0].strip()
        if not line or "=" not in line:
            continue
        name = line.split("=", 1)[0].strip()
        if name != "default":
            names.append(name)
print(" ".join(names))
PY
)
if [ -z "$FEATURES" ]; then
  echo "optional features: (none) -> the only feature combination is the empty set"
else
  echo "optional features: $FEATURES"
fi

# Build the list of combinations: the power set of the optional features, each
# also run with --no-default-features.
COMBOS=()
COMBOS+=("")                       # default features
COMBOS+=("--no-default-features")  # explicitly empty
COMBOS+=("--all-features")         # every feature
if [ -n "$FEATURES" ]; then
  read -r -a arr <<<"$FEATURES"
  n=${#arr[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    sel=()
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then sel+=("${arr[$i]}"); fi
    done
    joined=$(
      IFS=,
      echo "${sel[*]}"
    )
    COMBOS+=("--no-default-features --features $joined")
    COMBOS+=("--features $joined")
  done
fi

echo
echo "=== ${#COMBOS[@]} configuration(s) to verify ==="
for c in "${COMBOS[@]}"; do echo "  cargo ... ${c:-<default>}"; done

failures=0

run() {
  local label="$1"
  shift
  echo
  echo "--- $label ---"
  if timeout 600 "$@" >"$LOGDIR/cfgrun.$$" 2>&1; then
    tail -n 4 "$LOGDIR/cfgrun.$$"
    echo "OK: $label"
  else
    echo "FAILED: $label"
    tail -n 60 "$LOGDIR/cfgrun.$$"
    failures=$((failures + 1))
  fi
  rm -f "$LOGDIR/cfgrun.$$"
}

for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2086
  run "cargo check --all-targets ${combo:-<default>}" \
    cargo check $CARGO_FLAGS --all-targets $combo
done

for combo in "${COMBOS[@]}"; do
  # shellcheck disable=SC2086
  run "cargo build --lib --bins ${combo:-<default>}" \
    cargo build $CARGO_FLAGS --lib --bins $combo
  # shellcheck disable=SC2086
  run "cargo test ${combo:-<default>}" \
    cargo test $CARGO_FLAGS $combo
done

echo
echo "=== release profile (panic = \"abort\") ==="
run "cargo build --release --lib --bins" cargo build $CARGO_FLAGS --release --lib --bins
run "cargo test --release" cargo test $CARGO_FLAGS --release

echo
echo "=== symbol parity: nm -D on both shared objects ==="
mkdir -p "$ROOT/cbuild"
gcc -shared -fPIC -o "$ROOT/cbuild/libcdriver.so" "$ROOT/c_src/src/main.c" 2>/dev/null
C_SYMS=$(nm -D --defined-only "$ROOT/cbuild/libcdriver.so" | awk '{print $3}' | sort -u)
R_SYMS=$(nm -D --defined-only "$ROOT/target/debug/libdriver.so" | awk '{print $3}' | sort -u)
echo "C  defines: $(echo "$C_SYMS" | tr '\n' ' ')"
MISSING=$(comm -23 <(echo "$C_SYMS") <(echo "$R_SYMS"))
if [ -n "$MISSING" ]; then
  echo "MISSING from the Rust cdylib: $MISSING"
  failures=$((failures + 1))
else
  echo "MISSING from the Rust cdylib: (none)"
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "=== ALL CONFIGURATIONS PASSED ==="
else
  echo "=== $failures FAILURE(S) ==="
fi
exit "$failures"
