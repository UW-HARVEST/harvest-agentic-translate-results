# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C `.so`  : `c_src/build/libharvest-work-QiJ5vr.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust `.so`: `translation/target/{debug,release}/libcharinbuf_lib.so`
  (`crate-type = ["cdylib"]`, `name = "charinbuf_lib"`)

## Raw C exports (`nm -D --defined-only`, 10 symbols)

```
00000000000011b9 T increment_counter
00000000000011d9 T decrement_counter
00000000000011f7 T multiply_counter
0000000000001216 T reset_counter
000000000000122e T is_string_empty
000000000000125d T find_char_in_buffer
0000000000001299 T create_buffer
00000000000012f7 T validate_uint16_range
0000000000001322 T apply_operation
000000000000134c T charinbuf
```

`static int counter` and `typedef int (*operation_func)(int)` are file-local /
type-only and therefore produce no dynamic symbol.

## Raw Rust exports (`nm -D --defined-only`, 10 symbols)

```
T apply_operation
T charinbuf
T create_buffer
T decrement_counter
T find_char_in_buffer
T increment_counter
T is_string_empty
T multiply_counter
T reset_counter
T validate_uint16_range
```

## Parity table

| # | C symbol | C signature (`c_src/src/lib.c`) | in Rust `.so`? | Rust item |
|---|----------|--------------------------------|----------------|-----------|
| 1 | `increment_counter`   | `int increment_counter(int value)`                                  | yes | `#[no_mangle] pub extern "C" fn increment_counter` |
| 2 | `decrement_counter`   | `int decrement_counter(int value)`                                  | yes | `#[no_mangle] pub extern "C" fn decrement_counter` |
| 3 | `multiply_counter`    | `int multiply_counter(int value)`                                   | yes | `#[no_mangle] pub extern "C" fn multiply_counter` |
| 4 | `reset_counter`       | `int reset_counter(int value)`                                      | yes | `#[no_mangle] pub extern "C" fn reset_counter` |
| 5 | `is_string_empty`     | `int is_string_empty(const char *str)`                              | yes | `#[no_mangle] pub unsafe extern "C" fn is_string_empty` |
| 6 | `find_char_in_buffer` | `char *find_char_in_buffer(const char *buffer, size_t size, char target)` | yes | `#[no_mangle] pub unsafe extern "C" fn find_char_in_buffer` |
| 7 | `create_buffer`       | `char *create_buffer(const char *initial)`                          | yes | `#[no_mangle] pub unsafe extern "C" fn create_buffer` |
| 8 | `validate_uint16_range`| `int validate_uint16_range(int value)`                             | yes | `#[no_mangle] pub extern "C" fn validate_uint16_range` |
| 9 | `apply_operation`     | `int apply_operation(operation_func op, int value)`                 | yes | `#[no_mangle] pub unsafe extern "C" fn apply_operation` |
| 10 | `charinbuf`          | `int charinbuf(int mode, int value, int opt1, int opt2)` (only symbol in `include/lib.h`) | yes | `#[no_mangle] pub unsafe extern "C" fn charinbuf` |

**Missing symbols: 0.** `nm -D` diff (C exports minus Rust exports) is empty —
verified programmatically by `tests/phase_d_symbols.rs`.

## Undefined (imported) symbols

The C `.so` imports only libc: `free malloc memchr printf puts strcpy strlen`
(plus the usual weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

The Rust `.so` imports the same libc entry points — `free malloc memchr printf
puts strcpy strlen` — plus the Rust runtime's own libc/libgcc usage
(`memcpy`, `memset`, `mmap64`, `_Unwind_*`, `dl_iterate_phdr`, …). There are
**0 undefined non-libc/non-libgcc symbols**, i.e. nothing dangling.

Note that the Rust translation deliberately calls libc `printf`/`malloc`/`free`/
`memchr`/`strlen`/`strcpy` rather than re-implementing them, so that
(a) the byte stream written to `stdout` (including `printf` formatting and stdio
buffering) is identical, and (b) pointers returned by `create_buffer` remain
`free()`-able by an arbitrary C caller.
