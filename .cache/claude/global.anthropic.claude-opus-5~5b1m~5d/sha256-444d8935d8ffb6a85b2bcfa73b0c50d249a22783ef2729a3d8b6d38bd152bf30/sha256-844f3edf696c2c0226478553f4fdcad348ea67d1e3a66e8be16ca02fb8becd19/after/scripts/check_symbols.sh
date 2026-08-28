#!/usr/bin/env bash
# Phase D: symbol parity between the C .so and the Rust .so for every
# configuration. Prints the diff of "symbols exported by C but not by Rust"
# (which must be empty) plus the non-libc undefined-symbol check.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# libc / unwinder imports that both objects legitimately have
LIBC_RE='^(_Unwind_|__|_ITM|pthread_|printf|fprintf|puts|putchar|atoi|strtol|stderr|stdout|mem(cpy|move|set|cmp)|bcmp|strlen|malloc|calloc|realloc|free|posix_memalign|abort|write|writev|read|close|open|open64|lseek64|fstat64|stat64|statx|mmap64|munmap|getcwd|getenv|readlink|realpath|syscall|gettid|dl_iterate_phdr|sysconf|strerror_r|signal|sigaction|sigaltstack|raise|getpid|environ)'

defined() { nm -D --defined-only "$1" | awk '{print $3}' | grep -vE '^(_init|_fini|__)' | sort -u; }
undefined() { nm -D --undefined-only "$1" | awk '{print $NF}' | sed 's/@.*//' | sort -u; }

status=0
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    c="$ROOT/cbuild/so/libdriver_${op}_${rep}.so"
    r="$ROOT/cbuild/rs/libmacrodepth_${op}_${rep}.so"
    if [ ! -f "$c" ] || [ ! -f "$r" ]; then
      echo "SKIP $op:$rep (missing $( [ -f "$c" ] || echo "$c" ) $( [ -f "$r" ] || echo "$r" ))"
      status=1
      continue
    fi
    missing=$(comm -23 <(defined "$c") <(defined "$r") | tr '\n' ' ')
    extra_undef=$(undefined "$r" | grep -vE "$LIBC_RE" | tr '\n' ' ')
    nsym=$(defined "$c" | wc -l)
    if [ -n "$missing" ] || [ -n "$extra_undef" ]; then
      echo "FAIL $op:$rep  missing-from-rust: [$missing]  non-libc-undefined: [$extra_undef]"
      status=1
    else
      echo "ok   $op:$rep  ($nsym C exports, 0 missing, 0 non-libc undefined)"
    fi
  done
done
[ "$status" -eq 0 ] && echo "SYMBOL PARITY: empty diff for all configurations"
exit "$status"
