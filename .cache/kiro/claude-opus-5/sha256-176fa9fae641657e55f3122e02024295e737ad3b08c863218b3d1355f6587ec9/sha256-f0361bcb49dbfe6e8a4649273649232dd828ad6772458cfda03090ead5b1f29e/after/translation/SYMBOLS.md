# SYMBOLS.md — dynamic-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## Defined (exported) symbols

`nm -D --defined-only` output:

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|------------|--------|
| `driver` | `T` (0x1173) | `T` (0x11730) | MATCH |

C `.so` exported-symbol count: 1. Rust `.so` exported-symbol count: 1.
**Symbol diff is EMPTY.**

`print_hex` is `static` in `c_src/src/driver.c`, so it is deliberately absent
from the C dynamic symbol table; the Rust translation keeps it private for the
same reason. It is correctly NOT exported by either library.

## Weak / compiler-generated symbols (both libraries)

These are emitted by the toolchain, not by the translated source, and are
present in both objects:

| symbol | C | Rust |
|--------|---|------|
| `_ITM_deregisterTMCloneTable` | `w` | `w` |
| `_ITM_registerTMCloneTable` | `w` | `w` |
| `__cxa_finalize@GLIBC_2.2.5` | `w` | `w` |
| `__gmon_start__` | `w` | `w` |

## Undefined (imported) symbols

C `.so` imports: `printf@GLIBC_2.2.5`, `putchar@GLIBC_2.2.5`.
(`putchar` appears because GCC rewrites `printf("\n")` into `putchar('\n')`.)

Rust `.so` imports the same `printf@GLIBC_2.2.5` and `putchar@GLIBC_2.2.5`
plus the Rust standard-library / unwinder set, all of which are libc, libgcc
unwind (`_Unwind_*`) or pthread symbols:

```
_Unwind_Backtrace, _Unwind_GetDataRelBase, _Unwind_GetIP, _Unwind_GetIPInfo,
_Unwind_GetLanguageSpecificData, _Unwind_GetRegionStart, _Unwind_GetTextRelBase,
_Unwind_Resume, _Unwind_SetGR, _Unwind_SetIP, __cxa_thread_atexit_impl,
__errno_location, __tls_get_addr, abort, bcmp, calloc, close, dl_iterate_phdr,
free, fstat64, getcwd, getenv, gettid, lseek64, malloc, memcpy, memmove,
memset, mmap64, munmap, open64, posix_memalign, printf, pthread_key_create,
pthread_key_delete, pthread_setspecific, putchar, read, readlink, realloc,
realpath, stat64, statx, strlen, syscall, write, writev
```

**0 missing symbols. 0 undefined non-libc symbols in the Rust `.so`.**

No C source file was left untranslated: `c_src/src/driver.c` is the only
translation unit in `c_src/CMakeLists.txt`, and both of its functions
(`driver`, `print_hex`) are present in `translation/src/lib.rs`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configuration is the default (empty feature set). Verified:

```
$ grep -c '\[features\]' translation/Cargo.toml
0
```

`cargo check --no-default-features` and `cargo check` are therefore the same
configuration; both were run.

## How the differential suite is run

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo test --release
```

`cargo test` alone does **not** build a `cdylib`-only crate (an integration
test cannot link one), so `tests/differential.rs` builds the library itself into
`target/ffi/release/libdriver.so` and asserts that artifact is not older than
`src/lib.rs`. Without that, the suite would load whatever stale `.so` happened
to be on disk and pass vacuously — this was observed and fixed during
verification.

`.cargo/config.toml` sets `RUST_TEST_THREADS=1` because the tests redirect file
descriptor 1 to capture what the loaded `.so`s print; concurrent libtest
progress output would otherwise land inside a capture. `assert_same` also
rejects any captured byte that is not a lowercase hex digit or newline, so such
contamination fails loudly instead of masquerading as agreement.

## Negative controls (proof the suite detects divergence)

Each mutation was applied to `translation/src/lib.rs`, the suite was run, and
the source was restored:

| mutation | detected? | how |
|----------|-----------|-----|
| `bedrooms = 3` -> `4` | yes | byte diff at output offset 8 |
| `bathrooms = 2.0` -> `2.5` | yes | byte diff in the IEEE-754 image |
| `"%02x"` -> `"%02X"` | yes | contamination/hex-case guard |
| trailing `printf("\n")` removed | yes | all length/byte assertions |
| `HouseT` field order swapped (changes size 16 -> 24 and introduces padding) | yes | byte diff + length assertion |
| dump 15 bytes instead of 16 | yes | length + byte diff |

With the source unmutated, all 13 tests pass, repeatably (5 consecutive runs).
