# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-eb8Rqb.so`
* Rust `.so`: `translation/target/release/libenvy_lib.so`

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-eb8Rqb.so | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libenvy_lib.so | awk '{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # missing from Rust  -> MUST be empty
comm -13 /tmp/c_syms.txt /tmp/r_syms.txt   # extra in Rust
```

## Defined dynamic symbols

| # | symbol | C `.so` | Rust `.so` | C declaration | Rust definition |
|---|--------|---------|------------|---------------|-----------------|
| 1 | `apply_bit_operations` | `T` | `T` | `int apply_bit_operations(int value, struct ConfigFlags* flags)` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn apply_bit_operations` |
| 2 | `envy`                 | `T` | `T` | `int envy(int, int, int, int)` (the only symbol in `include/lib.h`) | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn envy` |
| 3 | `init_config_from_env` | `T` | `T` | `void init_config_from_env(struct ConfigFlags* flags)` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn init_config_from_env` |
| 4 | `parse_env_numeric`    | `T` | `T` | `int parse_env_numeric(const char* env_name, int default_val)` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_env_numeric` |
| 5 | `perform_operation`    | `T` | `T` | `int perform_operation(int val1, int val2, struct ConfigFlags* flags)` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn perform_operation` |

Counts: C = 5 defined dynamic symbols, Rust = 5.

There are no macro-generated symbols in the C source (`BUFFER_SIZE` is the only
object-like macro and it produces no symbol). There are no `static` helpers, so
the exported set equals the full set of functions in `src/lib.c`. No C module
was skipped by the translation: `src/lib.c` is the only translation unit in
`c_src/CMakeLists.txt`.

## Diff result

```
MISSING from Rust : (empty)
EXTRA in Rust     : (empty)
```

## Undefined (imported) symbols in the Rust `.so`

All undefined symbols are libc / toolchain runtime imports, none are
unresolved project symbols:

* libc used deliberately by the translation so that formatting and stream
  buffering are bit-identical to the C: `getenv`, `atoi`, `strchr`, `printf`,
  `fprintf`, `snprintf`, `stderr`.
* Rust `std` / `panic = "abort"` runtime and allocator imports: `malloc`,
  `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`, `memmove`,
  `memset`, `bcmp`, `strlen`, `abort`, `puts`, `write`, `writev`, `read`,
  `open64`, `close`, `lseek64`, `fstat64`, `stat64`, `statx`, `mmap64`,
  `munmap`, `getcwd`, `readlink`, `realpath`, `syscall`, `gettid`,
  `dl_iterate_phdr`, `__errno_location`, `__cxa_finalize`,
  `__cxa_thread_atexit_impl`, `__tls_get_addr`, `pthread_key_create`,
  `pthread_key_delete`, `pthread_setspecific`.
* libgcc unwinder + standard weak stubs: `_Unwind_*`, `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`, `__gmon_start__`.

## Gate

- [x] `nm -D` shows 0 symbols missing from the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc / non-runtime symbols in the Rust `.so`.

## Result

Symbol parity was already exact and required no new translation work: `src/lib.c`
is the only C translation unit, all five of its functions were present in the
Rust with `#[unsafe(no_mangle)] extern "C"` wrappers, and nothing was stubbed.

The diff is enforced from inside the test suite by
`tests/phase_d_symbols.rs::symbol_parity_is_exact`, which shells out to `nm -D`
on both objects and additionally asserts the C `.so` exports exactly the five
symbols listed above — so a future C function that the Rust never learns about
fails the build rather than passing silently.
`rust_so_has_no_unresolved_project_symbols` allowlists the libc/toolchain
imports and fails on anything else undefined.

Confirmed independently:

```
$ nm -D --defined-only c_src/build/libharvest-work-eb8Rqb.so | awk '{print $3}' | sort
apply_bit_operations envy init_config_from_env parse_env_numeric perform_operation
$ nm -D --defined-only translation/target/release/libenvy_lib.so | awk '{print $3}' | sort
apply_bit_operations envy init_config_from_env parse_env_numeric perform_operation
missing:[]  extra:[]
```
