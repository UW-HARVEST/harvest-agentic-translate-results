# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-Yp1VMa.so`
- Rust `.so`: `translation/target/release/libbuffapp_lib.so`

## Exported (defined, global) symbols

| # | C symbol (`nm -D`) | C type | present in Rust `.so` | Rust definition |
|---|--------------------|--------|-----------------------|-----------------|
| 1 | `create_buffer`      | `T` | YES (`T`) | `#[no_mangle] pub unsafe extern "C" fn create_buffer` |
| 2 | `append_to_buffer`   | `T` | YES (`T`) | `#[no_mangle] pub unsafe extern "C" fn append_to_buffer` |
| 3 | `destroy_buffer`     | `T` | YES (`T`) | `#[no_mangle] pub unsafe extern "C" fn destroy_buffer` |
| 4 | `get_operation_name` | `T` | YES (`T`) | `#[no_mangle] pub unsafe extern "C" fn get_operation_name` |
| 5 | `perform_operation`  | `T` | YES (`T`) | `#[no_mangle] pub unsafe extern "C" fn perform_operation` |
| 6 | `buffapp`            | `T` | YES (`T`) | `#[no_mangle] pub unsafe extern "C" fn buffapp` |

`c_src/include/lib.h` declares only `buffapp`, but `lib.c` gives external linkage
to the other five functions as well, so all six are part of the ABI surface and
all six must be (and are) exported by the Rust `cdylib`.

There are no macro-generated symbols, no exported data objects, no versioned
symbols and no weak definitions in the C `.so`.

## Symbol diff

```
$ comm -3 <(nm -D --defined-only C.so   | awk '{print $3}' | sort) \
          <(nm -D --defined-only RUST.so| awk '{print $3}' | sort)
<empty>
```

Missing-from-Rust count: **0**. Extra-in-Rust count: **0**.

## Undefined symbols in the Rust `.so`

`nm -D -u` on the Rust `.so` lists only libc / libgcc-unwind imports:

`malloc`, `realloc`, `free`, `calloc`, `posix_memalign`, `strlen`, `strcpy`,
`strcmp`, `sprintf`, `printf`, `memcpy`, `memmove`, `memset`, `bcmp`, `abort`,
`__errno_location`, `__tls_get_addr`, `pthread_*`, `write`, `writev`, `read`,
`open64`, `close`, `lseek64`, `fstat64`, `stat64`, `mmap64`, `munmap`,
`getcwd`, `getenv`, `readlink`, `realpath`, `syscall`, `dl_iterate_phdr`,
`_Unwind_*`, plus weak `__gmon_start__`, `_ITM_*`, `__cxa_finalize`,
`__cxa_thread_atexit_impl`, `gettid`, `statx`.

**0 missing/undefined non-libc symbols.**

## Notes on shared runtime state

The Rust translation deliberately imports glibc's `malloc`/`realloc`/`free`,
`str*` and the `printf` family rather than reimplementing them. That keeps
heap ownership interoperable with C callers and makes `buffapp`'s stdout bytes
identical, which the Phase B stdout differential test relies on.
