#!/usr/bin/env bash
# Phase D driver: build both libraries, prove symbol parity, and run the full
# differential suite under EVERY feature combination and BOTH Rust profiles.
#
# Usage:  ./run_all.sh            (from the `translation/` directory)
set -uo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$CRATE_DIR")"
cd "$CRATE_DIR"

RED=$'\033[31m'; GRN=$'\033[32m'; YLW=$'\033[33m'; RST=$'\033[0m'
FAILURES=0
step() { printf '\n%s=== %s ===%s\n' "$YLW" "$1" "$RST"; }
ok()   { printf '%s[ok]%s   %s\n' "$GRN" "$RST" "$1"; }
bad()  { printf '%s[FAIL]%s %s\n' "$RED" "$RST" "$1"; FAILURES=$((FAILURES+1)); }

# ---------------------------------------------------------------------------
step "1. Build the C shared library"
# ---------------------------------------------------------------------------
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || { bad "C build"; exit 1; }
C_SO="$(ls "$ROOT"/c_src/build/lib*.so | head -1)"
ok "C  .so = $C_SO"

# ---------------------------------------------------------------------------
step "2. Build the Rust cdylib (release + debug)"
# ---------------------------------------------------------------------------
cargo build --release -q || { bad "cargo build --release"; exit 1; }
cargo build -q          || { bad "cargo build (debug)";   exit 1; }
R_REL="$CRATE_DIR/target/release/libarr_push_lib.so"
R_DBG="$CRATE_DIR/target/debug/libarr_push_lib.so"
ok "RUST release = $R_REL"
ok "RUST debug   = $R_DBG"

# ---------------------------------------------------------------------------
step "3. Symbol parity (nm -D)"
# ---------------------------------------------------------------------------
WORK="$CRATE_DIR/target/symcheck"; mkdir -p "$WORK"
nm -D --defined-only "$C_SO"  | awk '{print $3}' | sort -u > "$WORK/c.txt"
nm -D --defined-only "$R_REL" | awk '{print $3}' | sort -u > "$WORK/r.txt"
MISSING="$(comm -23 "$WORK/c.txt" "$WORK/r.txt")"
printf 'C exports:    %s\n' "$(wc -l < "$WORK/c.txt")"
printf 'Rust exports: %s\n' "$(wc -l < "$WORK/r.txt")"
if [ -n "$MISSING" ]; then
  bad "symbols exported by C but MISSING from Rust:"; printf '  %s\n' $MISSING
else
  ok "symbol diff is EMPTY (0 missing)"
fi

# undefined non-libc / non-Rust-runtime symbols in the Rust .so
UNDEF="$(nm -D --undefined-only "$R_REL" | awk '{print $2}' \
        | grep -vE '^(_Unwind_|__|_ITM_|gettid|statx)' \
        | grep -vE '^(abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|lseek64|malloc|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_[a-z_]+|read|readlink|realloc|realpath|stat64|strlen|syscall|write|writev)(@.*)?$' \
        || true)"
if [ -n "$UNDEF" ]; then
  bad "unexpected undefined symbols in the Rust .so:"; printf '  %s\n' $UNDEF
else
  ok "0 undefined non-libc/non-runtime symbols"
fi

# ---------------------------------------------------------------------------
step "4. Enumerate feature combinations from Cargo.toml"
# ---------------------------------------------------------------------------
FEATS="$(python3 - <<'PY'
import re
src = open('Cargo.toml').read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.split('#', 1)[0].strip()
        if '=' in line:
            n = line.split('=', 1)[0].strip().strip('"')
            if n and n != 'default':
                names.append(n)
print(' '.join(names))
PY
)"

COMBOS=()
if [ -z "$FEATS" ]; then
  echo "no [features] table -> the only configurations are default / --no-default-features"
  COMBOS+=("DEFAULT" "NODEFAULT")
else
  echo "features: $FEATS"
  COMBOS+=("DEFAULT" "NODEFAULT")
  # full power set of the declared features, with default features off
  mapfile -t COMBOS < <(python3 - "$FEATS" <<'PY'
import itertools, sys
f = sys.argv[1].split()
print("DEFAULT"); print("NODEFAULT")
for n in range(1, len(f) + 1):
    for c in itertools.combinations(f, n):
        print("NODEFAULT:" + ",".join(c))
PY
)
fi
printf 'combinations to test: %s\n' "${#COMBOS[@]}"

# ---------------------------------------------------------------------------
step "5. Run the differential suite for every (combo x profile)"
# ---------------------------------------------------------------------------
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    DEFAULT)      FLAGS=() ;;
    NODEFAULT)    FLAGS=(--no-default-features) ;;
    NODEFAULT:*)  FLAGS=(--no-default-features --features "${combo#NODEFAULT:}") ;;
  esac
  # Rebuild the cdylib for this feature combo, then test against it.
  cargo build --release -q "${FLAGS[@]}" || { bad "build $combo (release)"; continue; }
  cargo build -q "${FLAGS[@]}"           || { bad "build $combo (debug)";   continue; }

  for prof in release debug; do
    SO="$CRATE_DIR/target/$prof/libarr_push_lib.so"
    LOG="$CRATE_DIR/target/test-$(echo "$combo" | tr ':,' '__')-$prof.log"
    if C_SO="$C_SO" RUST_SO="$SO" \
         timeout 600 cargo test --release "${FLAGS[@]}" -- --test-threads=1 >"$LOG" 2>&1; then
      N="$(grep -hoE '[0-9]+ passed' "$LOG" | awk '{s+=$1} END {print s}')"
      ok "$combo / rust-$prof : $N tests passed"
    else
      bad "$combo / rust-$prof : see $LOG"
      grep -E "^test .* FAILED|panicked at|SIGABRT|Assertion" "$LOG" | head -10
    fi
  done
done

# restore the canonical release build
cargo build --release -q

# ---------------------------------------------------------------------------
step "SUMMARY"
# ---------------------------------------------------------------------------
if [ "$FAILURES" -eq 0 ]; then
  printf '%sALL CHECKS PASSED%s\n' "$GRN" "$RST"
else
  printf '%s%d CHECK(S) FAILED%s\n' "$RED" "$FAILURES" "$RST"
fi
exit "$FAILURES"
