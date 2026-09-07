#!/usr/bin/env bash
# Process-level stdout comparison.
#
# The project builds no binary driver (c_src/CMakeLists.txt has only
# add_library; Cargo.toml declares only crate-type = ["cdylib"]). This script
# supplies the equivalent: a tiny external C consumer (scripts/loader.c) that
# dlopens ONE libdriver.so, calls driver() over a workload, and exits WITHOUT an
# explicit fflush -- so the at-exit flush behaviour is part of what is compared.
# The same loader is run against the C .so and the Rust .so and the two stdout
# streams are diffed byte-for-byte.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname "$here")"
root="$(dirname "$crate")"

C_SO="${C_DRIVER_SO:-$root/c_src/build/libdriver.so}"
RUST_SO="${RUST_DRIVER_SO:-$crate/target/release/libdriver.so}"
work="${TMPDIR:-/tmp}/driver-proc-cmp.$$"
mkdir -p "$work"
trap 'rm -rf "$work"' EXIT

[[ -f "$C_SO"    ]] || { echo "missing C .so: $C_SO" >&2; exit 1; }
[[ -f "$RUST_SO" ]] || { echo "missing Rust .so: $RUST_SO" >&2; exit 1; }

cc -O2 -o "$work/loader" "$here/loader.c" -ldl

# Deterministic workload: boundaries, single bits, per-lane byte sweeps, and a
# seeded pseudo-random tail.
build_workload() {
  python3 - <<'PY'
vals = [0, 1, -1, 2147483647, -2147483648, 2, -2, 255, 256, 65535, 65536,
        0x0f0f0f0f, -0x0f0f0f0f, 0x80808080 - (1 << 32), 0x7f7f7f7f]
vals += [(1 << k) - (1 << 32) if k == 31 else (1 << k) for k in range(32)]
for lane in range(4):
    for b in range(256):
        u = b << (8 * lane)
        vals.append(u - (1 << 32) if u >= (1 << 31) else u)
s = 0x5EED1234ABCDEF01
for _ in range(4000):
    s = (s + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
    z = s
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
    z ^= z >> 31
    u = (z >> 32) & 0xFFFFFFFF
    vals.append(u - (1 << 32) if u >= (1 << 31) else u)
print(" ".join(str(v) for v in vals))
PY
}

read -r -a WORKLOAD <<< "$(build_workload)"
echo "workload: ${#WORKLOAD[@]} values"

# argv has a length limit; run in chunks and concatenate, so the comparison also
# covers many separate process lifetimes.
: > "$work/c.out"
: > "$work/rust.out"
chunk=500
for ((i = 0; i < ${#WORKLOAD[@]}; i += chunk)); do
  slice=("${WORKLOAD[@]:i:chunk}")
  "$work/loader" "$C_SO"    "${slice[@]}" >> "$work/c.out"
  "$work/loader" "$RUST_SO" "${slice[@]}" >> "$work/rust.out"
done

if cmp -s "$work/c.out" "$work/rust.out"; then
  echo "PASS: process stdout identical ($(wc -c < "$work/c.out") bytes, $(wc -l < "$work/c.out") records)"
else
  echo "FAIL: process stdout differs" >&2
  diff <(head -50 "$work/c.out") <(head -50 "$work/rust.out") | head -40 >&2
  cmp "$work/c.out" "$work/rust.out" >&2 || true
  exit 1
fi

# Also compare when stdout is a PIPE (line-buffered vs fully-buffered paths in
# libc differ from the file case above).
c_pipe="$("$work/loader" "$C_SO"    0 -1 305419896 -2147483648 2147483647 | cat)"
r_pipe="$("$work/loader" "$RUST_SO" 0 -1 305419896 -2147483648 2147483647 | cat)"
[[ "$c_pipe" == "$r_pipe" ]] && echo "PASS: process stdout identical through a pipe" || {
  echo "FAIL: pipe stdout differs" >&2; exit 1; }

# And when stdout is a TTY-less /dev/null (exit code parity).
"$work/loader" "$C_SO"    42 > /dev/null; c_rc=$?
"$work/loader" "$RUST_SO" 42 > /dev/null; r_rc=$?
[[ "$c_rc" == "$r_rc" ]] && echo "PASS: exit codes identical ($c_rc)" || {
  echo "FAIL: exit codes differ ($c_rc vs $r_rc)" >&2; exit 1; }
