# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D -u   <each>
ldd -r     translation/target/release/libdriver.so
```

## C source inventory

The whole library is two files:

| file | contents |
|------|----------|
| `c_src/include/driver.h` | one declaration: `void driver(int x);` |
| `c_src/src/driver.c` | `house_t` (file-local typedef), `static void print_hex(unsigned char*, int)`, `void driver(int floors)` |

`print_hex` is `static`, so it is deliberately **not** part of the exported ABI
and must NOT appear in `nm -D` for either library. `house_t` is a typedef and
emits no symbol. There is no macro-generated symbol machinery, no namespace
prefix macro, no `#ifdef`-gated alternate implementation, and no second
translation unit. Therefore the complete expected export set is exactly one
name: `driver`.

No C source file was left untranslated: `driver.c` is the only `.c` file
referenced by `add_library(driver SHARED src/driver.c)` in `CMakeLists.txt`.

## Exported (defined, dynamic) symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | `T driver` | `T driver` | PRESENT in both |

Symbol diff (`comm -23` of the two sorted defined-symbol lists): **empty**.
Reverse diff (Rust exports that C does not): **empty**.

## Intentionally absent symbols

| symbol | reason it must not be exported |
|--------|-------------------------------|
| `print_hex` | `static` in C — internal linkage. Kept private (`fn print_hex`) in Rust. |

Verified absent from both `.so` files.

## Undefined (imported) symbols

The C `.so` imports `printf` and `putchar` from glibc, plus the four standard
weak CRT/ITM hooks.

The Rust `.so` imports the same `printf` and `putchar`, plus the Rust standard
library's runtime imports (`_Unwind_*` from libgcc, and glibc `malloc`,
`memcpy`, `write`, `pthread_key_*`, … ). These are all libc / libgcc runtime
symbols, not symbols that should have been defined by this crate.

`ldd -r translation/target/release/libdriver.so` reports **no** undefined
symbols — every import resolves.

## Completion gate for this file

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the identical name.
- [x] 0 missing symbols.
- [x] 0 undefined non-libc/non-libgcc symbols in the Rust `.so`.
- [x] No stubbed / `unimplemented!()` export was added to satisfy the diff.
