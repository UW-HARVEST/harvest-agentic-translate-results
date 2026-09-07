# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared libraries.

Build commands:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` — `nm -D c_src/build/libdriver.so`

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
0000000000001186 T bad
00000000000011c5 T driver
00000000000011ac T good
0000000000001139 T printLine
                 U puts@GLIBC_2.2.5
```

`nm -D --defined-only` (the actual exported ABI): **4 symbols**.

## Rust `.so` — `nm -D --defined-only translation/target/release/libdriver.so`

```
0000000000011780 T bad
0000000000011790 T driver
00000000000117b0 T good
00000000000117c0 T printLine
```

## Parity table

| # | C symbol | type | C source | exported by Rust `.so`? | Rust item |
|---|----------|------|----------|-------------------------|-----------|
| 1 | `printLine` | `T` (global text) | `src/driver.c:28` `void printLine(const char *line)` | YES | `#[no_mangle] pub unsafe extern "C" fn printLine` |
| 2 | `bad`       | `T` (global text) | `src/driver.c:42` `void bad(void)`                  | YES | `#[no_mangle] pub unsafe extern "C" fn bad` |
| 3 | `good`      | `T` (global text) | `src/driver.c:53` `void good(void)`                  | YES | `#[no_mangle] pub unsafe extern "C" fn good` |
| 4 | `driver`    | `T` (global text) | `src/driver.c:58` `void driver(int useGood)`         | YES | `#[no_mangle] pub unsafe extern "C" fn driver` |

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added; no C module was
left untranslated. `c_src` contains exactly one translation unit (`src/driver.c`,
68 lines) and one header (`include/driver.h`), both fully translated in
`translation/src/lib.rs`.

## Deliberately NOT exported (must stay internal, matching the C)

| C entity | why not in the ABI | Rust counterpart |
|----------|--------------------|------------------|
| `static char *helperBad()` (`driver.c:36`)    | `static` linkage → internal to the TU, absent from `nm -D` | private `fn helperBad()` |
| `static char *helperGood1()` (`driver.c:47`)  | `static` linkage → internal to the TU, absent from `nm -D` | private `fn helperGood1()` |
| `static char charString[] = "helperGood1 string";` (`driver.c:49`) | function-scope `static` object, no external linkage | `static mut HELPER_GOOD1_CHAR_STRING` (mangled, `LOCAL` binding) |

Verified: `nm -D --defined-only` on the Rust `.so` exports **only** the 4 C
symbols — `HELPER_GOOD1_CHAR_STRING` is a `LOCAL` symbol (`readelf -sW`) and is
not in the dynamic symbol table, so the Rust ABI is neither missing nor
over-exporting relative to the C ABI.

## Undefined (imported) symbols

C imports `puts@GLIBC_2.2.5` — GCC rewrites `printf("%s\n", line)` into
`puts(line)`. The Rust build independently arrives at the same call
(`jmp *puts@GOT`), so the *bytes written to `stdout` and the stdio buffering
mode used are identical*, not merely equivalent.

Rust additionally imports the usual `libstd`/`libunwind` set
(`_Unwind_*`, `malloc`, `memcpy`, `dl_iterate_phdr`, `pthread_key_create`, …).
All are libc/`libgcc_s` symbols supplied by the platform; `nm -D -u` shows
**0 missing/undefined non-libc symbols**, i.e. the library loads and resolves
cleanly under `dlopen(RTLD_NOW)` (asserted by the test suite, which opens the
Rust `.so` and resolves all 4 symbols).

## Object layout parity

| object | C | Rust |
|--------|---|------|
| `helperGood1`'s `static charString` | `.data` (`WA`), size `0x13` = 19 bytes | `.data` (`WA`), size 19 bytes |

Both are in *writable* memory (the C object is a mutable `char[]` initialised
from a literal, not a `.rodata` string literal), so the pointer returned by
`good()`'s helper has the same storage class and mutability in both builds.
