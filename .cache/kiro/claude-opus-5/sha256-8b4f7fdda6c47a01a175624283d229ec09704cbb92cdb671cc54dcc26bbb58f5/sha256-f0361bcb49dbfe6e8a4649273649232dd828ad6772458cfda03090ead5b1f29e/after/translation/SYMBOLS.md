# SYMBOLS.md — exported-symbol parity

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-VBsziz.so
nm -D --defined-only translation/target/release/libmathop_lib.so
```

The C library is built from exactly one translation unit (`c_src/src/lib.c`,
174 lines); `c_src/include/lib.h` declares only `mathop`. Every other symbol
below is a non-`static` definition in `lib.c` and therefore also exported.
There are no macro-generated symbols and no additional C source files, so no
module was skipped by the translation.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | Rust definition (`translation/src/lib.rs`) |
|---|--------|---------|-----------|--------------------------------------------|
| 1 | `is_valid_operation` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn is_valid_operation(c_char) -> bool` |
| 2 | `get_operation_priority` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn get_operation_priority(Operation) -> c_int` |
| 3 | `add_operation` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn add_operation(c_int, c_int, c_int) -> c_int` |
| 4 | `multiply_operation` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn multiply_operation(c_int, c_int, c_int) -> c_int` |
| 5 | `subtract_operation` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn subtract_operation(c_int, c_int, c_int) -> c_int` |
| 6 | `divide_operation` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn divide_operation(c_int, c_int, c_int) -> c_int` |
| 7 | `modulo_operation` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn modulo_operation(c_int, c_int, c_int) -> c_int` |
| 8 | `select_operation` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn select_operation(Operation) -> MathOperation` |
| 9 | `get_computation_timestamp` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn get_computation_timestamp() -> time_t` |
| 10 | `allocate_results` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn allocate_results(c_int) -> *mut ComputationResult` |
| 11 | `perform_computation_with_history` | T | T | `#[unsafe(no_mangle)] pub unsafe extern "C" fn perform_computation_with_history(...) -> c_int` |
| 12 | `mathop` | T | T | `#[unsafe(no_mangle)] pub extern "C" fn mathop(c_int, c_int, c_int, c_int) -> c_int` |

## Non-exported C entities (nothing to export)

| C entity | kind | note |
|----------|------|------|
| `Operation`, `StatusCode` | `typedef enum` | types only, no symbol |
| `ComputationResult` | `typedef struct` | type only, no symbol |
| `MathOperation` | function-pointer typedef | type only, no symbol |
| `computation_history`, `history_count` | function-local `static` in `mathop` | not exported by C (local statics); mirrored by Rust `static mut COMPUTATION_HISTORY` / `HISTORY_COUNT`, also not exported |

## Symbol diff

```
$ comm -3 <(nm -D --defined-only .../libharvest-work-VBsziz.so | awk '{print $3}' | sort) \
          <(nm -D --defined-only .../libmathop_lib.so        | awk '{print $3}' | sort)
(empty)
```

**Result: 0 symbols missing from the Rust `.so`.** No wrappers had to be added
and no C module was untranslated.

## Undefined (imported) symbols in the Rust `.so`

```
$ nm -D --undefined-only translation/target/release/libmathop_lib.so
```

All undefined symbols are libc / runtime imports (`printf`, `time`, `calloc`,
`memcpy`, `__cxa_*`/unwind glue and the `GLIBC`/`GCC` version markers). There
are no undefined *non-libc* symbols, i.e. the Rust `.so` has no dangling
reference to an untranslated helper.
