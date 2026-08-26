# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`,
  `CMAKE_BUILD_TYPE` empty ⇒ no `-O` flag ⇒ `-O0`)
* Rust `.so`: `target/debug/liboverunder_lib.so` (`crate-type = ["cdylib"]`)

## Exported (defined, global) symbols

| # | symbol | C `.so` | Rust `.so` | signature (from `c_src/src/lib.c`) |
|---|--------|---------|------------|------------------------------------|
| 1 | `safe_double_to_int`       | T | T | `int safe_double_to_int(double d)` |
| 2 | `process_with_fallthrough` | T | T | `int process_with_fallthrough(int code, int base_value)` |
| 3 | `copy_data_block`          | T | T | `void copy_data_block(DataBlock *dest, const DataBlock *src)` |
| 4 | `handle_pointer_operations`| T | T | `int handle_pointer_operations(int value)` |
| 5 | `overunder`                | T | T | `int overunder(int a, int b, int c, int d)` |

`c_src/include/lib.h` declares only `overunder`, but the other four C functions
are non-`static` and therefore part of the exported ABI surface; all five are
tested through the `.so` boundary.

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort) \
           <(nm -D --defined-only target/debug/liboverunder_lib.so   | awk '{print $3}' | sort)
<empty>
```

**0 symbols missing from the Rust `.so`.** No macro-generated symbols exist
(`MAKE_VAR_NAME` / `PRINT_VAR` expand to *local variables* and *format-string
literals* inside `overunder`, not to new external functions).

## Undefined (imported) symbols

The Rust `.so` imports only libc / libgcc-unwind symbols; there are **0 missing
non-libc symbols**:

* C `.so` imports: `memcpy`, `printf`, `putchar`, `sqrt`, `strncpy`
  (`putchar` is gcc's strength-reduction of `printf("\n")`).
* Rust `.so` imports: `printf`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`,
  `malloc`/`calloc`/`realloc`/`free`/`posix_memalign`, `abort`,
  `__errno_location`, `_Unwind_*`, `pthread_key_*`, plus the std-runtime
  file/mmap syscall wrappers. `sqrt` is inlined as the `sqrtsd` instruction and
  `strncpy` is open-coded, so neither appears as an import — behaviourally
  equivalent (IEEE-754 `sqrt` is exactly rounded).

## Types crossing the FFI boundary

`DataBlock` layout was verified to be identical:

| field | C offset | Rust `#[repr(C)]` offset |
|-------|----------|--------------------------|
| `id` (`int`)        | 0  | 0  |
| *(padding)*         | 4  | 4  |
| `value` (`double`)  | 8  | 8  |
| `label` (`char[20]`)| 16 | 16 |
| *(tail padding)*    | 36 | 36 |
| **sizeof / alignof**| **40 / 8** | **40 / 8** |
