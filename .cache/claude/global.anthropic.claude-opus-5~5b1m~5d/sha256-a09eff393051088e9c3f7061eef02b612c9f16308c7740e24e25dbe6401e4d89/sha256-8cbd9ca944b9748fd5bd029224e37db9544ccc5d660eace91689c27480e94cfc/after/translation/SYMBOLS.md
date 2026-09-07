# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## C `.so` (`c_src/build/libdriver.so`)

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001173 T driver
```

| # | symbol | type | in Rust `.so`? | notes |
|---|--------|------|----------------|-------|
| 1 | `driver` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub extern "C" fn driver(x: f32)` in `src/lib.rs` |

## Rust `.so` (`translation/target/release/libdriver.so`)

```
$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011720 T driver
```

## Non-exported C symbols (intentionally NOT in the dynamic table)

| C symbol | linkage | Rust counterpart | exported? |
|----------|---------|------------------|-----------|
| `print_hex` | `static void print_hex(unsigned char *p, int len)` | `unsafe fn print_hex(p: *const c_uchar, len: c_int)` (private) | No — matches C `static` |

## Undefined (imported) symbols

| `.so` | undefined non-libc symbols |
|-------|----------------------------|
| C | none (only `printf`, `putchar`/`puts` family + libc startup from glibc) |
| Rust | none (only `printf` + libc/`std` runtime from glibc) |

## Diff result

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $3}' | sort) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
=> (empty)
```

**Status: symbol parity achieved — 0 missing, 0 extra, 0 undefined non-libc symbols.**

Re-verified for BOTH cargo profiles after every fix:

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so            | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/difftest/debug/libdriver.so   | awk '{print $3}' | sort)
SYMBOL DIFF EMPTY
$ diff <(nm -D --defined-only c_src/build/libdriver.so            | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/difftest/release/libdriver.so | awk '{print $3}' | sort)
SYMBOL DIFF EMPTY
```

`nm -D --undefined-only` on the Rust `.so` lists only glibc/libgcc imports
(`printf`, `putchar`, `malloc`, `memcpy`, `_Unwind_*`, ...) — **0 undefined
non-libc symbols**. This is enforced as a test in `tests/phase_d_symbols.rs`
(`every_c_symbol_is_exported_by_rust`, `no_undefined_non_libc_symbols_in_rust`,
`c_source_tree_is_fully_translated`).

No C source file / module was left untranslated: `c_src` contains exactly one
translation unit (`src/driver.c`, 37 lines incl. the 22-line license header)
plus one public header (`include/driver.h`).
