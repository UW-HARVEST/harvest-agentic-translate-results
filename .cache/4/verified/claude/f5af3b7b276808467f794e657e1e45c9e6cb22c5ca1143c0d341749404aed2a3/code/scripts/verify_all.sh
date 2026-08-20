#!/usr/bin/env bash
# Phase D driver: enumerate every Cargo feature combination, build + check + test
# each one, and diff the exported symbols of the C .so against the Rust .so.
#
# Usage: scripts/verify_all.sh
set -uo pipefail

cd "$(dirname "$0")/.."
ROOT="$PWD"
FAIL=0
note() { printf '\n=== %s ===\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; }
bad()  { printf '  [FAIL] %s\n' "$*"; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Enumerate feature combinations mechanically from Cargo.toml.
# ---------------------------------------------------------------------------
note "Enumerating feature combinations from Cargo.toml"
FEATURES=$(python3 - <<'PY'
import re, sys
src = open('Cargo.toml').read()
# grab the [features] table, if any
m = re.search(r'^\[features\]\s*$(.*?)(?=^\[|\Z)', src, re.M | re.S)
feats = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if not line or '=' not in line:
            continue
        name = line.split('=', 1)[0].strip().strip('"')
        if name and name != 'default':
            feats.append(name)
print('\n'.join(feats))
PY
)
FEAT_LIST=()
while IFS= read -r f; do [ -n "$f" ] && FEAT_LIST+=("$f"); done <<< "$FEATURES"
N=${#FEAT_LIST[@]}
echo "declared non-default features: ${N} ${FEAT_LIST[*]:-(none)}"

# Build the power set of feature names. With N=0 this yields exactly one
# combination: the empty one (== --no-default-features).
COMBOS=()
for ((mask = 0; mask < (1 << N); mask++)); do
  combo=""
  for ((i = 0; i < N; i++)); do
    if ((mask & (1 << i))); then combo="${combo:+$combo,}${FEAT_LIST[$i]}"; fi
  done
  COMBOS+=("$combo")
done
# Always also verify the crate's own default set.
COMBOS+=("__DEFAULT__")
echo "combinations to verify: ${#COMBOS[@]}"
for c in "${COMBOS[@]}"; do echo "  - ${c:-<empty>}"; done

# ---------------------------------------------------------------------------
# 2. Build the C shared library.
# ---------------------------------------------------------------------------
note "Building the C shared library"
mkdir -p c_src/build
if (cd c_src/build \
    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null 2>&1 \
    && cmake --build . >/dev/null 2>&1); then
  ok "C .so built"
else
  bad "C .so build failed"
  exit 1
fi
C_SO="$ROOT/c_src/build/libtranslated_rust.so"
[ -f "$C_SO" ] || { bad "missing $C_SO"; exit 1; }

# ---------------------------------------------------------------------------
# 3. Per-combination: cargo check, cargo build (cdylib), symbol diff, cargo test.
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  if [ "$combo" = "__DEFAULT__" ]; then
    label="<crate default>"
    FLAGS=()
  else
    label="--no-default-features --features '${combo}'"
    FLAGS=(--no-default-features)
    [ -n "$combo" ] && FLAGS+=(--features "$combo")
  fi

  note "Combination: $label"

  if timeout 600 cargo check "${FLAGS[@]}" >"${TMPDIR:-/tmp}/check.log" 2>&1; then
    ok "cargo check"
  else
    bad "cargo check"; tail -30 "${TMPDIR:-/tmp}/check.log"; continue
  fi

  # The cdylib MUST be rebuilt explicitly: `cargo test` does not always
  # consider the cdylib a dependency of the integration-test targets.
  if timeout 600 cargo build "${FLAGS[@]}" >"${TMPDIR:-/tmp}/build.log" 2>&1; then
    ok "cargo build (cdylib)"
  else
    bad "cargo build"; tail -30 "${TMPDIR:-/tmp}/build.log"; continue
  fi

  RUST_SO="$ROOT/target/debug/liboverunder_lib.so"
  if [ ! -f "$RUST_SO" ]; then bad "missing $RUST_SO"; continue; fi

  # --- symbol parity ---
  c_syms=$(nm -D --defined-only "$C_SO"   | awk '{print $3}' | sort -u)
  r_syms=$(nm -D --defined-only "$RUST_SO" | awk '{print $3}' | sort -u)
  missing=$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))
  if [ -z "$missing" ]; then
    ok "symbol parity: all $(echo "$c_syms" | grep -c . ) C symbols exported by Rust"
  else
    bad "symbols missing from the Rust .so:"; echo "$missing" | sed 's/^/        /'
  fi

  # --- undefined non-libc symbols in the Rust .so ---
  undef=$(nm -D --undefined-only "$RUST_SO" | awk '$1=="U"{print $2}' \
          | sed 's/@.*//' \
          | grep -vE '^(_Unwind_|__errno_location|__tls_get_addr|__cxa_|__gmon_start__|_ITM_)' \
          | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|printf|putchar|pthread_[a-z_]+|read|readlink|realloc|realpath|sqrt|stat|stat64|statx|strlen|strncpy|syscall|write|writev)$')
  if [ -z "$undef" ]; then
    ok "no missing/undefined non-libc symbols"
  else
    bad "undefined non-libc symbols in the Rust .so:"; echo "$undef" | sed 's/^/        /'
  fi

  # --- Phase B + Phase C differential tests ---
  for t in phase_b_configs phase_c_errors phase_overunder; do
    if timeout 600 cargo test "${FLAGS[@]}" --test "$t" >"${TMPDIR:-/tmp}/test_$t.log" 2>&1; then
      res=$(grep -E '^test result:' "${TMPDIR:-/tmp}/test_$t.log" | tail -1)
      ok "cargo test --test $t  ($res)"
    else
      bad "cargo test --test $t"
      tail -40 "${TMPDIR:-/tmp}/test_$t.log"
    fi
  done
done

# ---------------------------------------------------------------------------
# 4. Robustness matrix: the C source relies on signed-overflow wraparound, which
#    is UB and could in principle be optimised differently. Cross-check the C at
#    -O0 (the CMake default), -O2 and -O3 against BOTH Rust profiles (the release
#    profile additionally sets panic = "abort"). Built out-of-tree so nothing in
#    c_src/ is modified.
# ---------------------------------------------------------------------------
note "Robustness matrix: C -O0/-O2/-O3  x  Rust debug/release"
TMP="${TMPDIR:-/tmp}"
timeout 600 cargo build --release >"$TMP/build_rel.log" 2>&1 \
  && ok "cargo build --release" || { bad "cargo build --release"; tail -20 "$TMP/build_rel.log"; }

for opt in O0 O2 O3; do
  d="$TMP/cbuild_$opt"; rm -rf "$d"; mkdir -p "$d"
  if ( cd "$d" && cmake "$ROOT/c_src" -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_C_FLAGS="-$opt" >/dev/null 2>&1 && cmake --build . >/dev/null 2>&1 ); then
    :
  else
    bad "C build at -$opt"; continue
  fi
  for rprof in debug release; do
    rso="$ROOT/target/$rprof/liboverunder_lib.so"
    [ -f "$rso" ] || { bad "missing $rso"; continue; }
    for t in phase_b_configs phase_c_errors phase_overunder; do
      log="$TMP/mx_${opt}_${rprof}_$t.log"
      if HARVEST_C_SO="$d/libtranslated_rust.so" HARVEST_RUST_SO="$rso" \
         timeout 600 cargo test --test "$t" >"$log" 2>&1; then
        ok "C=-$opt Rust=$rprof $t  ($(grep -E '^test result:' "$log" | tail -1))"
      else
        bad "C=-$opt Rust=$rprof $t"; tail -30 "$log"
      fi
    done
  done
done

note "RESULT"
if [ "$FAIL" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "THERE WERE FAILURES"
fi
exit "$FAIL"
