# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
nm -D -u  <each>            # undefined / imported
```

## Public (defined, dynamic) symbols of the C `.so`

| # | symbol | type | present in Rust `.so`? | notes |
|---|--------|------|------------------------|-------|
| 1 | `driver` | `T` (global text) | YES — `T driver` | `void driver(int x, int local_y, int z)`; the only symbol in `include/driver.h`. Rust: `#[unsafe(no_mangle)] pub extern "C" fn driver`. |

The C translation unit has exactly one external definition. `multi_stage` is
`static` in C (file-scope, internal linkage) and therefore **must not** be
exported; the Rust translation correctly keeps it a private `fn multi_stage`.
`static int y = 123;` is likewise `static` in C, so no `y` symbol is exported;
the Rust `static Y: AtomicI32` is private and not exported. This is parity, not
a gap.

**Symbol diff (C defined − Rust defined): EMPTY.**
**Extra non-libc symbols in Rust: NONE** (Rust exports only `driver`).

## Undefined / imported symbols

C imports: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, plus weak
`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__cxa_finalize@GLIBC_2.2.5`, `__gmon_start__`.

(`puts` appears because GCC rewrites `printf("literal\n")` — a format string
with no conversion specifiers — into `puts("literal")`. The emitted bytes on
stdout are identical, so this is an implementation detail, not a behavioural
difference.)

Rust imports: `printf@GLIBC_2.2.5`, `puts@GLIBC_2.2.5`, and the usual
libc / libgcc-unwind / `std` runtime set (`malloc`, `free`, `calloc`,
`realloc`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`,
`__errno_location`, `_Unwind_*`, `pthread_key_*`, `dl_iterate_phdr`,
`open64`/`read`/`write`/`close`/`stat64`/`mmap64`/... for the panic-backtrace
machinery, `__tls_get_addr`, `syscall`, `getenv`, `getcwd`, `readlink`,
`realpath`, `writev`, weak `statx`/`gettid`/`__cxa_thread_atexit_impl`).

**Non-libc / non-runtime undefined symbols in the Rust `.so`: 0.**

## Completion status

- [x] Every symbol the C `.so` exports is exported by the Rust `.so` with the
      exact same name (`driver`).
- [x] `nm -D` shows 0 missing symbols and 0 undefined non-libc symbols in the
      Rust `.so`.
- [x] No whole C module/file was skipped: `c_src/src/driver.c` (62 lines,
      1 public function + 1 static function + 1 static variable) is the entire
      C source set, and all of it is present in `translation/src/lib.rs`.
- [x] No stubs / `unimplemented!()` / `todo!()` anywhere in the Rust crate
      (verified by grep).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default one. `cargo test --no-default-features` is
equivalent to the default build here. Verified by grep: no `#[cfg(feature`
in the crate.
