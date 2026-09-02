# SYMBOLS.md — Phase A: exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

Commands used:

```sh
nm -D --defined-only  c_src/build/libdriver.so
nm -D --defined-only  translation/target/release/libdriver.so
nm -D --undefined-only c_src/build/libdriver.so
nm -D --undefined-only translation/target/release/libdriver.so
```

## Defined (exported) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `driver` | `T` (text, global) | `T` (text, global) | PRESENT in both |

`nm -D --defined-only` on the C `.so` yields exactly one line (`T driver`).
The Rust `.so` yields exactly one line (`T driver`).

**Symbol diff (C-defined minus Rust-defined): EMPTY.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so   | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort -u)
(no output)
```

## Non-exported C symbols (intentionally NOT in the Rust `.so`)

| C symbol | C linkage | Rust counterpart | why not exported |
|----------|-----------|------------------|------------------|
| `print_hex` | `static void print_hex(unsigned char *p, int len)` — internal, `t` in `nm` only, absent from `nm -D` | private `unsafe fn print_hex` in `src/lib.rs` | `static` in C gives it internal linkage, so it is not part of the C `.so`'s dynamic symbol table. Exporting it from Rust would ADD a symbol the C does not have. It is exercised indirectly, and exhaustively, through `driver` (which is its only caller). |

No C source file or module was skipped: `c_src` contains exactly one
translation unit (`src/driver.c`, 40 lines) and one public header
(`include/driver.h`), and both are fully translated in `translation/src/lib.rs`.
No symbol is stubbed or `unimplemented!()`.

## Undefined (imported) symbols

The C `.so` imports only `printf` and `putchar` from glibc (plus the standard
weak `_ITM_*` / `__cxa_finalize` / `__gmon_start__` set that every
`gcc`-produced shared object has).

The Rust `.so` imports those same two glibc stdio functions — the translation
deliberately binds to libc's `printf`/`putchar` rather than
`std::io::stdout`, so writes land in the *same* stdio `FILE` buffer with the
same formatting and flush semantics. Its remaining imports are all
libc/`libgcc` runtime support pulled in by the Rust standard library
(`_Unwind_*`, `malloc`/`free`/`realloc`/`calloc`/`posix_memalign`, `memcpy`,
`memmove`, `memset`, `bcmp`, `strlen`, `abort`, `__errno_location`,
`pthread_key_*`, `dl_iterate_phdr`, `open64`/`read`/`write`/`close`/`lseek64`,
`stat64`/`fstat64`/`statx`, `mmap64`/`munmap`, `getcwd`/`getenv`/`readlink`/
`realpath`, `syscall`, `writev`, `gettid`, `__tls_get_addr`,
`__cxa_thread_atexit_impl`).

**Undefined non-libc symbols in the Rust `.so`: 0.** Every `U`/`w` entry above
resolves against `libc.so.6` / `libgcc_s.so.1` / `ld-linux`, which is verified
by the fact that `libloading::Library::new` opens the Rust `.so` with
`RTLD_NOW` in the test suite and succeeds — `RTLD_NOW` forces eager resolution
of every undefined symbol at load time, so an unresolvable one would fail the
load.

## Completion checklist

- [x] `nm -D` shows 0 symbols present in the C `.so` and missing from the Rust `.so`.
- [x] `nm -D` shows 0 missing/undefined **non-libc** symbols in the Rust `.so`
      (proved by a successful `RTLD_NOW` load in `tests/differential.rs`).
- [x] No extra public symbols invented on the Rust side.
