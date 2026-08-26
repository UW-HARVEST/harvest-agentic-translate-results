#!/usr/bin/env bash
# Phase D — symbol parity gate: every symbol the C .so exports, the Rust .so
# must export under the exact same name. Also reports Rust imports that would
# not resolve at load time.
set -uo pipefail
cd "$(dirname "$0")"

PROFILE="${1:-debug}"
C_SO=c_src/build/libStaticAlias.so
R_SO="target/$PROFILE/libStaticAlias.so"

for f in "$C_SO" "$R_SO"; do
  [[ -f $f ]] || { echo "MISSING: $f (build it first)"; exit 1; }
done

# Exported (defined) dynamic symbols, names only.
CS="${TMPDIR:-/tmp}/c_syms.txt"; RS="${TMPDIR:-/tmp}/r_syms.txt"
nm -D --defined-only "$C_SO" | awk 'NF>=3{print $3}' | sort -u > "$CS"
nm -D --defined-only "$R_SO" | awk 'NF>=3{print $3}' | sort -u > "$RS"

echo "C  .so exports : $(wc -l < "$CS")"
echo "Rust .so exports: $(wc -l < "$RS") (incl. Rust-internal/std symbols)"
echo
echo "--- C symbols ---"
cat "$CS"
echo
echo "--- MISSING from Rust .so (must be empty) ---"
MISSING=$(comm -23 "$CS" "$RS")
if [[ -n $MISSING ]]; then echo "$MISSING"; else echo "(none)"; fi
echo
echo "--- Rust undefined imports that are NOT libc/libgcc (must be empty) ---"
UNRES=$(nm -D --undefined-only "$R_SO" | awk 'NF>=2{print $NF}' \
  | sed 's/@.*//' | sort -u \
  | grep -vE '^(_ITM_|__gmon_start__|__cxa_|_Unwind_|__tls_get_addr|__errno_location|statx|gettid)' \
  | while read -r s; do
      # anything resolvable in libc/libm/libgcc/libpthread/libdl is fine
      if ! grep -qx "$s" <(nm -D --defined-only /lib64/libc.so.6 /lib64/libm.so.6 \
           /lib64/libgcc_s.so.1 2>/dev/null | awk 'NF>=3{print $3}' | sed 's/@.*//' | sort -u); then
        echo "$s"
      fi
    done)
if [[ -n $UNRES ]]; then echo "$UNRES"; else echo "(none)"; fi
echo
if [[ -z $MISSING ]]; then echo "SYMBOL PARITY: PASS"; else echo "SYMBOL PARITY: FAIL"; exit 1; fi
