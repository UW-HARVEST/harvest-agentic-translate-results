#!/usr/bin/env bash
# Phase D driver: build the C .so, enumerate every cargo feature combination,
# and for each one rebuild the Rust cdylib, diff `nm -D` symbol sets, and run the
# whole differential suite.
#
# `cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact, so the
# `cargo build` before each `cargo test` is load-bearing, not cosmetic.
set -uo pipefail
cd "$(dirname "$0")"
ROOT="$(cd .. && pwd)"
fail=0

# ---------------------------------------------------------------- C .so
if ! ls "$ROOT"/c_src/build/lib*.so >/dev/null 2>&1; then
  echo "== building the C shared library =="
  (cd "$ROOT/c_src" && mkdir -p build && cd build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
    && cmake --build . >/dev/null) || { echo "C build FAILED"; exit 1; }
fi
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)
echo "C   .so: $C_SO"

# --------------------------------------------------- feature combinations
# Enumerate the powerset of the features declared in Cargo.toml. With no
# [features] table this yields exactly one (empty) combination -- which is the
# fact we want established mechanically rather than assumed.
mapfile -t FEATURES < <(python3 - <<'PY'
import re, pathlib
txt = pathlib.Path("Cargo.toml").read_text()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', txt, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#')[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=')[0].strip().strip('"')
        if name != 'default':
            names.append(name)
print('\n'.join(names))
PY
)
# drop empty entries
COMBOS=()
declare -a FEATS=()
for f in "${FEATURES[@]}"; do [ -n "$f" ] && FEATS+=("$f"); done
n=${#FEATS[@]}
echo "declared non-default features: ${n} ${FEATS[*]:-(none)}"
if [ "$n" -eq 0 ]; then
  COMBOS=("__default__")
else
  COMBOS=("__default__")
  for ((mask=0; mask<(1<<n); mask++)); do
    combo=""
    for ((i=0; i<n; i++)); do
      if (( mask & (1<<i) )); then combo="${combo:+$combo,}${FEATS[$i]}"; fi
    done
    COMBOS+=("$combo")
  done
fi

c_syms=$(nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TWi]$/ {print $3}' | sort -u)

for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__default__" ]; then
    label="default features"
    flags=()
  elif [ -z "$combo" ]; then
    label="--no-default-features (no features)"
    flags=(--no-default-features)
  else
    label="--no-default-features --features $combo"
    flags=(--no-default-features --features "$combo")
  fi
  echo
  echo "=============================================================="
  echo "== combination: $label"
  echo "=============================================================="

  if ! timeout 600 cargo build -q "${flags[@]}" 2>&1 | tail -20; then
    echo "  BUILD FAILED"; fail=1; continue
  fi
  R_SO="target/debug/libhex2bin_lib.so"

  # ---- symbol parity
  r_syms=$(nm -D --defined-only "$R_SO" | awk '$2 ~ /^[TWi]$/ {print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -n "$missing" ]; then
    echo "  MISSING SYMBOLS in the Rust .so:"; echo "$missing" | sed 's/^/    /'; fail=1
  else
    echo "  symbol parity: OK ($(echo "$c_syms" | wc -l) C symbol(s), 0 missing)"
  fi
  undef=$(nm -D --undefined-only "$R_SO" | awk '{print $NF}' \
    | grep -v '@GLIBC\|@GCC\|^_ITM_\|^__gmon_start__\|^__cxa_\|^statx$\|^gettid$' || true)
  if [ -n "$undef" ]; then
    echo "  UNRESOLVED non-libc symbols:"; echo "$undef" | sed 's/^/    /'; fail=1
  else
    echo "  undefined non-libc symbols: 0"
  fi

  # ---- differential suite
  if timeout 600 cargo test -q "${flags[@]}" >/tmp/run_all.log 2>&1; then
    echo "  differential suite: PASS"
    grep -E 'test result' /tmp/run_all.log | sed 's/^/    /'
  else
    echo "  differential suite: FAIL"; tail -40 /tmp/run_all.log | sed 's/^/    /'; fail=1
  fi
done

# ---------------------------------------------------- release profile too
echo
echo "== release profile (panic = \"abort\", optimised) =="
timeout 600 cargo build -q --release || { echo "release build FAILED"; fail=1; }
if [ -f target/release/libhex2bin_lib.so ]; then
  r_syms=$(nm -D --defined-only target/release/libhex2bin_lib.so | awk '$2 ~ /^[TWi]$/ {print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  [ -z "$missing" ] && echo "  symbol parity: OK" \
                    || { echo "  MISSING: $missing"; fail=1; }
  if HARVEST_RUST_SO="$PWD/target/release/libhex2bin_lib.so" \
     timeout 600 cargo test -q >/tmp/run_all_rel.log 2>&1; then
    echo "  differential suite against the RELEASE .so: PASS"
    grep -E 'test result' /tmp/run_all_rel.log | sed 's/^/    /'
  else
    echo "  differential suite against the RELEASE .so: FAIL"
    tail -40 /tmp/run_all_rel.log | sed 's/^/    /'; fail=1
  fi
fi

# ------------------------------------------- C at every optimisation level
# The C's constant-time bit tricks rely on integer promotion rules; confirm the
# ground truth is stable across -O levels and that the Rust matches all of them.
# Alternate builds go to /tmp so nothing under c_src/ is touched.
echo
echo "== Rust vs. the C built at every optimisation level =="
timeout 600 cargo build -q
for opt in -O0 -O1 -O2 -O3 -Os; do
  bd="/tmp/cbuild$opt"
  rm -rf "$bd"; mkdir -p "$bd"
  if ! (cd "$bd" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="$opt" >/dev/null 2>&1 && cmake --build . >/dev/null 2>&1); then
    echo "  C $opt: BUILD FAILED"; fail=1; continue
  fi
  so=$(ls "$bd"/lib*.so | head -1)
  if HARVEST_C_SO="$so" timeout 600 cargo test -q >/tmp/copt.log 2>&1; then
    echo "  C $opt: PASS"
  else
    echo "  C $opt: FAIL"; grep -m3 DIVERGENCE /tmp/copt.log | sed 's/^/    /'; fail=1
  fi
done

# --------------------------------------------------------- binaries?
echo
if ls target/debug/*.d >/dev/null 2>&1 && \
   python3 -c "import re,pathlib,sys; t=pathlib.Path('Cargo.toml').read_text(); sys.exit(0 if '[[bin]]' in t else 1)"; then
  echo "== NOTE: crate declares a [[bin]] -- stdout comparison required =="
  fail=1
else
  echo "== no binary target in Cargo.toml and no add_executable in CMakeLists.txt:"
  echo "   the 'compare C and Rust stdout' clause does not apply =="
  grep -c add_executable "$ROOT/c_src/CMakeLists.txt" >/dev/null 2>&1 \
    && { echo "   (but CMakeLists has add_executable!)"; fail=1; } || true
fi

echo
if [ "$fail" -ne 0 ]; then echo "RESULT: FAILURES PRESENT"; exit 1; fi
echo "RESULT: ALL COMBINATIONS PASS"
