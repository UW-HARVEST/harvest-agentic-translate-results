#!/usr/bin/env bash
# Completion gate for the PCRE2 C -> Rust differential verification.
#
# Checks, in order:
#   1. c_src/ is unmodified (the C is ground truth and must never be edited)
#   2. both shared libraries build
#   3. exported-symbol parity is EXACT (SYMBOLS.md / Phase D)
#   4. no undefined non-libc / non-Rust-runtime symbols in the Rust .so
#   5. every ERRORS.md row has a Phase C test and every CONFIGS.md row a
#      Phase B test (check_coverage.py)
#   6. the whole differential test suite passes
#
# Run from the crate root:  ./verify.sh
set -uo pipefail
cd "$(dirname "$0")"
: "${TMPDIR:=/tmp}"
W="$TMPDIR/pcre2-verify"; mkdir -p "$W"
fail=0
step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
ok()   { printf '  \033[32mOK\033[0m   %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$*"; fail=1; }

step "1. c_src/ untouched"
if [ -f "$W/c_src.md5" ]; then
  if md5sum -c --quiet "$W/c_src.md5" 2>/dev/null; then
    ok "c_src/ matches the recorded baseline"
  else
    bad "c_src/ HAS BEEN MODIFIED — the C is ground truth"
  fi
else
  find c_src/src c_src/include c_src/CMakeLists.txt -type f | sort | xargs md5sum > "$W/c_src.md5"
  ok "recorded c_src/ baseline ($(wc -l < "$W/c_src.md5") files)"
fi

step "2. build both libraries"
( mkdir -p c_src/build && cd c_src/build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null 2>&1 \
  && cmake --build . -j 8 >/dev/null 2>&1 ) \
  && ok "C  .so built" || bad "C .so build failed"
cargo build --release >/dev/null 2>&1 && ok "Rust .so built" || bad "Rust .so build failed"
C_SO=c_src/build/libpcre2.so
R_SO=target/release/libpcre2.so

step "3. exported-symbol parity"
nm -D --defined-only "$C_SO" | awk '$2!=""{print $3"\t"$2}' | sort > "$W/c.sym"
nm -D --defined-only "$R_SO" | awk '$2!=""{print $3"\t"$2}' | sort > "$W/r.sym"
nc=$(wc -l < "$W/c.sym"); nr=$(wc -l < "$W/r.sym")
miss=$(comm -23 <(cut -f1 "$W/c.sym") <(cut -f1 "$W/r.sym"))
extra=$(comm -13 <(cut -f1 "$W/c.sym") <(cut -f1 "$W/r.sym"))
printf '  C exports %s, Rust exports %s\n' "$nc" "$nr"
[ -z "$miss"  ] && ok "0 symbols missing from Rust"  || bad "missing from Rust: $miss"
[ -z "$extra" ] && ok "0 extra symbols in Rust"      || bad "extra in Rust: $extra"
if diff -q "$W/c.sym" "$W/r.sym" >/dev/null; then
  ok "symbol names AND nm types (T/R/D) identical"
else
  bad "symbol types differ:"; diff "$W/c.sym" "$W/r.sym" | head
fi

step "4. undefined symbols in the Rust .so"
# everything the Rust runtime and libc legitimately import
nm -D --undefined-only "$R_SO" | awk '{print $2}' | sed 's/@.*//' | sort -u \
  | grep -vE '^(_ITM_(de)?registerTMCloneTable|_Unwind_[A-Za-z]+|__cxa_finalize|__cxa_thread_atexit_impl|__errno_location|__gmon_start__|__tls_get_addr|abort|bcmp|calloc|close|dl_iterate_phdr|free|fstat64|getcwd|getenv|gettid|is[a-z]+|lseek64|malloc|memchr|memcmp|memcpy|memmove|memset|mmap64|munmap|open64|posix_memalign|pthread_key_create|pthread_key_delete|pthread_setspecific|read|readlink|realloc|realpath|stat64|statx|strlen|syscall|to(lower|upper)|write|writev)$' \
  > "$W/undef" || true
if [ ! -s "$W/undef" ]; then
  ok "0 undefined non-libc / non-runtime symbols"
else
  bad "unexpected undefined symbols:"; cat "$W/undef"
fi

step "5. ERRORS.md / CONFIGS.md row coverage"
python3 check_coverage.py && ok "every row has a test" || bad "rows without tests (see above)"

step "6. differential test suite"
if cargo test --release -- --test-threads=1 2>&1 | tee "$W/test.log" \
     | grep -E '^(running|test result)'; then :; fi
if grep -qE 'FAILED|panicked|error: test failed' "$W/test.log"; then
  bad "test failures:"; grep -E 'FAILED|panicked|error: test failed' "$W/test.log" | head
else
  tp=$(grep -oE '[0-9]+ passed' "$W/test.log" | awk '{s+=$1} END{print s}')
  ok "all ${tp:-0} tests passed"
fi

printf '\n\033[1m== RESULT: '
if [ "$fail" -eq 0 ]; then printf '\033[32mVERIFICATION COMPLETE\033[0m\n'; else printf '\033[31mINCOMPLETE\033[0m\n'; fi
exit "$fail"
