#!/usr/bin/env bash
# Full verification of the Rust translation against the C reference.
#
#   1. build the C shared library (default CMake configuration)
#   2. enumerate every Cargo feature combination and `cargo check` each one
#   3. build the Rust cdylib and diff the exported symbol tables
#   4. run the differential test suite for every feature combination
#
# Run from the `translation/` directory:  ./verify.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE="$ROOT/translation"
LOG=/tmp/verify-translation.log
: > "$LOG"

step() { printf '\n=== %s ===\n' "$1"; }

# ---------------------------------------------------------------- 1. C library
step "Building C reference (default configuration)"
mkdir -p "$ROOT/c_src/build"
(
  cd "$ROOT/c_src/build"
  timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON
  timeout 600 cmake --build .
) >>"$LOG" 2>&1
C_SO="$(find "$ROOT/c_src/build" -maxdepth 1 -name '*.so' | sort | head -1)"
[ -n "$C_SO" ] || { echo "no C .so produced; see $LOG"; exit 1; }
echo "C  .so: $C_SO"

# --------------------------------------------- 2. enumerate feature combinations
step "Enumerating feature combinations from Cargo.toml"
FEATURES=$(python3 - "$CRATE/Cargo.toml" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', text, re.S | re.M)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        key = line.split('=', 1)[0].strip().strip('"')
        if key != 'default':
            names.append(key)
print(' '.join(names))
PY
)

COMBOS=()
if [ -z "$FEATURES" ]; then
  echo "no [features] table -> a single configuration"
  COMBOS=("")
else
  # every subset of the feature set
  read -r -a NAMES <<<"$FEATURES"
  n=${#NAMES[@]}
  for ((mask = 0; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if ((mask & (1 << i))); then combo="${combo:+$combo,}${NAMES[i]}"; fi
    done
    COMBOS+=("$combo")
  done
  printf 'features: %s\n' "$FEATURES"
  printf '%s combinations\n' "${#COMBOS[@]}"
fi

# --------------------------------------------------------------- 3. cargo check
step "cargo check for every combination"
cd "$CRATE"
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then
    label="<default>"
    args=()
  else
    label="$combo"
    args=(--no-default-features --features "$combo")
  fi
  printf '  check %-40s' "$label"
  if timeout 600 cargo check "${args[@]}" >>"$LOG" 2>&1; then echo ok; else
    echo FAILED
    tail -40 "$LOG"
    exit 1
  fi
done

# ------------------------------------------------------------- 4. symbol parity
step "Comparing exported symbols"
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then args=(); else args=(--no-default-features --features "$combo"); fi
  timeout 600 cargo build --release "${args[@]}" >>"$LOG" 2>&1
  R_SO="$CRATE/target/release/libomni_manifold_lib.so"
  nm -D --defined-only "$C_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort -u >/tmp/verify-c-syms.txt
  nm -D --defined-only "$R_SO" | awk '$2=="T"||$2=="W"{print $3}' | sort -u >/tmp/verify-r-syms.txt
  missing=$(comm -23 /tmp/verify-c-syms.txt /tmp/verify-r-syms.txt || true)
  if [ -n "$missing" ]; then
    echo "  ${combo:-<default>}: Rust .so is missing:"
    echo "$missing" | sed 's/^/    /'
    exit 1
  fi
  printf '  %-40s ok (%s symbols)\n' "${combo:-<default>}" "$(wc -l </tmp/verify-c-syms.txt)"
done

# ------------------------------------------------------------------- 5. testing
step "Running differential tests for every combination"
for combo in "${COMBOS[@]}"; do
  if [ -z "$combo" ]; then args=(); else args=(--no-default-features --features "$combo"); fi
  printf '  test %-40s' "${combo:-<default>}"
  if timeout 600 cargo test --release "${args[@]}" >>"$LOG" 2>&1; then echo ok; else
    echo FAILED
    grep -E "^test |panicked|mismatch" "$LOG" | tail -40
    exit 1
  fi
done

step "All configurations verified"
grep -c "^test .* ok$" "$LOG" | sed 's/^/passing test cases logged: /'
echo "full log: $LOG"
