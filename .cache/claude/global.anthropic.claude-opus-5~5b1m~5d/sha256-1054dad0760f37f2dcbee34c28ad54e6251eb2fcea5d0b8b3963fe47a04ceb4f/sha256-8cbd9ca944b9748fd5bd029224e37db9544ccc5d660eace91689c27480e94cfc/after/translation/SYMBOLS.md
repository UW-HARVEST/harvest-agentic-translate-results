# SYMBOLS.md — Phase A / Phase D symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Commands

```
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release

nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C `.so` exported (defined) symbols

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `driver` | `T` (global text) | YES — `#[no_mangle] pub extern "C" fn driver` |

Raw output:

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001173 T driver

$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011720 T driver
```

## Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so        | awk '{print $2, $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $2, $3}' | sort)
(empty)
```

**Diff is EMPTY.** 0 missing symbols, 0 name mismatches.

## Non-exported C entities (correctly private in Rust)

| C entity | linkage | Rust counterpart | exported? |
|----------|---------|------------------|-----------|
| `print_hex(unsigned char *, int)` | `static` (file-local) | `fn print_hex` (private) | no — matches C |
| `house_t` (`typedef struct`) | type only, no linkage | `struct HouseT` (`#[repr(C)]`, private) | no — matches C |

No C source file/module was skipped: `c_src` contains exactly
`include/driver.h` (1 declaration) and `src/driver.c` (1 external function,
1 static function, 1 struct type). All are represented in `translation/src/lib.rs`.
No stubs, no `unimplemented!()`.

## Undefined (imported) symbols

Rust `.so` imports `printf` from libc, exactly like the C `.so`, so the
`stdout` FILE stream and its buffering are shared/identical:

```
$ nm -D --undefined-only translation/target/release/libdriver.so | grep -i printf
                 U printf@GLIBC_2.2.5
$ nm -D --undefined-only c_src/build/libdriver.so | grep -i printf
                 U printf@GLIBC_2.2.5
```

The C `.so`'s full undefined list is
`_ITM_{de,}registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__` (all weak),
`printf`, `putchar` — the last because GCC rewrites `printf("\n")` into
`putchar('\n')`.

The Rust `.so`'s undefined list is the same weak set plus `printf`, `putchar`
and the standard glibc / `libgcc` unwinder imports pulled in by `libstd`
(`_Unwind_*`, `malloc`, `free`, `memcpy`, `write`, `dl_iterate_phdr`, …).
Every one of them resolves out of `libc` / `libgcc_s`:

```
$ ldd -r translation/target/release/libdriver.so | grep -c 'undefined symbol'
0
```

**0 undefined non-libc symbols in the Rust `.so`.**
