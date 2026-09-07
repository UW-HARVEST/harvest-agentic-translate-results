# Phase A.1 — Symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-Xa61pS.so
cd translation && cargo build --release
# -> translation/target/release/libldexp_q2_lib.so
```

## C `.so` defined dynamic symbols (excluding weak/undefined libc/CRT glue)

```
$ nm -D --defined-only c_src/build/libharvest-work-Xa61pS.so | grep -v ' [wWuU] '
00000000000010f9 T ldexp_q2
```

## Rust `.so` defined dynamic symbols (excluding weak/undefined)

```
$ nm -D --defined-only translation/target/release/libldexp_q2_lib.so | grep -v ' [wWuU] '
0000000000011690 T ldexp_q2
```

## Parity table

| # | C symbol | type | present in Rust `.so` | notes |
|---|----------|------|-----------------------|-------|
| 1 | `ldexp_q2` | `T` (global text) | YES (`T`) | `#[unsafe(no_mangle)] pub extern "C" fn ldexp_q2(y: c_float, exp_q2: c_int) -> c_float` |

## Symbol diff

```
$ diff <(nm -D --defined-only <C.so>  | grep -v ' [wWuU] ' | awk '{print $3}' | sort) \
       <(nm -D --defined-only <RS.so> | grep -v ' [wWuU] ' | awk '{print $3}' | sort)
(empty)
```

**Result: 0 missing symbols. No untranslated C module.** The whole C library is
one translation unit (`c_src/src/lib.c`, 12 lines) exposing exactly one function
via `c_src/include/lib.h` (1 line). `g_expfrac` is a function-local
`static const` and therefore has no external linkage in C either — it is
correctly a private `const` in Rust and must NOT be exported.

Extra symbols exported by the Rust `.so` (`_init`, `_fini`, `__bss_start`,
`_edata`, `_end`, and the Rust `std` allocator/panic glue) are toolchain
artifacts, not part of the API surface, and are not required to match.
