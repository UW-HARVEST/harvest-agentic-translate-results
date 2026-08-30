# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D --undefined-only <each>
```

## C `.so` exported (defined, dynamic) symbols

The whole C library is one translation unit (`c_src/src/driver.c`) declaring one
public function in `c_src/include/driver.h`:

```c
void driver(int x, int y);
```

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `driver` | `T` (text, global) | YES (`T driver`) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(c_int, c_int)` |

There are no macro-generated exports, no exported data symbols, no versioned
symbols, and no other translation units in `c_src/` — `ls c_src/src` yields only
`driver.c`. Therefore the export set is complete at one symbol; nothing had to be
translated or wrappered to close a gap.

## Symbol diff

```
C defined, missing from Rust:   (none)
```

The diff is EMPTY. Verified by `scratch/symcheck.sh`, which fails the build if a
C-exported symbol is absent from the Rust `.so`.

## Undefined (imported) symbols

C `.so` imports, ignoring weak toolchain hooks (`_ITM_*`, `__cxa_finalize`,
`__gmon_start__`):

| symbol | source |
|--------|--------|
| `div@GLIBC_2.2.5` | `<stdlib.h>` |
| `printf@GLIBC_2.2.5` | `<stdio.h>` |

The Rust `.so` imports both of these same libc symbols (it calls them via
`extern "C"` rather than reimplementing them), plus the usual Rust runtime set:
`_Unwind_*`, `malloc`/`calloc`/`realloc`/`free`/`posix_memalign`, `memcpy`,
`memmove`, `memset`, `bcmp`, `strlen`, `abort`, `__errno_location`,
`__tls_get_addr`, `pthread_key_*`, `dl_iterate_phdr`, `open64`/`read`/`write`/
`writev`/`close`/`lseek64`/`stat64`/`fstat64`/`statx`, `mmap64`/`munmap`,
`getcwd`/`getenv`/`readlink`/`realpath`, `syscall`, `gettid`.

All are libc / libgcc_s symbols resolved by the dynamic loader. There are **0
missing or unresolvable non-libc undefined symbols**; confirmed with:

```sh
ldd -r translation/target/release/libdriver.so
```

which reports no unresolved symbols.

## Reusing libc is deliberate, not a shortcut

Because the Rust translation forwards to the identical `div(3)` and `printf(3)`
implementations, the fatal-signal behaviour for the trapping inputs
(`y == 0`, and `INT_MIN / -1`) is reproduced exactly rather than being converted
into a Rust panic or a wrapped/checked division. See `ERRORS.md` rows 1–3.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table and no optional
dependencies, so the only configuration is the default (empty) feature set.
`--no-default-features` and the default build are therefore the same code, and
both are still exercised by the test script for completeness.
