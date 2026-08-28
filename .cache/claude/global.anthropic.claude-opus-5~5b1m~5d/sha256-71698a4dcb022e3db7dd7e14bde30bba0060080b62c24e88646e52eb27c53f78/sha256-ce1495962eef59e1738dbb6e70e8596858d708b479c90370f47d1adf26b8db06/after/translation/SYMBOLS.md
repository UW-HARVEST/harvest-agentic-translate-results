# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-uATrTJ.so` (name comes from the parent
  directory via `cmake_path(GET parent FILENAME project_name)`, so it is
  environment-dependent — tests glob for it instead of hard-coding it).
* Rust `.so`: `translation/target/release/libbuffapp_lib.so`

Regenerate / re-verify with `./check_symbols.sh`.

## Defined (exported) symbols in the C `.so`

`nm -D <c.so> | awk '$2=="T"'`

| # | symbol | C signature (`src/lib.c`) | in Rust `.so`? | Rust item |
|---|--------|---------------------------|----------------|-----------|
| 1 | `create_buffer`      | `StringBuffer* create_buffer(int initial_capacity)`              | YES `T` | `#[no_mangle] create_buffer` |
| 2 | `append_to_buffer`   | `int append_to_buffer(StringBuffer *buffer, const char *str)`    | YES `T` | `#[no_mangle] append_to_buffer` |
| 3 | `destroy_buffer`     | `void destroy_buffer(StringBuffer *buffer)`                      | YES `T` | `#[no_mangle] destroy_buffer` |
| 4 | `get_operation_name` | `const char* get_operation_name(int op_code)`                    | YES `T` | `#[no_mangle] get_operation_name` |
| 5 | `perform_operation`  | `int perform_operation(int a, int b, const char *operation)`     | YES `T` | `#[no_mangle] perform_operation` |
| 6 | `buffapp`            | `int buffapp(int,int,int,int)` (the only symbol in `include/lib.h`) | YES `T` | `#[no_mangle] buffapp` |

**Missing from the Rust `.so`: none.** No C source file was left untranslated —
`src/lib.c` is the only translation unit in `CMakeLists.txt`, and all six of its
external definitions have real Rust bodies (no stubs, no `unimplemented!()`).

Note: `StringBuffer` is *not* declared in the public header; it is a private
`typedef` in `src/lib.c`. It is still part of the observable ABI because
`create_buffer` returns one and `append_to_buffer`/`destroy_buffer` consume one,
so the tests re-declare it as `#[repr(C)]` and diff its fields directly
(`data` contents, `capacity`, `length`).

## Undefined (imported) symbols

The C `.so` imports only libc: `malloc`, `realloc`, `free`, `strlen`, `strcpy`,
`strcmp`, `sprintf`, `printf` (+ the weak `_ITM_*` / `__gmon_start__` /
`__cxa_finalize` boilerplate every ELF shared object gets).

The Rust `.so` imports that same libc set — deliberately, since it calls the
platform `malloc`/`realloc`/`free`/`str*`/`sprintf`/`printf` so that heap
interop and the exact `printf` byte stream are identical — **plus** the Rust
runtime's own imports (`_Unwind_*`, `__errno_location`, `abort`, `memcpy`,
`mmap64`, `dl_iterate_phdr`, `pthread_key_*`, …). These extra `U` entries are
libc/libgcc symbols pulled in by `std` and the panic machinery.

**0 missing / undefined non-libc symbols in the Rust `.so`.**
The parity requirement is one-directional (every symbol the C `.so` exports must
also be exported by the Rust `.so`); the Rust `.so` legitimately imports more
libc than the C one does.
