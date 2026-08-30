#!/bin/bash
# Run the FFI differential tests across every feature combination.
#
# Usage: ./run_all_tests.sh [extra args passed to `cargo test`, e.g. --test ffi_low]
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
pass=0; fail=0; failed=()
for backend in haraka sha2 shake blake; do
  for thash in robust simple; do
    for secpar in 128s 128f 192s 192f 256s 256f; do
      combo="${backend}/${thash}/${secpar}"
      out=$("$ROOT/run_tests.sh" "$backend" "$thash" "$secpar" "$@" 2>&1)
      if echo "$out" | grep -qE "^(error|test result: FAILED)" || \
         echo "$out" | grep -q "FAILED" || [ -z "$(echo "$out" | grep 'test result: ok')" ]; then
        echo "FAIL $combo"
        echo "$out" | grep -E "panicked|mismatch|FAILED|error|SIGABRT|signal:|assertion" | head -12 | sed 's/^/      /'
        fail=$((fail+1)); failed+=("$combo")
      else
        echo "ok   $combo"
        pass=$((pass+1))
      fi
    done
  done
done
echo
echo "passed: $pass   failed: $fail"
if [ "$fail" -gt 0 ]; then printf 'failing combos: %s\n' "${failed[*]}"; exit 1; fi
