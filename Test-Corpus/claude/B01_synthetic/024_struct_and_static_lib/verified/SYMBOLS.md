# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C `.so`:    `c_src/build/libdriver.so` (cmake, default build type, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust `.so`: `target/debug/libdriver.so` (`cargo build`, `crate-type = ["cdylib"]`)

Regenerate with:

```sh
nm -D --defined-only c_src/build/libdriver.so | sort
nm -D --defined-only target/debug/libdriver.so | sort
```

## Defined (exported) dynamic symbols

| symbol | C `.so` | Rust `.so` | C declaration | notes |
|--------|---------|------------|---------------|-------|
| `driver` | T (defined) | T (defined) | `void driver(int x);` (`c_src/include/driver.h:27`) | the only symbol in the public header |
| `run`    | T (defined) | T (defined) | `void run(int extra_bedrooms);` (`c_src/src/driver.c:53`) | not declared in `driver.h`, but non-`static` so it is part of the exported ABI; the Rust side exports it too |

**Symbol diff (C-defined not exported by Rust): EMPTY.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only target/debug/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

## C symbols that are deliberately *not* exported

These are `static` in `c_src/src/driver.c`, therefore absent from `nm -D` on the C
`.so`, and correspondingly private (module-local `fn`) in the Rust translation.
They are *not* completeness gaps:

| C entity | C site | Rust counterpart |
|----------|--------|------------------|
| `house_t` (typedef struct) | `driver.c:29-33` | `struct HouseT` (`src/driver.rs`, `#[repr(C)]`) |
| `static house_t the_house` | `driver.c:35` | `static THE_HOUSE: TheHouse` (interior-mutable singleton) |
| `static void add_floor(house_t*)` | `driver.c:37` | `unsafe fn add_floor(*mut HouseT)` |
| `static void add_bedrooms(house_t*, int)` | `driver.c:41` | `unsafe fn add_bedrooms(*mut HouseT, c_int)` |
| `static void add_floor_to_the_house()` | `driver.c:45` | `fn add_floor_to_the_house()` |
| `static void print_the_house()` | `driver.c:49` | `fn print_the_house()` |

## Undefined (imported) symbols

The C `.so` imports exactly one non-weak libc symbol, `printf@GLIBC_2.2.5`, plus
the usual weak toolchain symbols (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports `printf@GLIBC_2.2.5` as well — the translation calls the
*same* libc entry point, so `%d` / `%.1f` formatting and stdout buffering are
byte-identical — plus libc/`_Unwind_*` symbols pulled in by the Rust standard
library (`malloc`, `memcpy`, `write`, `dl_iterate_phdr`, …).

**0 missing / undefined non-libc symbols in the Rust `.so`.** Every undefined
symbol in the Rust `.so` is either libc (glibc-versioned) or the platform unwinder
that ships with the Rust runtime; none is an untranslated function from `c_src/`.

## Source-file coverage

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source | translated to | status |
|----------|---------------|--------|
| `c_src/src/driver.c` | `src/driver.rs` (declared `mod driver;` in `src/lib.rs`) | complete — all 6 functions + the file-scope singleton |
| `c_src/include/driver.h` | (header only, no code) | complete |

No C source file in `c_src/` is untranslated, so no module had to be written to
close a symbol gap.
