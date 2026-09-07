# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C:    `c_src/build/libdriver.so`   (cmake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
- Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)

## C translation units

`c_src/CMakeLists.txt` builds exactly one source file: `src/driver.c`.
There are **no untranslated C modules** — `translation/src/lib.rs` covers the
whole library.

## `nm -D` on the C `.so` (defined symbols only, `T`/`D`/`B`/`W` local)

| symbol | type | in Rust `.so`? | notes |
|--------|------|----------------|-------|
| `driver` | `T` (global text) | YES (`T driver`) | declared in `include/driver.h` |
| `run`    | `T` (global text) | YES (`T run`)    | **not** in the public header, but non-`static` in `driver.c`, therefore an exported public symbol; Rust must export it too |

## Undefined (imported) symbols in the C `.so`

| symbol | notes |
|--------|-------|
| `printf@GLIBC_2.2.5` | libc; Rust `.so` also imports `printf` (the translation calls C `printf` directly so formatting/buffering is byte-identical) |

## Toolchain-generated weak symbols (ignored for parity)

Present in one or both objects purely as artifacts of the linker / runtime, not
part of the library's API surface:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`,
and, Rust-side only, `__cxa_thread_atexit_impl@GLIBC_2.18`,
`gettid@GLIBC_2.30`, `statx@GLIBC_2.28` (weak libc-version probes emitted by
the Rust std runtime).

## Static (internal, non-exported) C symbols — reproduced as private Rust items

These are `static` in `driver.c` and must NOT appear in `nm -D` of either object.
Verified absent from both.

| C internal | Rust counterpart |
|------------|------------------|
| `house_t` (typedef) | `struct house_t` (`#[repr(C)]`) |
| `static house_t the_house = {2, 5, 2.5}` | `static mut THE_HOUSE` |
| `static void add_floor(house_t *)` | `unsafe fn add_floor` |
| `static void add_bedrooms(house_t *, int)` | `unsafe fn add_bedrooms` |
| `static void add_floor_to_the_house()` | `unsafe fn add_floor_to_the_house` |
| `static void print_the_house()` | `unsafe fn print_the_house` |

## Result

Symbol diff (C defined-global symbols minus Rust defined-global symbols): **EMPTY**.

```
$ comm -23 <(nm -D c_src/build/libdriver.so     | awk '$2=="T"{print $3}' | sort) \
           <(nm -D translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort)
(no output)
```

- [x] `nm -D` shows 0 missing / undefined non-libc symbols in Rust.
