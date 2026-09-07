#!/usr/bin/env bash
# Differential verification driver.
#  1. builds the C .so and the Rust .so
#  2. diffs `nm -D` exported symbols (must be empty)
#  3. runs every test suite under EVERY cargo feature combination
set -uo pipefail
cd "$(dirname "$0")"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/difftest.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
ROOT=$(cd .. && pwd)

echo "== building C shared library =="
( cd "$ROOT/c_src" && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null \
  && cmake --build . >/dev/null ) || exit 1
C_SO=$(ls "$ROOT"/c_src/build/lib*.so | head -1)

echo "== enumerating feature combinations =="
# every declared feature (none => only the default build)
FEATS=$(python3 - <<'PY'
import re,sys
txt=open('Cargo.toml').read()
m=re.search(r'^\[features\](.*?)(^\[|\Z)', txt, re.S|re.M)
names=[]
if m:
    for line in m.group(1).splitlines():
        line=line.split('#')[0].strip()
        if '=' in line:
            n=line.split('=')[0].strip()
            if n not in ('default',):
                names.append(n)
print(' '.join(names))
PY
)
if [ -z "$FEATS" ]; then
  echo "   (no [features] declared -> single default configuration)"
  COMBOS=("default")
else
  COMBOS=("default" "none")
  # power set of the declared features
  python3 - "$FEATS" <<'PY' > "$TMP/combos"
import itertools,sys
f=sys.argv[1].split()
for r in range(1,len(f)+1):
    for c in itertools.combinations(f,r):
        print(','.join(c))
PY
  while read -r c; do COMBOS+=("$c"); done < "$TMP/combos"
  rm -f "$TMP/combos"
fi

FAIL=0
for combo in "${COMBOS[@]}"; do
  case "$combo" in
    default) FLAGS=() ;;
    none)    FLAGS=(--no-default-features) ;;
    *)       FLAGS=(--no-default-features --features "$combo") ;;
  esac
  echo
  echo "############ feature combination: $combo ############"
  cargo build --release --offline "${FLAGS[@]}" >/dev/null 2>&1 || { echo "BUILD FAILED"; FAIL=1; continue; }
  R_SO=target/release/libstr_dups_lib.so

  echo "-- symbol parity (nm -D) --"
  nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > "$TMP/c.syms"
  nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > "$TMP/r.syms"
  MISSING=$(comm -23 "$TMP/c.syms" "$TMP/r.syms")
  EXTRA=$(comm -13 "$TMP/c.syms" "$TMP/r.syms")
  echo "   C exports:    $(wc -l < "$TMP/c.syms")"
  echo "   Rust exports: $(wc -l < "$TMP/r.syms")"
  if [ -n "$MISSING" ]; then echo "   MISSING FROM RUST:"; echo "$MISSING" | sed 's/^/     /'; FAIL=1; else echo "   missing: none"; fi
  if [ -n "$EXTRA" ];   then echo "   extra in Rust:"; echo "$EXTRA" | sed 's/^/     /'; fi
  rm -f "$TMP/c.syms" "$TMP/r.syms"

  echo "-- undefined non-libc symbols in the Rust .so --"
  nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
    | grep -vE '^(_ITM_.*|_Unwind_.*|_.*|realloc|free|malloc|calloc|posix_memalign|mem(set|cpy|move|cmp|rchr)|str(cmp|len|chr)|printf|sprintf|snprintf|fprintf|abort|dl_iterate_phdr|getenv|getcwd|gettid|realpath|write|writev|pthread_.*|__.*|gnu_get_libc_version|syscall|sysconf|open|open64|close|read|readlink|munmap|mmap|mmap64|mprotect|poll|sigaltstack|sigaction|sigaddset|sigemptyset|abs|qsort|bcmp|environ|stat|stat64|statx|fstat|fstat64|lstat|lseek|lseek64)$' \
    | sed 's/^/     /' > "$TMP/undef" || true
  if [ -s "$TMP/undef" ]; then cat "$TMP/undef"; echo "   ^^ NON-LIBC UNDEFINED SYMBOLS"; FAIL=1; else echo "   none (libc / Rust-runtime only)"; fi

  echo "-- differential tests --"
  timeout 600 cargo test --release --offline "${FLAGS[@]}" -- --test-threads=1 2>&1 \
    | grep -E "Running|^test result|FAILED|panicked" || FAIL=1
  cargo test --release --offline "${FLAGS[@]}" >/dev/null 2>&1 || FAIL=1
done

echo
if [ "$FAIL" = 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "FAILURES DETECTED"; fi
exit $FAIL
