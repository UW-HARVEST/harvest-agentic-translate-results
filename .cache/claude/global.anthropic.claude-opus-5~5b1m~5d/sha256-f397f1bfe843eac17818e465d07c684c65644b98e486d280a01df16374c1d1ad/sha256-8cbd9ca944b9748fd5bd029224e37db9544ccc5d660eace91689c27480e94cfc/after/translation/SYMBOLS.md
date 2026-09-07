# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C `.so` (`c_src/build/libdriver.so`)

```
$ nm -D --defined-only c_src/build/libdriver.so
00000000000011e2 T decode_base64
```

| # | symbol | type | in C `.so` | in Rust `.so` | notes |
|---|--------|------|-----------|---------------|-------|
| 1 | `decode_base64` | `T` (global text) | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn` |

## Rust `.so` (`translation/target/release/libdriver.so`)

```
$ nm -D --defined-only translation/target/release/libdriver.so | grep -v ' [aAbBdD] '
00000000000116e0 T decode_base64
```

## Internal (non-exported) C functions

These are `static` in `c_src/src/lib.c`, therefore **not** part of the ABI and
correctly *not* exported by the Rust `.so`. They are still translated and
exercised indirectly through `decode_base64`.

| C symbol | linkage | Rust counterpart |
|----------|---------|------------------|
| `decode(char c)` | `static unsigned char` | `fn decode(c: c_char) -> u8` (private) |
| `is_base64(char c)` | `static int` | `fn is_base64(c: c_char) -> c_int` (private) |

## Undefined / imported symbols

The Rust `.so` imports only libc symbols (`calloc`, `malloc`, `free`, `strlen`
plus the Rust runtime's own libc/`ld` needs). No non-libc symbol is undefined.

```
$ nm -D --undefined-only translation/target/release/libdriver.so
# only libc / ld-linux entries (calloc, malloc, free, strlen, memcpy, ...)
```

## Result

**Symbol diff (C exports not present in Rust): EMPTY.**
No module of `c_src` was left untranslated: `c_src/src/lib.c` is the only
translation unit and all three of its functions (1 public + 2 `static`) have
Rust counterparts. No stubs, no `unimplemented!()`.

## Verification run (Phase D)

```
$ nm -D --defined-only c_src/build/libdriver.so       | awk '$2=="T"{print $3}'
decode_base64
$ nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}'
decode_base64
$ comm -23 <(c syms) <(rust syms)
(empty)
```

- `tests/phase_d_symbols.rs::symbol_parity_c_subset_of_rust` — **PASS**
- `tests/phase_d_symbols.rs::rust_so_has_no_unresolved_nonlibc_symbols` — **PASS**
  (the Rust `.so`'s only undefined symbols are `@GLIBC_*`-versioned libc entries)

**0 missing, 0 undefined non-libc symbols.**
