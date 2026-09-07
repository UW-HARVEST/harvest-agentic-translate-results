# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory

The whole library is a single translation unit, `c_src/src/driver.c` (66 lines,
license header included). The public header `c_src/include/driver.h` declares
only `driver`, but the `.so` exports every non-`static` function in the file:

| C function | declared in header? | translated in Rust? |
|------------|---------------------|---------------------|
| `printLine(const char *)` | no (file-scope, non-static → exported) | yes |
| `printIntLine(int)`       | no (file-scope, non-static → exported) | yes |
| `bad(void)`               | no (file-scope, non-static → exported) | yes |
| `good(void)`              | no (file-scope, non-static → exported) | yes |
| `driver(void)`            | yes | yes |

No other `.c` file exists in `c_src/`, so no module was skipped by the
translation. No macro-generated symbols, no exported globals, no
`__attribute__((constructor))`.

## Dynamic symbol table comparison

`T` = global text symbol, defined and exported.

| symbol | C `.so` | Rust `.so` | status |
|--------|---------|------------|--------|
| `bad`          | `T` | `T` | present in both |
| `driver`       | `T` | `T` | present in both |
| `good`         | `T` | `T` | present in both |
| `printIntLine` | `T` | `T` | present in both |
| `printLine`    | `T` | `T` | present in both |

**Missing from Rust `.so`: none.** No `#[no_mangle]` wrapper needed to be added
and no C module needed to be translated.

The Rust `.so` exports no *extra* non-libc global symbols beyond these five
(the remainder of its dynamic symbol table is weak/undefined libc, `_Unwind_*`
and `_ITM_*` entries pulled in by `libstd`).

## Undefined (imported) symbols in the Rust `.so`

All undefined symbols are libc / C++-unwinder ABI entries, i.e. 0 missing
non-libc symbols:

`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `_Unwind_Backtrace`,
`_Unwind_GetDataRelBase`, `_Unwind_GetIP`, `_Unwind_GetIPInfo`,
`_Unwind_GetLanguageSpecificData`, `_Unwind_GetRegionStart`,
`_Unwind_GetTextRelBase`, `_Unwind_Resume`, `_Unwind_SetGR`, `_Unwind_SetIP`,
`__cxa_finalize`, `__cxa_thread_atexit_impl`, `__errno_location`,
`__gmon_start__`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `printf`, `pthread_key_create`, `pthread_key_delete`,
`pthread_setspecific`, `puts`, `read`, `readlink`, `realloc`, `realpath`,
`stat64`, `statx`, `strlen`, `syscall`, `write`, `writev`.

The important one is `printf@GLIBC_2.2.5`: the Rust translation calls the
platform `printf` rather than reimplementing formatting, so the emitted bytes
and the `stdout` buffering/flush semantics are the same object as the C
library's.

## Cargo feature surface

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configurations are the default one and `--no-default-features`
(which is identical, since there are no default features). Both are checked
in Phase D.

## Verification checklist

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`
- [x] no stubbed / `unimplemented!()` symbols

## Phase D evidence (re-run, both profiles)

```sh
nmdef(){ nm -D --defined-only --format=posix "$1" \
         | awk '$2=="T"||$2=="D"||$2=="B"||$2=="R"{print $1}' | sort -u; }
comm -23 <(nmdef c_src/build/libdriver.so) <(nmdef translation/target/debug/libdriver.so)
comm -23 <(nmdef c_src/build/libdriver.so) <(nmdef translation/target/release/libdriver.so)
```

| object | exported globals |
|--------|------------------|
| C `.so`               | `bad driver good printIntLine printLine` |
| Rust `.so` (dev)      | `bad driver good printIntLine printLine` |
| Rust `.so` (release)  | `bad driver good printIntLine printLine` |

Symbols exported by the C `.so` but missing from the Rust `.so`: **none**, in
both profiles. `grep -cE 'unimplemented!|todo!|unreachable!\(\)' src/lib.rs` → 0,
so no symbol is present only as a stub.

## Test-harness integrity notes

Two things had to be fixed before the differential results meant anything; both
are recorded here because they are the kind of flaw that makes a suite pass
vacuously:

1. **libtest output contaminating the capture.** The tests capture `stdout` by
   `dup2`-ing fd 1 onto a temp file, and fd 1 is process-global, so libtest's own
   progress lines from *other* test threads landed inside captures. Fixed by
   `RUST_TEST_THREADS=1` in `.cargo/config.toml` plus flushing Rust's buffered
   stdout before each redirect.
2. **`cargo test` does not build a `cdylib`-only lib target.** Plain
   `cargo test` therefore `dlopen`ed a `.so` left over from an earlier build —
   including silently falling back to the *release* artifact during a *debug*
   run. Fixed by resolving the `.so` strictly inside the running test binary's
   own profile directory and asserting it is newer than every crate source file
   (`STALE ARTIFACT` panic otherwise). `run_difftests.sh` builds `--lib` for each
   configuration before testing it.

### Negative controls (mutation testing)

Each mutation was applied to `src/lib.rs`, the cdylib rebuilt, and the suite
re-run; the source was restored afterwards.

| mutation | detected by |
|----------|-------------|
| `bad()`'s discarded `intOne + intTwo` "fixed" to actually assign | `cfg_row11`, `cfg_row12`, `cfg_row13`, `cfg_row14`, `cfg_row15` |
| `printLine`'s `NULL` check removed | `err_row1`, `err_row6`, `err_generic_void_fns_after_rejection`, `cfg_row15` |
| `printIntLine` switched from libc `printf` to Rust `println!` | `cfg_row12`, `cfg_row14`, `cfg_row15`, `driver_stdout_parity` |
| `#[no_mangle]` dropped from `good` | `nm_symbol_diff_is_empty`, `dlsym_resolves_every_symbol_in_both` |
| `good` renamed | `cfg_row10`–`cfg_row14` |

## Configuration matrix result

`./run_difftests.sh` builds the C library, then for each of
{dev, release} × {default, `--no-default-features`, `--all-features`} builds the
cdylib and runs all 27 tests: **6/6 configurations pass, 0 failures.**
`cargo check --all-targets` is warning-free in every combination.
