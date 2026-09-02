#!/usr/bin/env bash
# Symbol-parity check: for every OP x REPEAT configuration, every symbol the
# C .so exports must also be exported by the Rust .so, with the exact name.
set -u
cd "$(dirname "$0")"
ROOT=..
fail=0
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT

# Only real, non-libc, non-toolchain symbols.
filter() {
  nm -D --defined-only "$1" \
    | awk '{print $3}' \
    | grep -vE '^(_init|_fini|_edata|_end|__bss_start|_ITM_|__cxa_|__gmon_|rust_|__rust_|_ZN|DW\.)' \
    | grep -vE '^$' | sort -u
}

for OP in add sub mul; do
  for R in 0 1 2 3 4 5 6 7; do
    cargo build --release --no-default-features --features "$OP,repeat_$R" >/dev/null 2>&1 || {
      echo "RUST BUILD FAIL $OP/$R"; fail=1; continue; }
    filter "$ROOT/cbuild/lib/libmd_${OP}_${R}.so" > "$tmp/c.txt"
    filter target/release/libdriver.so                > "$tmp/r.txt"
    missing=$(comm -23 "$tmp/c.txt" "$tmp/r.txt")
    if [[ -n "$missing" ]]; then
      echo "MISSING IN RUST ($OP/$R): $(echo "$missing" | tr '\n' ' ')"; fail=1
    fi
    extra=$(comm -13 "$tmp/c.txt" "$tmp/r.txt")
    [[ -n "$extra" ]] && echo "note: rust-only ($OP/$R): $(echo "$extra" | tr '\n' ' ')"
    # Undefined symbols in the Rust .so must all be provided by the platform
    # runtime (glibc, libgcc_s unwinder, loader weak hooks).
    und=$(nm -D -u target/release/libdriver.so | awk '{print $2}' \
          | grep -vE '@GLIBC|@GCC_|^_ITM_|^__cxa_finalize$|^__gmon_start__$|^$' || true)
    if [[ -n "$und" ]]; then
      echo "UNRESOLVED NON-LIBC IN RUST ($OP/$R): $(echo "$und" | tr '\n' ' ')"; fail=1
    fi
    # And the dynamic loader must be able to resolve every one of them.
    if ldd -r target/release/libdriver.so 2>&1 | grep -q 'undefined symbol'; then
      echo "LOADER UNRESOLVED ($OP/$R):"; ldd -r target/release/libdriver.so 2>&1 \
        | grep 'undefined symbol' | sed 's/^/    /'; fail=1
    fi
  done
done

if [[ $fail -eq 0 ]]; then echo "SYMBOL PARITY OK for all 24 configurations"; fi
exit $fail
