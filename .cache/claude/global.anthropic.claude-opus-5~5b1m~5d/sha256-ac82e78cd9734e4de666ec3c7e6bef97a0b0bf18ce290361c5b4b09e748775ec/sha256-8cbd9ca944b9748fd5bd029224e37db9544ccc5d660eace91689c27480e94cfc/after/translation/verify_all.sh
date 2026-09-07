#!/bin/bash
# Phase D completion gate: symbol parity + the full suite under every
# feature/profile configuration.
set -uo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
fail=0

echo "=============== feature combinations declared in Cargo.toml ==============="
python3 - <<'PY'
import re
s=open('Cargo.toml').read()
m=re.search(r'^\[features\]\s*$(.*?)(^\[|\Z)', s, re.M|re.S)
feats=[]
if m:
    feats=[l.split('=')[0].strip() for l in m.group(1).splitlines()
           if l.strip() and not l.strip().startswith('#')]
print("features:", feats if feats else "(none declared)")
PY

echo
echo "=============== running suite under every configuration ==============="
for profile in debug release; do
  for featflag in "" "--no-default-features" "--all-features"; do
    label="profile=$profile flags=${featflag:-<default>}"
    out=$(PROFILE=$profile timeout 600 ./run_tests.sh $featflag 2>&1)
    if echo "$out" | grep -qE '^test result: FAILED|SIGSEGV|SIGABRT|error: test failed|error\[|error: could not'; then
      echo "FAIL  $label"
      echo "$out" | grep -E '^test .* FAILED|^error' | head -6 | sed 's/^/        /'
      fail=1
    else
      tot=$(echo "$out" | grep -oE '^test result: ok\. [0-9]+ passed' \
            | grep -oE '[0-9]+' | awk '{s+=$1} END{print s+0}')
      echo "PASS  $label  ($tot tests)"
    fi
  done
done

echo
echo "=============== symbol parity (Phase A / D gate) ==============="
CSO=$(ls "$ROOT"/c_src/build/*.so | head -1)
RSO=$(ls -t target/release/libsynth_pair_lib.so target/debug/libsynth_pair_lib.so 2>/dev/null | head -1)
echo "C   : $CSO"
echo "Rust: $RSO"
nm -D --defined-only "$CSO"  | awk '$2=="T"||$2=="W"{print $3}' | sort -u > .scratch/c.syms
nm -D --defined-only "$RSO" | awk '$2=="T"||$2=="W"{print $3}' | sort -u > .scratch/r.syms
echo "-- C exported symbols:";  sed 's/^/     /' .scratch/c.syms
echo "-- exported by C but MISSING from Rust:"
missing=$(comm -23 .scratch/c.syms .scratch/r.syms)
if [ -z "$missing" ]; then echo "     (none)"; else echo "$missing" | sed 's/^/     /'; fail=1; fi

echo "-- undefined (imported) non-libc symbols in the Rust .so:"
# Strip the @GLIBC_x.y / @GCC_x.y version suffix before filtering, otherwise
# every plain libc import looks like an unresolved symbol.
und=$(nm -D -u "$RSO" | awk '{print $2}' | sed 's/@.*//' \
      | grep -vE '^(_ITM_|__cxa_|__gmon_|__tls_|_Unwind_|__libc|__rust|pthread_|__errno_location)' \
      | grep -vE '^(abort|memcpy|memmove|memset|memcmp|bcmp|free|malloc|calloc|realloc|posix_memalign|dl_iterate_phdr|sysconf|open64|open|close|read|readlink|realpath|write|writev|lseek64|fstat64|stat64|statx|mmap64|munmap|gettid|syscall|getcwd|getenv|strlen|__stack_chk_fail)$' \
      | sort -u)
if [ -z "$und" ]; then echo "     (none)"; else echo "$und" | sed 's/^/     /'; fi

echo
echo "=============== binary/driver executable ==============="
if grep -qE '^\[\[bin\]\]' Cargo.toml || grep -qE 'add_executable' "$ROOT/c_src/CMakeLists.txt"; then
  echo "  a binary exists -- stdout comparison REQUIRED"; fail=1
else
  echo "  none: CMakeLists.txt declares only add_library(SHARED), Cargo.toml only cdylib."
  echo "  -> stdout comparison N/A"
fi

echo
if [ "$fail" = 0 ]; then echo "ALL CONFIGURATIONS PASS"; else echo "SOME CHECKS FAILED"; fi
exit $fail
