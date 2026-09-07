# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The entire C library is two files:

| C file | public functions defined |
|--------|--------------------------|
| `c_src/include/lib.h` | (declaration only) `char *custom_strdup(const char *str);` |
| `c_src/src/lib.c` | `custom_strdup` |

`c_src/CMakeLists.txt` compiles exactly one translation unit (`src/lib.c`) into
`add_library(driver SHARED ...)`. There is **no** `add_executable`, so the
project builds **no binary driver** — the stdout-comparison item of the
completion gate is N/A (see `CONFIGS.md`).

There are no namespace-renaming macros, no `#ifdef`-gated alternate
implementations, and no macro-generated symbol families in the header, so the
source-level name is the final linker name. No C module was left untranslated.

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | status |
|---|--------|------------|---------------|--------|
| 1 | `custom_strdup` | `T` (yes) | `T` (yes) | ✅ present in both |

**Missing from Rust `.so`: none.** No `#[no_mangle]` wrapper had to be added and
no C module had to be translated; the Rust `cdylib` already exports the full C
surface. Nothing is stubbed or `unimplemented!()`.

## Undefined (imported) symbols

The C `.so` imports only `malloc`, `memcpy`, `strlen` (plus the usual weak
`__cxa_finalize` / `__gmon_start__` / `_ITM_*` glibc-toolchain markers).

The Rust `.so` imports that same set plus the Rust standard-library runtime's
own libc/unwind dependencies (`_Unwind_*`, `abort`, `calloc`, `realloc`, `free`,
`mmap64`, `munmap`, `open64`, `read`, `write`, `pthread_key_*`,
`__errno_location`, `dl_iterate_phdr`, …).

**0 missing / undefined non-libc symbols in the Rust `.so`** — every `U` entry
resolves against `libc`/`libgcc_s` (glibc + unwinder), which are present on the
target and are pulled in by `std` itself, not by unresolved translated code.
Verified with:

```sh
nm -D --undefined-only translation/target/release/libdriver.so \
  | grep -v -E 'GLIBC|GCC_|_ITM_|__gmon_start__|statx|gettid'   # -> empty
ldd -r translation/target/release/libdriver.so                  # -> no "undefined symbol"
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default one. `--no-default-features` and
`--all-features` resolve to the identical unit. The "every feature combination"
gate is satisfied by the single default configuration, which is verified
explicitly in `run_all.sh`.
