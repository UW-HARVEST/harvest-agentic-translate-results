# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Sources:
- C  `.so`: `c_src/build/libharvest-work-9IYgVv.so`
- Rust `.so`: `translation/target/release/libcomplexmode_lib.so`

Commands used:

```sh
nm -D --defined-only c_src/build/libharvest-work-9IYgVv.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libcomplexmode_lib.so | awk '{print $3}' | sort
comm -23 c_syms r_syms      # symbols in C but NOT in Rust  -> MUST be empty
nm -D --undefined-only translation/target/release/libcomplexmode_lib.so
```

## Defined (exported) symbols

| # | symbol | in C `.so` | in Rust `.so` | C source | note |
|---|--------|-----------|---------------|----------|------|
| 1 | `check_permissions`   | T | T | `lib.c:47`  | leaf predicate |
| 2 | `compare_operations`  | T | T | `lib.c:90`  | wraps `strcmp` |
| 3 | `complexmode`         | T | T | `lib.c:99`  | only symbol in `include/lib.h` |
| 4 | `copy_and_sum`        | T | T | `lib.c:67`  | malloc + memcpy + sum |
| 5 | `create_result_string`| T | T | `lib.c:38`  | malloc(64) + snprintf |
| 6 | `multiply_with_log`   | T | T | `lib.c:59`  | out-param `char**` |
| 7 | `safe_add`            | T | T | `lib.c:51`  | permission-gated add |

`create_result_string`, `check_permissions`, `safe_add`, `multiply_with_log`,
`copy_and_sum` and `compare_operations` are **not** declared in
`include/lib.h`, but they have external linkage in `src/lib.c`, so they are real
public ABI symbols of the C `.so` and are tested as such.

There are no macro-generated symbols in this library (the only macros are
`READ_PERM`/`WRITE_PERM`/`EXEC_PERM`, which are integer constants).

## Symbol diff

```
$ comm -23 /tmp/c_syms.txt /tmp/r_syms.txt
(empty)
```

**0 symbols missing from the Rust `.so`.** No module of the C source was left
untranslated: `src/lib.c` is the only C translation unit and all 7 of its
external functions are present in `src/lib.rs` with `#[unsafe(no_mangle)]
pub extern "C"` wrappers. No stubs, no `unimplemented!()`.

## Undefined symbols in the Rust `.so`

All undefined symbols are libc / libgcc-unwind / Rust-runtime imports:

`printf snprintf malloc free memcpy strcmp strlen memmove memset bcmp calloc
realloc posix_memalign puts abort __errno_location __cxa_finalize
__cxa_thread_atexit_impl __tls_get_addr pthread_key_* dl_iterate_phdr
getcwd getenv gettid open64 close read write writev lseek64 fstat64 stat64
statx mmap64 munmap readlink realpath syscall _Unwind_* _ITM_* __gmon_start__`

**0 undefined non-libc symbols.** (`strcpy` does not appear: LLVM lowered the
`strcpy` of the string literals in `complexmode` into `memcpy`/stores, which is
behaviourally identical for NUL-terminated literals of known length.)

The C `.so` imports `printf malloc free memcpy strcmp strcpy snprintf puts` —
i.e. the Rust translation deliberately calls the *same* libc routines rather
than re-implementing formatting/allocation, so `stdout` buffering and heap
ownership are byte-identical.

## Gate

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`.
- [x] `nm -D` shows 0 undefined non-libc symbols in the Rust `.so`.
