#!/usr/bin/env bash
# Phase D: every dynamic symbol the C .so exports must also be exported by the
# Rust .so, with the exact same name. Exits non-zero if the diff is non-empty.
set -uo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
crate="$(dirname -- "$here")"
root="$(dirname -- "$crate")"

profile="${PROFILE:-release}"

c_build="$root/c_src/build"
if [ ! -d "$c_build" ]; then
    echo "building the C shared library ..."
    mkdir -p "$c_build"
    ( cd "$c_build" && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON >/dev/null && cmake --build . >/dev/null )
fi

# The CMake project is named after the parent directory, so glob for the .so.
mapfile -t c_sos < <(find "$c_build" -maxdepth 1 -name 'lib*.so' -print | sort)
if [ "${#c_sos[@]}" -ne 1 ]; then
    echo "FAIL: expected exactly one lib*.so in $c_build, found ${#c_sos[@]}" >&2
    exit 1
fi
c_so="${c_sos[0]}"

rust_so="$crate/target/$profile/libcolourblind_lib.so"
if [ ! -f "$rust_so" ]; then
    echo "building the Rust cdylib ($profile) ..."
    if [ "$profile" = "release" ]; then
        ( cd "$crate" && cargo build --release >/dev/null )
    else
        ( cd "$crate" && cargo build >/dev/null )
    fi
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

nm -D --defined-only "$c_so"    | awk '{print $NF}' | sort -u > "$tmp/c.txt"
nm -D --defined-only "$rust_so" | awk '{print $NF}' | sort -u > "$tmp/rust.txt"

echo "C    .so: $c_so   ($(wc -l < "$tmp/c.txt") defined dynamic symbols)"
echo "Rust .so: $rust_so   ($(wc -l < "$tmp/rust.txt") defined dynamic symbols)"

missing="$(comm -23 "$tmp/c.txt" "$tmp/rust.txt")"
extra="$(comm -13 "$tmp/c.txt" "$tmp/rust.txt")"

status=0
if [ -n "$missing" ]; then
    echo
    echo "FAIL: exported by C but MISSING from Rust:"
    echo "$missing" | sed 's/^/  - /'
    status=1
fi
if [ -n "$extra" ]; then
    echo
    echo "note: exported by Rust but not by C (surface widening):"
    echo "$extra" | sed 's/^/  + /'
fi

# Undefined, non-weak, non-libc imports in the Rust .so would mean an
# untranslated dependency.
undef="$(nm -D -u "$rust_so" | awk '{print $NF}' \
    | grep -vE '^(__|_ITM_|_Unwind_|GLIBC)' \
    | grep -vE '@GLIBC' | sort -u || true)"
if [ -n "$undef" ]; then
    echo
    echo "FAIL: Rust .so has undefined non-libc symbols:"
    echo "$undef" | sed 's/^/  ? /'
    status=1
fi

if [ "$status" -eq 0 ]; then
    echo
    echo "PASS: symbol parity — 0 missing, 0 undefined non-libc symbols."
fi
exit "$status"
