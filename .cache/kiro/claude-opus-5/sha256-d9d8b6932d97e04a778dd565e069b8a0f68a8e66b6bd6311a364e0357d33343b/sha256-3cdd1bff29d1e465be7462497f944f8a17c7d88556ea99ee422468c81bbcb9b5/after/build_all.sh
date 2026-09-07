#!/bin/bash
# Build every (OP, REPEAT) configuration of both implementations.
#
#   cbuild/c/libmdcore_<op>_<r>.so   -- C shared library from src/mdcore.c
#   cbuild/c/driver_<op>_<r>         -- C executable (mdcore.c + mdmain.c)
#   cbuild/rust/libdriver_<op>_<r>.so
#   cbuild/rust/driver_<op>_<r>
#
# The upstream CMakeLists only defines add_executable(driver ...); the .so is
# produced here with an equivalent gcc invocation, leaving c_src/ untouched.
set -u
ROOT="$(cd "$(dirname "$0")" && pwd)"
CFLAGS_COMMON="-O2 -fPIC -std=c11"
mkdir -p "$ROOT/cbuild/c" "$ROOT/cbuild/rust"

OPS="${OPS:-add sub mul}"
REPS="${REPS:-0 1 2 3 4 5 6 7}"
ONLY="${ONLY:-both}"   # both | c | rust

fail=0
for op in $OPS; do
  for r in $REPS; do
    tag="${op}_${r}"

    if [ "$ONLY" != "rust" ]; then
      gcc $CFLAGS_COMMON -DOP=$op -DREPEAT=$r -shared \
        -o "$ROOT/cbuild/c/libmdcore_${tag}.so" "$ROOT/c_src/src/mdcore.c" \
        2> "$ROOT/cbuild/c/libmdcore_${tag}.log" || { echo "C .so FAIL $tag"; fail=1; }
      gcc $CFLAGS_COMMON -DOP=$op -DREPEAT=$r \
        -o "$ROOT/cbuild/c/driver_${tag}" \
        "$ROOT/c_src/src/mdcore.c" "$ROOT/c_src/src/mdmain.c" \
        2> "$ROOT/cbuild/c/driver_${tag}.log" || { echo "C exe FAIL $tag"; fail=1; }
    fi

    if [ "$ONLY" != "c" ]; then
      ( cd "$ROOT/translation" && \
        timeout 300 cargo build --release --no-default-features --features "$op,$r" \
          > "$ROOT/cbuild/rust/build_${tag}.log" 2>&1 ) \
        || { echo "Rust FAIL $tag"; fail=1; continue; }
      cp "$ROOT/translation/target/release/libdriver.so" \
         "$ROOT/cbuild/rust/libdriver_${tag}.so"
      cp "$ROOT/translation/target/release/driver" \
         "$ROOT/cbuild/rust/driver_${tag}"
    fi

    echo "built $tag"
  done
done
exit $fail
