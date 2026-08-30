# SYMBOLS.md — Phase A symbol surface

## Source inventory (mechanical)

`c_src/CMakeLists.txt` builds exactly **one** translation unit into
`libdriver.so`:

```
add_library(driver SHARED src/driver.c)
```

So the whole C library is `c_src/src/driver.c` (68 lines) + `c_src/include/driver.h`.
There is **no** untranslated module: `translation/src/lib.rs` covers every
function in `driver.c`.

| C source entity | linkage in C | present in Rust? | Rust item |
|---|---|---|---|
| `void printLine(const char *line)` | external (`T`) | yes | `#[no_mangle] pub unsafe extern "C" fn printLine` |
| `static char *helperBad()`         | **internal** (`static`, not exported) | yes | private `fn helperBad()` |
| `void bad()`                       | external (`T`) | yes | `#[no_mangle] pub unsafe extern "C" fn bad` |
| `static char *helperGood1()`       | **internal** (`static`, not exported) | yes | private `fn helperGood1()` |
| `void good()`                      | external (`T`) | yes | `#[no_mangle] pub unsafe extern "C" fn good` |
| `void driver(int useGood)`          | external (`T`) | yes | `#[no_mangle] pub unsafe extern "C" fn driver` |

There are no macros that generate symbols, no `#ifdef`-guarded extra
definitions, and no global/static data with external linkage in the C source.

## `nm -D --defined-only` — C `.so` (ground truth)

Command: `nm -D --defined-only c_src/build/libdriver.so`

```
0000000000001186 T bad
00000000000011c5 T driver
00000000000011ac T good
0000000000001139 T printLine
```

4 exported symbols. (`nm -D` additionally lists the weak/undefined entries
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`, `puts@GLIBC_2.2.5` — these are
toolchain/libc artifacts, not part of the library's API surface.)

## `nm -D --defined-only` — Rust `.so`

Command: `nm -D --defined-only translation/target/release/libdriver.so`

```
0000000000011780 T bad
0000000000011790 T driver
00000000000117b0 T good
00000000000117c0 T printLine
```

## Parity result

| C symbol | exported by Rust `.so`? |
|---|---|
| `bad`       | ✅ |
| `driver`    | ✅ |
| `good`      | ✅ |
| `printLine` | ✅ |

**Missing symbols: 0.** The set difference `C_defined \ Rust_defined` is empty.
The Rust `.so` exports no *extra* C-API symbols either — the sets are exactly
equal — so no accidental over-export.

`tests/symbols.rs::phase_d_symbol_parity_c_minus_rust_is_empty` re-derives both
lists at test time with `nm -D --defined-only` and asserts the difference is
empty, so this table cannot silently rot.

## Undefined (imported) symbols in the Rust `.so`

All undefined/weak entries resolve to libc / libgcc-unwind and are therefore
acceptable (0 missing non-libc symbols):

```
_Unwind_*@GCC_*            (libgcc_s — panic unwinding machinery)
__cxa_finalize, __cxa_thread_atexit_impl, __errno_location, __tls_get_addr,
__gmon_start__, _ITM_*TMCloneTable, gettid, statx        (glibc / toolchain)
abort bcmp calloc close dl_iterate_phdr free fstat64 getcwd getenv lseek64
malloc memcpy memmove memset mmap64 munmap open64 posix_memalign
pthread_key_create pthread_key_delete pthread_setspecific puts read readlink
realloc realpath stat64 strlen syscall write writev            (glibc)
```

Note `puts@GLIBC_2.2.5`: the Rust translation writes its output with
`printf("%s\n", line)` and LLVM applies the same `printf`→`puts` builtin
transformation GCC applies to the C source, so both libraries end up importing
the identical libc entry point. Output bytes are identical either way
(`puts(s)` ≡ `printf("%s\n", s)` for any NUL-terminated `s`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration (the default). `scripts/check_features.sh` enumerates
the feature list from `cargo metadata` and loops over every combination; with an
empty feature set the only combination is `--no-default-features` ≡ default.
Phases B–D therefore cover 100% of configurations. See `FEATURES.md` for the
recorded run.
