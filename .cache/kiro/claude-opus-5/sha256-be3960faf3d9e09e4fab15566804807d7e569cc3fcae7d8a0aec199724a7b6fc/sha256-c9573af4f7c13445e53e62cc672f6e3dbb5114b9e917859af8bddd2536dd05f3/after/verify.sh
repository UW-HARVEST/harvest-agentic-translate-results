#!/usr/bin/env bash
# Full verification driver: build both libraries, diff exported symbols, and run
# every differential test under every Cargo feature combination.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/.verify"
mkdir -p "$OUT"
fail=0

echo "=== 1. build the C shared library ==="
mkdir -p "$ROOT/c_src/build"
( cd "$ROOT/c_src/build" \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >"$OUT/cmake.log" 2>&1 \
  && timeout 600 cmake --build . -- -j8 >"$OUT/cbuild.log" 2>&1 ) \
  || { echo "C BUILD FAILED (see $OUT/cbuild.log)"; exit 1; }
ls -l "$ROOT/c_src/build/libmujs.so"

echo
echo "=== 2. enumerate Cargo feature combinations ==="
# The crate declares no [features], so the only combination is the default.
mapfile -t FEATURES < <(
  python3 - "$ROOT/translation/Cargo.toml" <<'PY'
import re, sys, itertools
src = open(sys.argv[1]).read()
m = re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', src, re.M | re.S)
names = []
if m:
    for line in m.group(1).splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        k = line.split('=')[0].strip()
        if k and k != 'default':
            names.append(k)
if not names:
    print("__default__")
else:
    print("__default__")
    print("__none__")
    for r in range(1, len(names) + 1):
        for c in itertools.combinations(names, r):
            print(",".join(c))
PY
)
printf 'combination: %s\n' "${FEATURES[@]}"

for combo in "${FEATURES[@]}"; do
  case "$combo" in
    __default__) ARGS=() ; tag=default ;;
    __none__)    ARGS=(--no-default-features) ; tag=no-default ;;
    *)           ARGS=(--no-default-features --features "$combo") ; tag="$combo" ;;
  esac

  echo
  echo "=== 3.$tag  cargo build --release ${ARGS[*]} ==="
  ( cd "$ROOT/translation" && timeout 600 cargo build --release "${ARGS[@]}" ) \
    >"$OUT/rustbuild-$tag.log" 2>&1 \
    || { echo "RUST BUILD FAILED for '$tag' (see $OUT/rustbuild-$tag.log)"; fail=1; continue; }

  echo "=== 4.$tag  symbol parity (nm -D) ==="
  nm -D --defined-only "$ROOT/c_src/build/libmujs.so"           | awk '{print $3}' | sort > "$OUT/c_syms.txt"
  nm -D --defined-only "$ROOT/translation/target/release/libmujs.so" | awk '{print $3}' | sort > "$OUT/r_syms-$tag.txt"
  miss=$(comm -23 "$OUT/c_syms.txt" "$OUT/r_syms-$tag.txt")
  extra=$(comm -13 "$OUT/c_syms.txt" "$OUT/r_syms-$tag.txt")
  echo "C exports:    $(wc -l < "$OUT/c_syms.txt")"
  echo "Rust exports: $(wc -l < "$OUT/r_syms-$tag.txt")"
  if [ -n "$miss" ]; then echo "MISSING IN RUST:"; echo "$miss"; fail=1; else echo "missing: none"; fi
  if [ -n "$extra" ]; then echo "EXTRA IN RUST:"; echo "$extra"; else echo "extra: none"; fi
  echo "undefined non-libc symbols in the Rust .so:"
  nm -D --undefined-only "$ROOT/translation/target/release/libmujs.so" \
    | awk '{print $2}' | grep -v '@' | grep -vE '^(_ITM_|__gmon_start__)' || echo "  none"

  echo "=== 5.$tag  differential tests ==="
  for t in leaf_pure regexp_engine diag_alloc smoke_interp state_api interp_builtins interp_lang interp_errors; do
    printf '  %-16s ' "$t"
    if ( cd "$ROOT/translation" && timeout 600 cargo test --release "${ARGS[@]}" --test "$t" -- --test-threads=1 ) \
         >"$OUT/test-$tag-$t.log" 2>&1; then
      grep -h '^test result:' "$OUT/test-$tag-$t.log" | tr '\n' ' '; echo
    else
      echo "FAILED (see $OUT/test-$tag-$t.log)"; fail=1
    fi
  done
done

echo
if [ "$fail" = 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit $fail
