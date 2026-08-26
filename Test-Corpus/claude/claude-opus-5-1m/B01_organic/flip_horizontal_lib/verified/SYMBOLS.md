# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

```
C   : c_src/build/libtranslated_rust.so        (cmake, default build type)
Rust: target/release/libflip_horizontal_lib.so (cargo build --release)
```

## C source inventory (completeness check)

The whole C library is two files; there is no untranslated module:

| C file | lines | translated to |
|---|---|---|
| `c_src/include/lib.h` | 16 | `src/lib.rs` (`cp_pixel_t`, `cp_image_t`, fn decl) |
| `c_src/src/lib.c` | 19 | `src/lib.rs` (`flip_horizontal`) |

`CMakeLists.txt` lists exactly one source file (`src/lib.c`), so the `.so`'s
entire public surface comes from that one translation unit.

## Defined (exported) symbols

`nm -D --defined-only`, filtering out the toolchain-generated weak symbols that
every glibc shared object carries (`_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, `__gmon_start__`,
`__cxa_thread_atexit_impl`, `gettid`, `statx`):

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `flip_horizontal` | `T` (0x10f9) | `T` (0x11c70) | **present in both** |

### Symbol diff

```
$ comm -23 <(c defined syms) <(rust defined syms)
(empty)
```

**0 symbols missing from the Rust `.so`.** No `#[no_mangle]` wrapper had to be
added and no C module was left untranslated. The macro-generated / weak
toolchain symbols above are emitted by the linker, not by the library source,
and are present in both objects (Rust additionally emits
`__cxa_thread_atexit_impl`, `gettid`, `statx` as weak because it links the Rust
standard library).

## Undefined (imported) symbols

The C `.so` imports nothing but the weak toolchain hooks. The Rust `.so`
imports only glibc and libgcc-unwind symbols pulled in by `std`
(`malloc`, `free`, `memcpy`, `memmove`, `memset`, `bcmp`, `calloc`, `realloc`,
`posix_memalign`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`,
`stat64`, `fstat64`, `readlink`, `realpath`, `getcwd`, `getenv`, `mmap64`,
`munmap`, `syscall`, `strlen`, `abort`, `__errno_location`, `__tls_get_addr`,
`pthread_key_create`, `pthread_key_delete`, `pthread_setspecific`,
`dl_iterate_phdr`, `_Unwind_*`).

**0 undefined non-libc symbols.** Verified with:

```
$ nm -D --undefined-only target/release/libflip_horizontal_lib.so \
    | grep -vE 'GLIBC|GCC_|__tls_get_addr|dl_iterate_phdr'
(empty)
```

## Feature combinations

`Cargo.toml` has **no `[features]` table**, so there is exactly one valid
configuration: `--no-default-features` (with no features). Verified
mechanically by `run_tests.sh`, which parses the `[features]` table out of
`Cargo.toml` and enumerates the power set — here, one empty combination.

Both cargo profiles are covered, since they differ in ways that are observable
across the FFI boundary (`debug-assertions` inserts UB checks; `release` sets
`panic = "abort"`):

| profile | `.so` | symbol parity | suites |
|---|---|---|---|
| `dev` (debug-assertions on) | `target/debug/libflip_horizontal_lib.so` | OK | 27 + 24 pass |
| `release` (`panic = "abort"`) | `target/release/libflip_horizontal_lib.so` | OK | 27 + 24 pass |

> Note: `cargo test` does **not** build a `cdylib` artifact, because the
> integration tests `dlopen` the library instead of linking it. The suites
> therefore assert the `.so` is newer than `src/*.rs` (`assert_so_fresh`) so a
> stale artifact can never be silently validated; `run_tests.sh` builds both
> profiles before testing.

## Result

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
- [x] Verified for both the debug and the release `.so` (`./check_symbols.sh <so>`).
