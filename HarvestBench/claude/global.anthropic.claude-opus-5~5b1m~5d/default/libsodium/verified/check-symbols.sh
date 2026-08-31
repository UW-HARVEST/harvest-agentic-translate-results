#!/bin/sh
# Phase D: exported-symbol parity between the C reference .so and the Rust .so.
# Exits non-zero if the Rust .so is missing ANY symbol the C .so exports, or if
# the Rust .so has an undefined symbol that is not a libc/libgcc import.
set -eu
cd "$(dirname "$0")"

C_SO=${SODIUM_C_SO:-../c_src/build/libsodium.so}
R_SO=${SODIUM_RUST_SO:-target/release/liblibsodium.so}

[ -f "$C_SO" ] || { echo "missing $C_SO — build c_src first" >&2; exit 2; }
[ -f "$R_SO" ] || { echo "missing $R_SO — run 'cargo build --offline --release' first" >&2; exit 2; }

W=$(mktemp -d)
trap 'rm -rf "$W"' EXIT

syms() { nm -D --defined-only "$1" | awk '$2~/^[TBDR]$/{print $3}' | sort -u; }
syms "$C_SO" > "$W/c"
syms "$R_SO" > "$W/r"
comm -23 "$W/c" "$W/r" > "$W/missing"
comm -13 "$W/c" "$W/r" > "$W/extra"

nm -D --undefined-only "$R_SO" | awk '{print $2}' \
  | grep -vE '@GLIBC|@GCC|^_ITM_|^__gmon_start__$|^_Unwind_' > "$W/undef" || true

printf 'C exported:            %s\n' "$(wc -l < "$W/c")"
printf 'Rust exported:         %s\n' "$(wc -l < "$W/r")"
printf 'missing from Rust:     %s\n' "$(wc -l < "$W/missing")"
printf 'extra in Rust:         %s\n' "$(wc -l < "$W/extra")"
printf 'undefined non-libc:    %s\n' "$(wc -l < "$W/undef")"

rc=0
if [ -s "$W/missing" ]; then
  echo; echo 'MISSING FROM THE RUST .so:'; cat "$W/missing"; rc=1
fi
if [ -s "$W/undef" ]; then
  echo; echo 'UNDEFINED NON-LIBC SYMBOLS IN THE RUST .so:'; cat "$W/undef"; rc=1
fi
[ "$rc" -eq 0 ] && echo && echo 'SYMBOL PARITY OK (diff is empty)'
exit "$rc"
