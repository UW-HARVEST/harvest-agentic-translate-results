# SYMBOLS.md — Phase A symbol surface

Sources:
- C  `.so`: `c_src/build/libharvest-work-bDukX0.so` (built via CMake, `add_library(... SHARED src/lib.c)`)
- Rust `.so`: `translation/target/release/libfindrep_lib.so` (`crate-type = ["cdylib"]`, `name = "findrep_lib"`)

Command used:
```
nm -D --defined-only <so> | sort
```

## Dynamic symbol table comparison

| # | C symbol (`nm -D`) | C type | exported by Rust `.so`? | Rust item | status |
|---|--------------------|--------|-------------------------|-----------|--------|
| 1 | `add_to_accumulator`       | `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn add_to_accumulator`       | OK |
| 2 | `multiply_with_multiplier` | `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn multiply_with_multiplier` | OK |
| 3 | `subtract_from_accumulator`| `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn subtract_from_accumulator`| OK |
| 4 | `divide_multiplier`        | `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn divide_multiplier`        | OK |
| 5 | `process_octal_string`     | `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_octal_string`     | OK |
| 6 | `find_and_replace_char`    | `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn find_and_replace_char`    | OK |
| 7 | `validate_and_normalize`   | `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn validate_and_normalize`   | OK |
| 8 | `findrep`                  | `T` | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn findrep`                  | OK |

**Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C module was
left untranslated — `src/lib.c` is the only C translation unit in `CMakeLists.txt`, and all
eight of its external functions are present in the Rust `cdylib`.

## Deliberately NOT exported (correctly absent from both `.so` files)

These are `static` (internal linkage) in the C and therefore never appear in `nm -D`.
The Rust translation likewise keeps them private, so the dynamic tables stay identical.

| C declaration | Rust counterpart | note |
|---------------|------------------|------|
| `static int accumulator = 0;`      | `static mut ACCUMULATOR: c_int = 0`      | hidden mutable state |
| `static int multiplier = 1;`       | `static mut MULTIPLIER: c_int = 1`       | hidden mutable state, **initialised to 1** |
| `static int operation_count = 0;`  | `static mut OPERATION_COUNT: c_int = 0`  | hidden mutable state |
| `static operation_func operations[4]` | `static OPERATIONS: [OperationFunc; 4]` | dispatch table |
| `typedef int (*operation_func)(int,int)` | `type OperationFunc` | type only, no symbol |
| `typedef void (*string_processor)(char*,int)` | `type StringProcessor` | type only, unused in both |

## Undefined / imported symbols

The C `.so` imports `strlen`, `strcpy`, `memchr`, `sprintf` from libc. The Rust `.so`
reimplements these internally (`c_strlen`, `c_strcpy_from`, `c_memchr`,
`format_octal_message`) so it imports fewer libc symbols. This is not a parity
violation: the requirement is that every symbol the C `.so` *defines* is also
*defined* by the Rust `.so`, which holds. There are 0 missing/undefined non-libc
symbols in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly one
build configuration. `--no-default-features` is still exercised in Phase D and is
equivalent to the default build.
