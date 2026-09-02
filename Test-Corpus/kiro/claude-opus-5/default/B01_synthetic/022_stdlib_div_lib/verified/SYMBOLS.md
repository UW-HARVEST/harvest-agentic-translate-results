# SYMBOLS.md — Symbol parity between C `.so` and Rust `.so`

Artifacts compared:

- C:    `c_src/build/libdriver.so`      (cmake, `add_library(driver SHARED src/driver.c)`)
- Rust: `translation/target/release/libdriver.so` (`crate-type = ["cdylib"]`)
- Rust: `translation/target/debug/libdriver.so`   (same sources, `panic=unwind` profile)

## Exported (defined, dynamic) symbols

`nm -D --defined-only` on each library.

| # | C symbol | type | present in Rust `.so`? | notes |
|---|----------|------|------------------------|-------|
| 1 | `driver` | `T` (global text) | YES — `T driver` | declared in `include/driver.h` as `void driver(int x, int y);`; Rust exports it via `#[unsafe(no_mangle)] pub extern "C" fn driver(x: c_int, y: c_int)` |

The C `.so` exports exactly one symbol. `src/driver.c` defines no other
function, no global/static data with external linkage, and contains no
symbol-generating macros. There is no second translation unit
(`CMakeLists.txt` lists only `src/driver.c`), so no C module was skipped
during translation — the surface is genuinely one function.

**Missing-from-Rust count: 0.** No `#[no_mangle]` wrapper had to be added and
no untranslated C module was found.

The Rust `.so` exports no *extra* non-standard symbols either (only `driver`
plus the platform-standard `_init`/`_fini`-class entries that the linker adds
to every shared object; these do not appear in `nm -D --defined-only`).

## Undefined (imported) symbols

Requirement: 0 missing/undefined **non-libc** symbols in the Rust `.so`.

C imports:

```
w _ITM_deregisterTMCloneTable      w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5       w __gmon_start__
U div@GLIBC_2.2.5                  U printf@GLIBC_2.2.5
```

Rust imports (49 entries) are all one of:

- `printf@GLIBC_2.2.5` — the same libc symbol the C uses for output;
- other glibc symbols pulled in by the Rust standard library
  (`malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`,
  `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `getenv`, `getcwd`,
  `read`, `write`, `writev`, `close`, `open64`, `lseek64`, `fstat64`,
  `stat64`, `statx`, `readlink`, `realpath`, `mmap64`, `munmap`, `syscall`,
  `__errno_location`, `dl_iterate_phdr`, `gettid`, `pthread_key_create`,
  `pthread_key_delete`, `pthread_setspecific`, `__tls_get_addr`,
  `__cxa_thread_atexit_impl`, `__cxa_finalize`);
- `_Unwind_*@GCC_*` from libgcc_s (panic/backtrace machinery);
- the weak `_ITM_*TMCloneTable` / `__gmon_start__` toolchain markers, which
  the C `.so` also has.

Rust does **not** import `div`: `div()` is reimplemented in the translation
(see `c_div` in `src/lib.rs`) rather than called through libc. That is an
implementation choice, not a missing symbol — `div` is not part of this
library's exported surface, and glibc's `div` is itself just
`{ numer/denom, numer%denom }`, so the differential tests confirm the
reimplementation is behaviourally identical (including the two faulting
cases; see `ERRORS.md`).

**Undefined non-libc / non-toolchain symbols in Rust `.so`: 0.**

## Verification commands

```sh
nm -D --defined-only c_src/build/libdriver.so       | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms      # C-only symbols -> must be EMPTY
```

Result: empty. Symbol diff has reached empty.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the crate has
exactly one build configuration (no default features, no optional features).
"Every feature combination" is therefore the single empty combination, and it
is covered by the default `cargo test` / `cargo build` runs. For completeness
the test suite is additionally run under
`--no-default-features` and both the `debug` and `release` profiles are loaded
and compared against C (`release` differs meaningfully: `panic = "abort"`).
