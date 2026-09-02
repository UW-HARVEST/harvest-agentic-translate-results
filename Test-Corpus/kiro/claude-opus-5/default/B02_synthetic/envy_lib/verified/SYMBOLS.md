# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-irN63C.so
nm -D --defined-only translation/target/release/libenvy_lib.so
```

The C library consists of a single translation unit (`c_src/src/lib.c`) with no
`static` functions, so every function definition in that file becomes a public
dynamic symbol. `c_src/include/lib.h` declares only `envy`, but the other four
functions are exported too and are therefore part of the verified surface.

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | C source (`c_src/src/lib.c`) | Rust source (`src/lib.rs`) |
|---|--------|-----------|---------------|------------------------------|----------------------------|
| 1 | `parse_env_numeric`    | T | T | `int parse_env_numeric(const char*, int)`             | `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_env_numeric` |
| 2 | `init_config_from_env` | T | T | `void init_config_from_env(struct ConfigFlags*)`      | `#[unsafe(no_mangle)] pub unsafe extern "C" fn init_config_from_env` |
| 3 | `perform_operation`    | T | T | `int perform_operation(int, int, struct ConfigFlags*)`| `#[unsafe(no_mangle)] pub unsafe extern "C" fn perform_operation` |
| 4 | `apply_bit_operations` | T | T | `int apply_bit_operations(int, struct ConfigFlags*)`  | `#[unsafe(no_mangle)] pub unsafe extern "C" fn apply_bit_operations` |
| 5 | `envy`                 | T | T | `int envy(int, int, int, int)`                        | `#[unsafe(no_mangle)] pub unsafe extern "C" fn envy` |

**Missing from Rust: 0.** No symbol required an added `#[no_mangle]` wrapper and
no C module was left untranslated — `c_src/src/lib.c` is the only C source file
listed in `c_src/CMakeLists.txt` and all five of its functions are translated
with real bodies (no stubs, no `unimplemented!()`).

There are no macro-generated symbols: the only object-like macro in the C is
`BUFFER_SIZE`, which expands to a constant and emits no symbol.

## Undefined (imported) symbols

The C `.so` imports only libc: `atoi`, `fprintf`, `getenv`, `printf`, `puts`,
`snprintf`, `stderr`, `strchr` (plus the usual `__cxa_finalize`,
`__gmon_start__`, `_ITM_*` toolchain weak symbols).

The Rust `.so` imports that same libc set (the translation deliberately calls
libc `getenv`/`atoi`/`strchr`/`printf`/`fprintf`/`snprintf` and uses the libc
`stderr` stream so buffering and stdout/stderr interleaving match), plus the
Rust runtime's own libc/libgcc dependencies: `_Unwind_*` (libgcc), `malloc`,
`free`, `calloc`, `realloc`, `posix_memalign`, `memcpy`, `memmove`, `memset`,
`bcmp`, `strlen`, `abort`, `__errno_location`, `__tls_get_addr`,
`__cxa_thread_atexit_impl`, `pthread_key_*`, `pthread_setspecific`, `gettid`,
`syscall`, `dl_iterate_phdr`, `getcwd`, `readlink`, `realpath`, `open64`,
`close`, `read`, `write`, `writev`, `lseek64`, `fstat64`, `stat64`, `statx`,
`mmap64`, `munmap`.

`ldd` on the Rust `.so` resolves to `libgcc_s.so.1`, `libc.so.6`,
`ld-linux-x86-64.so.2` only.

**Missing / undefined non-libc symbols in Rust: 0.**

Enforced by the automated test `symbol_parity::c_and_rust_export_identical_symbols`
in `tests/symbol_parity.rs`, which shells out to `nm -D` on both libraries and
asserts the defined-symbol sets are equal and that every Rust undefined symbol
resolves against libc/libgcc.
