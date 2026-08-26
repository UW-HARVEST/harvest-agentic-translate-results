# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from the C sources, not from assumptions.

## How the two `.so` files are produced

`c_src/CMakeLists.txt` contains exactly one target:

```cmake
cmake_minimum_required(VERSION 3.10)
project(driver)
add_executable(driver src/main.c)
```

i.e. the C project is an **executable**, and there is only ONE translation unit
(`c_src/src/main.c`, 65 lines).  Every function in it has external linkage, so
the identical translation unit also links as a shared object:

```sh
# executable, exactly as CMakeLists.txt specifies (used for end-to-end tests)
cmake -S c_src -B c_src/build -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build c_src/build
# same TU as a shared library (used for the libloading differential tests)
gcc -shared -fPIC -O2 -o c_build/libdriver_c.so c_src/src/main.c
```

The Rust crate mirrors this: `[[bin]] driver` (byte-identical replacement for the
C executable) plus `[lib] crate-type = ["cdylib"]` → `target/<profile>/libdriver.so`
carrying the `#[no_mangle] extern "C"` wrappers in `src/lib.rs`.

Regenerate the comparison at any time with `./check_symbols.sh`.

## `nm -D --defined-only` on the C `.so`

```
0000000000001[...] T bad
0000000000001[...] T good
0000000000001[...] T main
0000000000001[...] T printIntLine
0000000000001[...] T printLine
```

## Symbol-by-symbol parity

| # | C symbol | C declaration (`c_src/src/main.c`) | exported by Rust `.so` | Rust export wrapper |
|---|----------|------------------------------------|------------------------|---------------------|
| 1 | `bad` | `void bad()` (line 39) | ✅ `T bad` | `src/lib.rs::bad` |
| 2 | `good` | `void good()` (line 47) | ✅ `T good` | `src/lib.rs::good` |
| 3 | `main` | `int main(int argc, char *argv[])` (line 55) | ✅ `T main` | `src/lib.rs::main` |
| 4 | `printIntLine` | `void printIntLine(int intNumber)` (line 34) | ✅ `T printIntLine` | `src/lib.rs::printIntLine` |
| 5 | `printLine` | `void printLine(const char *line)` (line 26) | ✅ `T printLine` | `src/lib.rs::printLine` |

**Missing symbols: none.**  No C source file was skipped by the translation —
`c_src/src/main.c` is the only file under `c_src/`, and all 5 of its external
symbols are implemented (not stubbed) in `src/driver.rs` and exported verbatim.

The Rust `.so` exports *exactly* these 5 dynamic symbols and nothing else
(`nm -D --defined-only target/release/libdriver.so | wc -l` → `5`), so the diff
is empty in **both** directions.

## Undefined / weak symbols

| symbol | in C `.so` | in Rust `.so` | note |
|--------|-----------|---------------|------|
| `printf@GLIBC_2.2.5` | `U` | not needed | libc; Rust uses `std::io::stdout()` |
| `puts@GLIBC_2.2.5` | `U` | not needed | gcc `-O2` rewrites `printf("%s\n",p)` → `puts(p)`; byte-identical output |
| `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__gmon_start__` | `w` (undefined weak) | — | crt/gmon artifacts, not part of the API |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | — | glibc shared-object teardown |

Only non-libc **defined** symbols matter for parity, and those are the 5 rows
above.  `nm -D --undefined-only` on the Rust `.so` resolves entirely against
`libc`/`libgcc_s`/`ld-linux` (verified by `ldd -r`), i.e. 0 missing non-libc
symbols.
