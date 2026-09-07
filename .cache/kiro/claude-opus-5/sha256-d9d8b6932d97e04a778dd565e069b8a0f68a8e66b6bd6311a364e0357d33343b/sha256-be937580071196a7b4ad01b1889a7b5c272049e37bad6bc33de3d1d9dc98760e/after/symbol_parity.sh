#!/usr/bin/env bash
# Symbol-parity + undefined-symbol gate for every feature combination.
#   ./symbol_parity.sh
#
# Parity:    every symbol nm -D reports for the C .so must also be reported for
#            the Rust .so, and vice versa.
# Undefined: every symbol the Rust .so imports must be resolvable in the system
#            libraries it links (libc / libgcc_s / ld-linux) or be a weak CRT
#            hook. Anything left over would mean a piece of the translation is
#            missing rather than merely unexported.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Build the set of symbols the platform provides, straight from the .so files the
# loader actually uses -- no hand-maintained allowlist.
SYS_SYMS="$(mktemp)"
trap 'rm -f "$SYS_SYMS"' EXIT
for lib in /lib64/libc.so.6 /lib64/libgcc_s.so.1 /lib64/ld-linux-x86-64.so.2 \
           /lib64/libm.so.6 /lib64/libpthread.so.0 /lib64/libdl.so.2; do
  [[ -e "$lib" ]] && nm -D --defined-only "$lib" 2>/dev/null | awk '{print $NF}' | sed 's/@.*//'
done | sort -u > "$SYS_SYMS"
echo "platform provides $(wc -l < "$SYS_SYMS") dynamic symbols"

fail=0
for op in add sub mul; do
  for rep in 0 1 2 3 4 5 6 7; do
    out="$ROOT/cbuild/${op}_${rep}"
    "$ROOT/build_c.sh" "$op" "$rep" >/dev/null || { echo "!!!! C build $op,$rep"; fail=1; continue; }
    ( cd "$ROOT/translation" && timeout 600 cargo build --release \
        --no-default-features --features "$op,$rep" >/dev/null 2>&1 ) \
      || { echo "!!!! rust build $op,$rep"; fail=1; continue; }
    cp "$ROOT/translation/target/release/libdriver.so" "$out/libdriver.so"

    c_syms="$(nm -D --defined-only "$out/libmdcore.so" | awk '{print $NF}' | sort -u)"
    r_syms="$(nm -D --defined-only "$out/libdriver.so"  | awk '{print $NF}' | sort -u)"
    missing="$(comm -23 <(echo "$c_syms") <(echo "$r_syms") | tr '\n' ' ')"
    extra="$(comm -13 <(echo "$c_syms") <(echo "$r_syms") | tr '\n' ' ')"

    # Strong (non-weak) undefined symbols not provided by the platform.
    undef="$(nm -D --undefined-only "$out/libdriver.so" \
             | awk '$1 != "w" {print $NF}' | sed 's/@.*//' | sort -u \
             | comm -23 - "$SYS_SYMS" | tr '\n' ' ')"

    status="parity OK, undefined OK"
    if [[ -n "${missing// /}" ]]; then status="MISSING: $missing"; fail=1; fi
    if [[ -n "${extra// /}" ]];   then status="$status EXTRA: $extra"; fail=1; fi
    if [[ -n "${undef// /}" ]];   then status="$status UNRESOLVED-NONLIBC: $undef"; fail=1; fi
    printf '%-8s C=%d Rust=%d  %s\n' "$op,$rep" \
      "$(echo "$c_syms" | wc -l)" "$(echo "$r_syms" | wc -l)" "$status"
  done
done

[[ $fail -eq 0 ]] && echo "SYMBOL PARITY: clean for all 24 configurations" \
                  || echo "SYMBOL PARITY: FAILURES"
exit "$fail"
