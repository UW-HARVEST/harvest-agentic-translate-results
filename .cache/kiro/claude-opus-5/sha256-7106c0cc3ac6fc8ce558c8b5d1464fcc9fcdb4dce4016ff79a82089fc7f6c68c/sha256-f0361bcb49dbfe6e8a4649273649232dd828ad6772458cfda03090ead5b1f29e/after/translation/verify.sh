#!/usr/bin/env bash
# Phase D driver: symbol parity + every feature combination.
# Usage: ./verify.sh            (from the translation/ directory)
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
C_BUILD="$ROOT/c_src/build"
RUST_SO="$CRATE_DIR/target/release/libfallcalc_lib.so"
FAIL=0

section() { printf '\n=== %s ===\n' "$1"; }

# ---------------------------------------------------------------------------
section "Building C shared library"
mkdir -p "$C_BUILD"
( cd "$C_BUILD" \
  && timeout 600 cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && timeout 600 cmake --build . >/dev/null ) || { echo "C build FAILED"; exit 1; }
C_SO="$(find "$C_BUILD" -maxdepth 1 -name '*.so' | sort | head -1)"
echo "C   .so: $C_SO"

# ---------------------------------------------------------------------------
section "Enumerating feature combinations from Cargo.toml"
# Mechanically derive the feature list; with no [features] table the only
# configuration is the default one.
mapfile -t FEATURES < <(
  awk '
    /^\[features\]/ { inf=1; next }
    /^\[/           { inf=0 }
    inf && /^[A-Za-z0-9_-]+[[:space:]]*=/ { sub(/[[:space:]]*=.*/, ""); print }
  ' "$CRATE_DIR/Cargo.toml"
)
NONDEFAULT=()
for f in "${FEATURES[@]:-}"; do
  [ -z "$f" ] && continue
  [ "$f" = "default" ] && continue
  NONDEFAULT+=("$f")
done
echo "declared features: ${FEATURES[*]:-<none>}"

# Build the list of combos to test: default build, --no-default-features,
# and (if any non-default features exist) the full power set.
COMBOS=("--" "--no-default-features")
n=${#NONDEFAULT[@]}
if [ "$n" -gt 0 ]; then
  total=$(( 1 << n ))
  for (( mask=0; mask<total; mask++ )); do
    sel=""
    for (( i=0; i<n; i++ )); do
      if (( (mask >> i) & 1 )); then
        sel="${sel:+$sel,}${NONDEFAULT[$i]}"
      fi
    done
    if [ -n "$sel" ]; then
      COMBOS+=("--no-default-features --features $sel")
      COMBOS+=("--features $sel")
    fi
  done
fi
printf 'combinations to verify: %d\n' "${#COMBOS[@]}"
for cb in "${COMBOS[@]}"; do echo "  cargo test $cb"; done

# ---------------------------------------------------------------------------
for cb in "${COMBOS[@]}"; do
  flags="${cb/#--$/}"
  [ "$cb" = "--" ] && flags=""
  section "Combination: cargo [check|build|test] $flags"

  # shellcheck disable=SC2086
  ( cd "$CRATE_DIR" && timeout 600 cargo check $flags ) >/tmp/vc.log 2>&1 \
    || { echo "cargo check FAILED"; tail -20 /tmp/vc.log; FAIL=1; continue; }

  # shellcheck disable=SC2086
  ( cd "$CRATE_DIR" && timeout 600 cargo build --release $flags ) >/tmp/vb.log 2>&1 \
    || { echo "cargo build FAILED"; tail -20 /tmp/vb.log; FAIL=1; continue; }

  # ---- symbol parity ----
  nm -D --defined-only "$C_SO"   | awk '{print $3}' | grep -v '^$' | sort -u > /tmp/c_syms.txt
  nm -D --defined-only "$RUST_SO" | awk '{print $3}' | grep -v '^$' | sort -u > /tmp/r_syms.txt
  missing="$(comm -23 /tmp/c_syms.txt /tmp/r_syms.txt)"
  extra="$(comm -13 /tmp/c_syms.txt /tmp/r_syms.txt)"
  echo "C exports:    $(wc -l < /tmp/c_syms.txt)"
  echo "Rust exports: $(wc -l < /tmp/r_syms.txt)"
  if [ -n "$missing" ]; then
    echo "MISSING from Rust .so:"; echo "$missing" | sed 's/^/  /'; FAIL=1
  else
    echo "missing symbols: 0  [OK]"
  fi
  [ -n "$extra" ] && { echo "extra in Rust .so (informational):"; echo "$extra" | sed 's/^/  /'; }

  # ---- undefined non-libc symbols ----
  undef="$(nm -D -u "$RUST_SO" | awk '$1=="U"{print $2}' | sed 's/@.*//' \
    | grep -vE '^(_Unwind_|__)' \
    | grep -vxE 'malloc|free|calloc|realloc|posix_memalign|memcpy|memmove|memset|bcmp|strlen|abort|getenv|getcwd|readlink|realpath|open64|close|read|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|dl_iterate_phdr|syscall|gettid|pthread_key_create|pthread_key_delete|pthread_setspecific|pthread_getspecific')"
  if [ -n "$undef" ]; then
    echo "UNRESOLVED non-libc undefined symbols:"; echo "$undef" | sed 's/^/  /'; FAIL=1
  else
    echo "undefined non-libc symbols: 0  [OK]"
  fi

  # ---- differential tests (debug: unwinding panics give full diff reports) ----
  # shellcheck disable=SC2086
  ( cd "$CRATE_DIR" && timeout 600 cargo test $flags ) >/tmp/vt.log 2>&1
  rc=$?
  grep -E 'test result:' /tmp/vt.log | sed 's/^/  debug   /'
  [ $rc -ne 0 ] && { echo "  DEBUG TESTS FAILED"; grep -E 'FAILED|divergence' /tmp/vt.log | head -30; FAIL=1; }

  # shellcheck disable=SC2086
  ( cd "$CRATE_DIR" && timeout 600 cargo test --release $flags ) >/tmp/vtr.log 2>&1
  rc=$?
  grep -E 'test result:' /tmp/vtr.log | sed 's/^/  release /'
  [ $rc -ne 0 ] && { echo "  RELEASE TESTS FAILED"; grep -E 'FAILED|divergence' /tmp/vtr.log | head -30; FAIL=1; }
done

# ---------------------------------------------------------------------------
section "Binary targets"
if grep -q '^\[\[bin\]\]' "$CRATE_DIR/Cargo.toml" || [ -f "$CRATE_DIR/src/main.rs" ]; then
  echo "Rust binary target present -- stdout comparison required"
  FAIL=1
else
  echo "no Rust binary target (cdylib only)  [OK]"
fi
if grep -qE '^\s*add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "C executable present -- stdout comparison required"
  FAIL=1
else
  echo "no C executable target (SHARED library only)  [OK]"
fi

section "RESULT"
if [ "$FAIL" -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "FAILURES PRESENT"; fi
exit "$FAIL"
