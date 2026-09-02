#!/usr/bin/env bash
# Phase D: symbol parity between the C .so and the Rust .so.
# The exported-symbol diff MUST be empty, and the Rust .so must have no
# undefined symbol that is not provided by libc / libgcc / the loader.
set -uo pipefail
cd "$(dirname "$0")/../.."   # working-directory root

C_SO=c_src/build/libdriver.so
R_SO=${RUST_DRIVER_SO:-translation/target/release/libdriver.so}

for f in "$C_SO" "$R_SO"; do
  [ -f "$f" ] || { echo "missing $f"; exit 1; }
done

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only --extern-only "$C_SO" | awk '{print $NF}' | sort -u > "$tmp/c.syms"
nm -D --defined-only --extern-only "$R_SO" | awk '{print $NF}' | sort -u > "$tmp/r.syms"

echo "C   exports: $(wc -l < "$tmp/c.syms")"
echo "Rust exports: $(wc -l < "$tmp/r.syms")"
echo
echo "--- exported by C but MISSING from Rust (must be empty) ---"
comm -23 "$tmp/c.syms" "$tmp/r.syms" | tee "$tmp/missing"
echo "--- exported by Rust but not by C (informational) ---"
comm -13 "$tmp/c.syms" "$tmp/r.syms"
echo

# static (internal-linkage) C functions must NOT be exported by Rust either.
echo "--- C internal-linkage functions that Rust wrongly exports (must be empty) ---"
nm "$C_SO" | awk '$2=="t" {print $NF}' | sort -u > "$tmp/c.local"
comm -12 "$tmp/c.local" "$tmp/r.syms" | tee "$tmp/leaked"
echo

echo "--- Rust undefined symbols not resolvable from libc/libgcc/loader (must be empty) ---"
nm -D -u "$R_SO" | awk '{print $NF}' | sed 's/@.*//' | sort -u > "$tmp/r.undef"
# Everything the Rust std runtime imports comes from the NEEDED libraries.
: > "$tmp/provided"
for lib in $(readelf -d "$R_SO" | awk -F'[][]' '/NEEDED/ {print $2}'); do
  for cand in "/lib64/$lib" "/usr/lib64/$lib" "/lib/x86_64-linux-gnu/$lib" "/usr/lib/x86_64-linux-gnu/$lib"; do
    [ -f "$cand" ] && nm -D --defined-only "$cand" 2>/dev/null | awk '{print $NF}' | sed 's/@.*//' >> "$tmp/provided"
  done
done
sort -u "$tmp/provided" -o "$tmp/provided"
comm -23 "$tmp/r.undef" "$tmp/provided" | grep -vE '^(_ITM_|__gmon_start__|__cxa_|statx|gettid)' | tee "$tmp/unresolved"
echo

rc=0
[ -s "$tmp/missing" ]    && { echo "FAIL: Rust .so is missing C exports";            rc=1; }
[ -s "$tmp/leaked" ]     && { echo "FAIL: Rust .so exports C-static symbols";        rc=1; }
[ -s "$tmp/unresolved" ] && { echo "FAIL: Rust .so has unresolved non-libc symbols"; rc=1; }
[ "$rc" -eq 0 ] && echo "SYMBOL PARITY: OK (diff empty, no leaked statics, no unresolved non-libc symbols)"
exit "$rc"
