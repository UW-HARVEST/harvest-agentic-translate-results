# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported (defined) dynamic symbols

`nm -D --defined-only c_src/build/libdriver.so`

| # | symbol | type | present in Rust `.so`? |
|---|--------|------|------------------------|
| 1 | `driver` | `T` (global text) | YES |

That is the complete list. `c_src/include/driver.h` declares exactly one
function, `void driver(int x)`.

`print_hex` is declared `static` in `c_src/src/driver.c`, so it has internal
linkage and is deliberately **not** part of the dynamic symbol table of the C
`.so`. It is therefore correctly kept private (`unsafe fn print_hex`) in the
Rust translation; exporting it would be a *divergence*, not a fix.

## Rust `.so` exported (defined) dynamic symbols

`nm -D --defined-only translation/target/release/libdriver.so`

| # | symbol | type |
|---|--------|------|
| 1 | `driver` | `T` (global text) |

## Symbol diff

```
comm -3 <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $NF}' | sort -u) \
        <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
```

Result: **empty**. 0 symbols missing from the Rust `.so`.

No C source file was left untranslated: `CMakeLists.txt` lists exactly one
translation unit (`src/driver.c`), and both of its functions (`driver`,
`print_hex`) are present in `translation/src/lib.rs`.

## Undefined (imported) symbols — informational

The C `.so` imports `printf@GLIBC_2.2.5` and `putchar@GLIBC_2.2.5`; `putchar`
appears only because the compiler rewrites the constant-string `printf("\n")`
call into `putchar('\n')`. The Rust `.so` imports `printf` and resolves it
against the same glibc in-process, so both libraries write through the *same*
`FILE *stdout` with the same buffering. There are no undefined non-libc symbols
in the Rust `.so`.

Verification helper: `translation/check_symbols.sh`.
