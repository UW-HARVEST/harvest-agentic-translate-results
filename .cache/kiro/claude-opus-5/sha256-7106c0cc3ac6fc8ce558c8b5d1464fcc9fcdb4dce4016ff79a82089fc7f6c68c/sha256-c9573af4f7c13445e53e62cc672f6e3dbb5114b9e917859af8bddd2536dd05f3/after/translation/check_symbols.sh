#!/usr/bin/env bash
# Phase D -- symbol parity. Compares `nm -D` on the C .so and the Rust .so.
# Exits non-zero if the C .so exports anything the Rust .so does not, or if the
# Rust .so has undefined non-libc references.
set -uo pipefail
cd "$(dirname "$0")"

C_SO=$(ls ../c_src/build/*.so 2>/dev/null | head -1)
if [ -z "${C_SO:-}" ]; then
  echo "C .so not found. Build it:"
  echo "  cd ../c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
  exit 1
fi

RS_SO=${1:-}
if [ -z "$RS_SO" ]; then
  cargo build --lib --target-dir target/ffi-so >/dev/null 2>&1 || { echo "cargo build --lib failed"; exit 1; }
  RS_SO=target/ffi-so/debug/libfallcalc_lib.so
fi
[ -f "$RS_SO" ] || { echo "Rust .so not found: $RS_SO"; exit 1; }

echo "C    .so: $C_SO"
echo "Rust .so: $RS_SO"
echo

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT

# Defined, non-weak dynamic symbols exported by the C library.
nm -D --defined-only "$C_SO" | awk '$2 ~ /^[TDBRtdbr]$/ {print $3}' | sort -u > "$tmp/c.txt"

# Same for Rust, minus Rust-internal symbols that have no C counterpart.
nm -D --defined-only "$RS_SO" \
  | awk '$2 ~ /^[TDBRtdbr]$/ {print $3}' \
  | grep -v -E '^_ZN|^__rust_|^__rdl_|^__rg_|^rust_' \
  | sort -u > "$tmp/rs.txt"

echo "C exports   ($(wc -l < "$tmp/c.txt")):"
sed 's/^/  /' "$tmp/c.txt"
echo
echo "Rust exports ($(wc -l < "$tmp/rs.txt")), Rust-internal symbols filtered:"
sed 's/^/  /' "$tmp/rs.txt"
echo

comm -23 "$tmp/c.txt" "$tmp/rs.txt" > "$tmp/missing.txt"
comm -13 "$tmp/c.txt" "$tmp/rs.txt" > "$tmp/extra.txt"

rc=0
if [ -s "$tmp/missing.txt" ]; then
  echo "FAIL: exported by C but MISSING from Rust:"
  sed 's/^/  - /' "$tmp/missing.txt"
  rc=1
else
  echo "OK: symbol diff (C -> Rust) is empty; 0 missing symbols."
fi

if [ -s "$tmp/extra.txt" ]; then
  echo "note: exported by Rust but not by C (harmless, listed for completeness):"
  sed 's/^/  + /' "$tmp/extra.txt"
fi

# Undefined references in the Rust .so must all be resolvable from the system
# C runtime. Rather than hand-maintain an allowlist, resolve each one against
# the actual glibc / libgcc / libm shared objects.
nm -D --undefined-only "$RS_SO" \
  | awk '$1 == "U" || $2 == "U" {print $NF}' \
  | sed 's/@.*//' \
  | sort -u > "$tmp/rs_undef.txt"

: > "$tmp/libc_syms.txt"
for l in /lib64/libc.so.6 /lib64/libm.so.6 /lib64/libgcc_s.so.1 /lib64/libpthread.so.0 \
         /lib64/libdl.so.2 /lib64/librt.so.1 /lib64/ld-linux-x86-64.so.2 \
         /lib/x86_64-linux-gnu/libc.so.6 /lib/x86_64-linux-gnu/libm.so.6 \
         /lib/x86_64-linux-gnu/libgcc_s.so.1; do
  [ -f "$l" ] && nm -D --defined-only "$l" 2>/dev/null | awk '{print $NF}' | sed 's/@.*//' >> "$tmp/libc_syms.txt"
done
sort -u "$tmp/libc_syms.txt" -o "$tmp/libc_syms.txt"

# Weak toolchain stubs are absent from libc by design and are never called.
WEAK_STUBS='^(_ITM_deregisterTMCloneTable|_ITM_registerTMCloneTable|__gmon_start__)$'

comm -23 "$tmp/rs_undef.txt" "$tmp/libc_syms.txt" \
  | grep -v -E "$WEAK_STUBS" > "$tmp/rs_undef_bad.txt"

echo
echo "Rust .so undefined references: $(wc -l < "$tmp/rs_undef.txt") total, \
all checked against $(wc -l < "$tmp/libc_syms.txt") symbols exported by the system C runtime"
if [ -s "$tmp/rs_undef_bad.txt" ]; then
  echo "FAIL: Rust .so has undefined references that the C runtime does not provide:"
  sed 's/^/  - /' "$tmp/rs_undef_bad.txt"
  rc=1
else
  echo "OK: Rust .so has 0 undefined non-libc symbols."
fi

# The two functional imports the C library itself makes must be the same ones
# the Rust library makes, so allocation behaviour is bit-identical.
for s in malloc free; do
  if grep -qx "$s" "$tmp/rs_undef.txt"; then
    echo "OK: Rust .so imports libc \`$s\` (same allocator as the C library)"
  else
    echo "FAIL: Rust .so does not import libc \`$s\`; allocation behaviour may diverge"
    rc=1
  fi
done

# Eager-resolution check: RTLD_NOW fails if anything is unresolvable.
cat > "$tmp/dlcheck.c" <<'EOF'
#include <dlfcn.h>
#include <stdio.h>
int main(int argc, char **argv) {
    void *h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!h) { printf("dlopen failed: %s\n", dlerror()); return 1; }
    const char *syms[] = {"safe_double_to_int","process_array_reverse",
        "switch_fallthrough_calculator","allocate_and_compute","foreach_sum","fallcalc"};
    for (int i = 0; i < 6; i++) {
        dlerror();
        void *s = dlsym(h, syms[i]);
        const char *e = dlerror();
        if (!s || e) { printf("dlsym(%s) failed: %s\n", syms[i], e ? e : "(null)"); return 1; }
    }
    return 0;
}
EOF
if gcc -O0 -o "$tmp/dlcheck" "$tmp/dlcheck.c" -ldl 2>/dev/null; then
  for so in "$C_SO" "$RS_SO"; do
    if "$tmp/dlcheck" "$(readlink -f "$so")"; then
      echo "OK: RTLD_NOW dlopen + dlsym of all 6 symbols succeeded for $so"
    else
      echo "FAIL: RTLD_NOW dlopen/dlsym failed for $so"
      rc=1
    fi
  done
fi

exit $rc
