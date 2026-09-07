#!/usr/bin/env bash
# Phase D — symbol parity between the C .so and the Rust .so.
# Exits non-zero if the Rust object is missing any symbol the C object defines.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$here")"

c_so="$(find "$root/c_src/build" -maxdepth 1 -name '*.so' | head -1)"
rust_so="${RUST_SO:-$root/translation/target/release/libhsl_to_rgb_lib.so}"

if [[ -z "$c_so" || ! -f "$c_so" ]]; then
  echo "FAIL: no C .so found under $root/c_src/build" >&2
  exit 1
fi
if [[ ! -f "$rust_so" ]]; then
  echo "FAIL: Rust .so not found at $rust_so" >&2
  exit 1
fi

echo "C    .so: $c_so"
echo "Rust .so: $rust_so"
echo

# Defined symbols only, dropping the compiler/runtime underscore-prefixed
# housekeeping entries (_init, _fini, __cxa_*, _ITM_*, ...), which are not API.
syms() {
  nm -D --defined-only "$1" | awk '{print $NF}' | grep -v '^_' | sort -u
}

# Undefined (imported) symbols, for the "0 undefined non-libc symbols" check.
undef() {
  nm -D --undefined-only "$1" | awk '{print $NF}' | sed 's/@.*//' | grep -v '^_' | sort -u
}

c_syms="$(syms "$c_so")"
r_syms="$(syms "$rust_so")"

echo "=== C defined symbols ($(echo "$c_syms" | grep -c . )) ==="
echo "$c_syms"
echo
echo "=== Rust defined symbols ($(echo "$r_syms" | grep -c . )) ==="
echo "$r_syms"
echo

missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms"))"
extra="$(comm -13 <(echo "$c_syms") <(echo "$r_syms"))"

echo "=== Missing from Rust (must be empty) ==="
echo "${missing:-<none>}"
echo
echo "=== Extra in Rust (informational) ==="
echo "${extra:-<none>}"
echo

echo "=== Rust undefined imports (resolved against the dynamic loader) ==="
undef "$rust_so" | sed 's/^/  /'
echo
# Authoritative check: ask the loader to resolve every relocation. Any symbol
# that is not provided by a DT_NEEDED library shows up as "undefined symbol".
echo "=== ldd -r unresolved report (must be empty) ==="
ldd_out="$(ldd -r "$rust_so" 2>&1)"
bad_undef="$(echo "$ldd_out" | grep -i 'undefined symbol' || true)"
echo "${bad_undef:-<none>}"
echo
echo "=== DT_NEEDED of the Rust .so ==="
objdump -p "$rust_so" | awk '/NEEDED/ {print "  " $2}'
echo
echo "=== ldd -r unresolved report for the C .so (baseline) ==="
c_bad="$(ldd -r "$c_so" 2>&1 | grep -i 'undefined symbol' || true)"
echo "${c_bad:-<none>}"
echo

rc=0
if [[ -n "$missing" ]]; then
  echo "FAIL: Rust .so is missing $(echo "$missing" | grep -c .) symbol(s) the C .so exports." >&2
  rc=1
fi
if [[ -n "$bad_undef" ]]; then
  echo "FAIL: Rust .so has undefined symbols the loader cannot resolve." >&2
  rc=1
fi
if [[ $rc -eq 0 ]]; then
  echo "PASS: symbol diff is empty and every Rust import resolves via the loader."
fi
exit $rc
