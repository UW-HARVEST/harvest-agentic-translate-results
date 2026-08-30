# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libStaticLoop.so

cd translation && cargo build --release
# -> translation/target/release/libStaticLoop.so
```

## C source inventory

The entire library is two translation units' worth of surface:

| C file | public functions defined |
|---|---|
| `c_src/src/staticloop.c` | `static_sum`, `driver` |
| `c_src/include/staticloop.h` | declares `int static_sum(int update);` and `void driver(int update);` |

There are no other `.c` files, no macro-generated symbol families, no global
variables with external linkage, and no additional headers. `sum` inside
`static_sum` is a function-scope `static int` and therefore has **no** external
linkage — it must NOT appear in `nm -D` on either side (confirmed below).

## `nm -D --defined-only` — C `.so`

```
0000000000001139 T driver
0000000000001119 T static_sum
```

## `nm -D --defined-only` — Rust `.so`

```
0000000000011730 T driver
0000000000011850 T static_sum
```

## Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | status |
|---|--------|------|-----------|--------------|--------|
| 1 | `static_sum` | `T` (global text) | yes | yes | MATCH |
| 2 | `driver`     | `T` (global text) | yes | yes | MATCH |

**Symbols exported by C but missing from Rust: 0.**
**Symbols exported by Rust but not by C: 0.**

No `#[no_mangle]` wrapper had to be added and no C module was left
untranslated — `staticloop.c` is the only implementation file and both of its
functions are present in `translation/src/lib.rs`.

## Undefined (imported) symbols

The Rust `.so` must not depend on any non-libc symbol. `nm -D --undefined-only`
on the Rust `.so` resolves entirely against `libc`/`libgcc` (`printf`,
`memcpy`, unwinder/`__cxa` personality stubs, `__libc_start_main` family). The
Rust translation deliberately calls C's `printf` via `extern "C"` rather than
Rust's `std::io::stdout`, so `driver`'s bytes land in the *same* stdio stream,
with the same buffering discipline, as the C original's.

See `check_symbols.sh` in the crate root for the automated diff that must
produce empty output.
