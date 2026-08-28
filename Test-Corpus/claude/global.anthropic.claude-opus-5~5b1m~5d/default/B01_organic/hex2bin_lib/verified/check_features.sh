#!/usr/bin/env bash
# Phase D automation:
#   1. enumerate every feature combination declared in Cargo.toml
#   2. cargo check + full differential test suite for each
#   3. symbol-parity diff (nm -D) between the C .so and the Rust .so for each
set -uo pipefail
cd "$(dirname "$0")"

ROOT=$(cd .. && pwd)
C_SO=$(ls "$ROOT"/c_src/build/lib*.so 2>/dev/null | head -1)
if [ -z "${C_SO:-}" ]; then
  echo "C .so missing; building it"
  (cd "$ROOT/c_src" && mkdir -p build && cd build \
     && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
     && cmake --build . >/dev/null)
  C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
fi
LOG="${TMPDIR:-.}/ft.log"
echo "C  .so: $C_SO"

# --- 1. enumerate features from Cargo.toml -----------------------------------
mapfile -t FEATURES < <(python3 - <<'PY'
import re, pathlib
s = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M | re.S)
if not m:
    raise SystemExit(0)
for line in m.group(1).splitlines():
    line = line.split('#')[0].strip()
    if not line or '=' not in line:
        continue
    name = line.split('=')[0].strip()
    if name and name != 'default':
        print(name)
PY
)

echo "declared non-default features: ${#FEATURES[@]} ${FEATURES[*]:-(none)}"

# Build the list of configurations: default, --no-default-features, then every
# subset of the declared features (powerset) with --no-default-features.
CONFIGS=("" "--no-default-features")
n=${#FEATURES[@]}
if [ "$n" -gt 0 ]; then
  for ((mask = 1; mask < (1 << n); mask++)); do
    combo=""
    for ((i = 0; i < n; i++)); do
      if (((mask >> i) & 1)); then combo+="${FEATURES[i]},"; fi
    done
    CONFIGS+=("--no-default-features --features ${combo%,}")
    CONFIGS+=("--features ${combo%,}")
  done
fi

# --- 2 & 3. per-configuration check, test, symbol diff -----------------------
rc=0
for cfg in "${CONFIGS[@]}"; do
  label=${cfg:-"(default)"}
  echo
  echo "=============================================================="
  echo "CONFIG: $label"
  echo "=============================================================="

  if ! timeout 600 cargo check $cfg >/dev/null 2>&1; then
    echo "  cargo check : FAIL"; rc=1; continue
  fi
  echo "  cargo check : ok"

  # Build the cdylib for this configuration into an isolated target dir and
  # diff its exported symbols against the C .so.
  SO_DIR="target/featurecheck"
  if ! CARGO_TARGET_DIR="$SO_DIR" timeout 600 cargo build --release --lib $cfg >/dev/null 2>&1; then
    echo "  cargo build : FAIL"; rc=1; continue
  fi
  R_SO="$SO_DIR/release/libhex2bin_lib.so"
  echo "  cargo build : ok -> $R_SO"

  missing=$(comm -23 \
    <(nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u) \
    <(nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u))
  if [ -n "$missing" ]; then
    echo "  symbol diff : FAIL — exported by C but not by Rust:"
    echo "$missing" | sed 's/^/                /'
    rc=1
  else
    echo "  symbol diff : ok (0 missing)"
  fi

  # Undefined non-libc symbols in the Rust .so must be empty too.
  undef=$(nm -D -u "$R_SO" | awk '{print $2}' \
    | grep -v '^$' \
    | grep -vE '^(_ITM_|__cxa_|__gmon_|__tls_|_Unwind_|__rust_probestack)' \
    | grep -vE '@GLIBC|@GCC' | sort -u)
  if [ -n "$undef" ]; then
    echo "  undefined   : FAIL — non-libc undefined symbols:"; echo "$undef" | sed 's/^/                /'
    rc=1
  else
    echo "  undefined   : ok (only libc/toolchain imports)"
  fi

  # Full differential suite against THIS configuration's .so.
  if RUST_SO_PATH="$(pwd)/$R_SO" C_SO_PATH="$C_SO" \
       timeout 600 cargo test --tests $cfg >"$LOG" 2>&1; then
    echo "  diff tests  : ok ($(grep -c 'test result: ok' "$LOG") binaries)"
    grep 'test result:' "$LOG" | sed 's/^/                /'
  else
    echo "  diff tests  : FAIL"; tail -30 "$LOG" | sed 's/^/                /'; rc=1
  fi
done

echo
if [ $rc -eq 0 ]; then echo "ALL CONFIGURATIONS PASSED"; else echo "SOME CONFIGURATIONS FAILED"; fi
exit $rc
