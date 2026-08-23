#!/usr/bin/env bash
# End-to-end verification driver.
#
#   .phaseA/verify_all.sh
#
# 1. builds the C reference .so
# 2. builds the Rust cdylib for every Cargo feature combination
# 3. compares `nm -D` symbol tables (must be empty in both directions)
# 4. runs the whole differential test suite for every feature combination,
#    recording which tests passed
# 5. rewrites the `test` column / checkboxes of ERRORS.md and CONFIGS.md from
#    the recorded pass set and prints the coverage summary
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1
OUT="${TMPDIR:-/tmp}/verify_all"
mkdir -p "$OUT"
rc=0

banner() { printf '\n============================================================\n%s\n============================================================\n' "$1"; }

banner "1. build the C reference shared library"
mkdir -p c_src/build
( cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON > "$OUT/cmake.log" 2>&1 \
  && cmake --build . -j 8 > "$OUT/cbuild.log" 2>&1 )
if [ ! -f c_src/build/libsodium.so ]; then
  echo "!!! C build FAILED (see $OUT/cbuild.log)"; exit 1
fi
echo "OK  c_src/build/libsodium.so ($(stat -c%s c_src/build/libsodium.so) bytes)"

banner "2. feature combinations"
bash .phaseA/feature_matrix.sh check 2>&1 | tee "$OUT/features.log" | sed -n '1,12p'
grep -q '!!! FAILED' "$OUT/features.log" && { echo "!!! a feature combination failed to check"; rc=1; }

banner "3. symbol parity (nm -D)"
timeout 600 cargo build --no-default-features > "$OUT/rbuild.log" 2>&1 \
  || { echo "!!! cargo build FAILED"; tail -20 "$OUT/rbuild.log"; exit 1; }
nm -D --defined-only c_src/build/libsodium.so    | awk '$3!=""{print $3}' | sort -u > "$OUT/c.txt"
nm -D --defined-only target/debug/liblibsodium.so | awk '$3!=""{print $3}' | sort -u > "$OUT/r.txt"
n_c=$(wc -l < "$OUT/c.txt"); n_r=$(wc -l < "$OUT/r.txt")
comm -23 "$OUT/c.txt" "$OUT/r.txt" > "$OUT/missing.txt"
comm -13 "$OUT/c.txt" "$OUT/r.txt" > "$OUT/extra.txt"
n_m=$(wc -l < "$OUT/missing.txt"); n_x=$(wc -l < "$OUT/extra.txt")
echo "C exports: $n_c   Rust exports: $n_r   missing: $n_m   extra: $n_x"
if [ "$n_m" -ne 0 ]; then echo "!!! MISSING SYMBOLS:"; cat "$OUT/missing.txt"; rc=1; fi
if [ "$n_x" -ne 0 ]; then echo "!!! EXTRA SYMBOLS:"; cat "$OUT/extra.txt"; rc=1; fi
echo "--- undefined (imported) symbols in the Rust .so that are NOT libc/libgcc ---"
nm -D --undefined-only target/debug/liblibsodium.so | awk '{print $NF}' \
  | grep -vE '^(_ITM_|__|_Unwind_|abort|bcmp|calloc|close|dl_iterate_phdr|fcntl|free|fstat|getcwd|getenv|gettid|gettimeofday|lseek|malloc|memchr|memcmp|memcpy|memmove|memset|mmap|munmap|open|poll|posix_memalign|pthread_|read|readlink|realloc|realpath|stat|statx|strchr|strlen|strrchr|syscall|write|writev)' \
  | sed 's/@.*//' | sort -u | tee "$OUT/undef.txt"
[ -s "$OUT/undef.txt" ] && { echo "!!! non-libc undefined symbols present"; rc=1; } || echo "(none)"

banner "4. differential test suite, per feature combination"
: > "$OUT/passed.txt"
mapfile -t COMBOS < <(bash -c '
  awk "/^\[features\]/{inf=1;next} /^\[/{inf=0} inf && /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/{sub(/[[:space:]]*=.*/,\"\");gsub(/[[:space:]]/,\"\");if(\$0!=\"default\")print}" Cargo.toml')
if [ "${#COMBOS[@]}" -eq 0 ]; then COMBO_LIST=(""); else
  COMBO_LIST=(""); n=${#COMBOS[@]}; total=$((1 << n))
  for ((mask=1; mask<total; mask++)); do
    combo=""
    for ((i=0;i<n;i++)); do ((mask & (1<<i))) && combo="${combo:+$combo,}${COMBOS[$i]}"; done
    COMBO_LIST+=("$combo")
  done
fi
for combo in "${COMBO_LIST[@]}"; do
  label="${combo:-<no features>}"
  echo "--- cargo test --no-default-features --features \"$combo\" ---"
  if [ -z "$combo" ]; then
    timeout 1800 cargo test --no-default-features -- --test-threads=1 > "$OUT/test.log" 2>&1
  else
    timeout 1800 cargo test --no-default-features --features "$combo" -- --test-threads=1 > "$OUT/test.log" 2>&1
  fi
  trc=$?
  grep -E '^test result:' "$OUT/test.log"
  # record "<binary>::<test>" for every passing test
  awk '
    /^ *Running (unittests|tests\/)/ { bin = $2; sub(/^tests\//, "", bin); sub(/\.rs$/, "", bin); next }
    /^test .* \.\.\. ok$/ { name = $2; sub(/:.*/, "", name); print bin "::" name }
  ' "$OUT/test.log" >> "$OUT/passed.txt"
  if [ "$trc" -ne 0 ]; then
    echo "!!! TESTS FAILED for $label"
    grep -E '^(test .* FAILED|failures:|---- )' "$OUT/test.log" | head -40
    rc=1
  else
    echo "OK: $label"
  fi
done
sort -u -o "$OUT/passed.txt" "$OUT/passed.txt"
echo "recorded $(wc -l < "$OUT/passed.txt") passing tests"

banner "5. ERRORS.md / CONFIGS.md row coverage"
python3 .phaseA/coverage.py --write --passed "$OUT/passed.txt" 2>&1 | tee "$OUT/coverage.log" | grep -vE '^   UNCOVERED'
if grep -qE 'UNCOVERED \([1-9]' "$OUT/coverage.log"; then
  echo "!!! some rows are still uncovered:"
  grep -E '^   UNCOVERED' "$OUT/coverage.log" | cut -c1-400
  rc=1
fi

banner "6. exported-symbol reachability audit"
python3 .phaseA/symbol_audit.py | tee "$OUT/audit.log" | head -8
grep -qE 'UNREFERENCED +: 0$' "$OUT/audit.log" || { echo "!!! some exported symbols are never driven by a test"; rc=1; }

banner "RESULT"
if [ "$rc" -eq 0 ]; then
  echo "ALL CHECKS PASSED"
else
  echo "FAILURES PRESENT (see above)"
fi
exit "$rc"
