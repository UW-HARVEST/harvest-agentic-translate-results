# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on the built shared objects.

- C: `c_src/build/libStaticAlias.so` (cmake, `add_library(StaticAlias SHARED src/staticalias.c)`)
- Rust: `translation/target/release/libStaticAlias.so` (`crate-type = ["cdylib"]`, `[lib] name = "StaticAlias"`)

Commands used:

```sh
nm -D --defined-only  c_src/build/libStaticAlias.so            | awk '{print $3}' | sort
nm -D --defined-only  translation/target/release/libStaticAlias.so | awk '{print $3}' | sort
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt      # symbols in C but not Rust
```

## Public (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | source of truth | notes |
|---|--------|---------|------------|-----------------|-------|
| 1 | `static_alias` | `T` (0x1119) | `T` | `c_src/include/staticalias.h:29`, `c_src/src/staticalias.c:28` | `int *static_alias(int *outer)` |
| 2 | `driver`       | `T` (0x1168) | `T` | `c_src/include/staticalias.h:30`, `c_src/src/staticalias.c:43` | `void driver(int initial_value, int iterations)` |

There are exactly **two** public symbols. Both are declared in the single public
header and defined in the single translation unit; the C source contains no other
non-static functions and no exported data objects (the only `static` object,
`inner`, has internal linkage and is therefore correctly *absent* from the C
dynamic symbol table — the Rust translation likewise does not export `INNER`).

No macro-generated symbols exist: `c_src/src/staticalias.c` defines no
function-generating macros, and its only `#include`s are `<stdio.h>` and
`"staticalias.h"`.

## Symbol diff

```
missing in Rust: <empty>
extra in Rust  : <empty>
```

**0 missing symbols.** No `#[no_mangle]` wrapper had to be added and no C module
was left untranslated: `src/staticalias.c` is the only source file listed in
`c_src/CMakeLists.txt`, and both of its functions are translated in
`translation/src/lib.rs` with `#[unsafe(no_mangle)] pub unsafe extern "C"`.

## Undefined (imported) symbols

The C `.so` imports only `printf@GLIBC_2.2.5` plus the standard weak
CRT/ITM hooks (`__cxa_finalize`, `__gmon_start__`,
`_ITM_{de,}registerTMCloneTable`).

The Rust `.so` imports `printf@GLIBC_2.2.5` (the translation binds libc `printf`
directly rather than using `std::io::stdout`, so stdout buffering and ordering
match the C exactly) plus the Rust `std`/`libunwind` runtime imports
(`_Unwind_*`, `malloc`/`free`/`realloc`/`calloc`/`posix_memalign`, `memcpy`,
`memmove`, `memset`, `bcmp`, `strlen`, `abort`, `dl_iterate_phdr`,
`pthread_key_*`, `__tls_get_addr`, `__errno_location`, `open64`/`read`/`write`/
`writev`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`/`mmap64`/`munmap`,
`getcwd`/`getenv`/`readlink`/`realpath`/`syscall`/`gettid`,
`__cxa_thread_atexit_impl`).

All of these are **libc / language-runtime** symbols satisfied by
`libc.so.6` / `libgcc_s.so.1`. There are **0 missing or undefined non-libc
symbols** in the Rust `.so`.

Verified with:

```sh
ldd -r translation/target/release/libStaticAlias.so   # no "undefined symbol" lines
```

## Completion checklist

- [x] Every C dynamic symbol is exported by the Rust `.so` with the exact same name.
- [x] Symbol diff is empty in both directions.
- [x] 0 missing/undefined non-libc symbols in the Rust `.so`.
- [x] Holds for all four build configurations swept by `./verify.sh`
      (`{default, --no-default-features} × {release, debug}`).
- [x] Enforced as a test (`symbol_parity_is_exact`,
      `rust_so_has_no_unresolved_nonlibc_symbols` in
      `tests/phase_d_parity.rs`), which also fails if the C public surface ever
      grows a symbol, so this document cannot silently go stale.

## Nothing was stubbed

No symbol here is a stub. `src/staticalias.c` is the only source file in
`c_src/CMakeLists.txt` and both of its functions were already fully translated,
so no module was missing and no `unimplemented!()` was introduced. The one change
made to `translation/src/lib.rs` during verification was a behavioural fidelity
fix inside `static_alias` (unaligned pointer access — see `ERRORS.md`), not the
addition of an export.
