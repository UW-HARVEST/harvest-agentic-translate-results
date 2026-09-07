#!/bin/bash
# Symbol-parity check: every dynamic symbol the C .so defines must also be
# defined by the Rust .so, with the identical name and symbol class (T/D).
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
OPS="${OPS:-add sub mul}"
REPS="${REPS:-0 1 2 3 4 5 6 7}"
rc=0

# Symbols that belong to the Rust/std runtime rather than to the translated C.
RUST_RT='_ZN|^rust_|_Unwind|__rdl_|__rust_|^_R'

for op in $OPS; do
  for r in $REPS; do
    tag="${op}_${r}"
    cso="$ROOT/cbuild/c/libmdcore_${tag}.so"
    rso="$ROOT/cbuild/rust/libdriver_${tag}.so"
    nm -D --defined-only "$cso" | awk '{print $2, $3}' | sort > "/tmp/csym_$tag"
    nm -D --defined-only "$rso" | awk '{print $2, $3}' | sort > "/tmp/rsym_$tag"
    missing=$(comm -23 "/tmp/csym_$tag" "/tmp/rsym_$tag")
    if [ -n "$missing" ]; then
      echo "[$tag] MISSING FROM RUST:"; echo "$missing"; rc=1
    fi
    # Undefined (non-weak, non-libc) symbols in the Rust .so would mean an
    # unresolved translation.
    undef=$(nm -D --undefined-only "$rso" | awk '$1=="U"{print $2}' \
            | grep -vE "$RUST_RT" | grep -vE '@GLIBC|@GCC|^__' )
    if [ -n "$undef" ]; then
      echo "[$tag] UNRESOLVED IN RUST:"; echo "$undef"; rc=1
    fi
    [ -z "$missing" ] && [ -z "$undef" ] && echo "[$tag] symbol parity OK ($(wc -l < /tmp/csym_$tag) C symbols)"
  done
done
exit $rc
