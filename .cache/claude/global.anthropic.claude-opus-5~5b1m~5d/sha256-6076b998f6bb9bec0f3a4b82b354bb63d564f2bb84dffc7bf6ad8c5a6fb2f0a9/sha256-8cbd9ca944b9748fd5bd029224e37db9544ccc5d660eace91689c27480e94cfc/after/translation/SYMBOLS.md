# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-gHuGlz.so`
- Rust `.so`: `translation/target/release/libmathop_lib.so`

## Symbol table

| # | C symbol (`nm -D`) | C type | Exported by Rust `.so`? | Rust item |
|---|--------------------|--------|-------------------------|-----------|
| 1 | `is_valid_operation` | T | YES | `#[no_mangle] pub extern "C" fn is_valid_operation(c_char) -> bool` |
| 2 | `get_operation_priority` | T | YES | `#[no_mangle] pub extern "C" fn get_operation_priority(c_int) -> c_int` |
| 3 | `add_operation` | T | YES | `#[no_mangle] pub extern "C" fn add_operation(c_int,c_int,c_int) -> c_int` |
| 4 | `multiply_operation` | T | YES | `#[no_mangle] pub extern "C" fn multiply_operation(...)` |
| 5 | `subtract_operation` | T | YES | `#[no_mangle] pub extern "C" fn subtract_operation(...)` |
| 6 | `divide_operation` | T | YES | `#[no_mangle] pub extern "C" fn divide_operation(...)` |
| 7 | `modulo_operation` | T | YES | `#[no_mangle] pub extern "C" fn modulo_operation(...)` |
| 8 | `select_operation` | T | YES | `#[no_mangle] pub extern "C" fn select_operation(c_int) -> MathOperation` |
| 9 | `get_computation_timestamp` | T | YES | `#[no_mangle] pub extern "C" fn get_computation_timestamp() -> time_t` |
| 10 | `allocate_results` | T | YES | `#[no_mangle] pub extern "C" fn allocate_results(c_int) -> *mut ComputationResult` |
| 11 | `perform_computation_with_history` | T | YES | `#[no_mangle] pub unsafe extern "C" fn perform_computation_with_history(...)` |
| 12 | `mathop` | T | YES | `#[no_mangle] pub extern "C" fn mathop(c_int,c_int,c_int,c_int) -> c_int` |

## Result

**Missing symbols: 0.** Every one of the 12 dynamic symbols the C `.so` defines
is also defined by the Rust `.so` under the exact same name. No whole C module
was skipped (`c_src` contains exactly one translation unit, `src/lib.c`), and no
stubs were required.

Undefined (imported) non-libc symbols in the Rust `.so`: none — only
`calloc`, `time`, `printf` and the usual `libc`/`ld` runtime symbols, which are
exactly what the C `.so` imports as well.

Verification command (must print nothing):

```sh
diff <(nm -D --defined-only c_src/build/libharvest-work-gHuGlz.so | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libmathop_lib.so | awk '{print $3}' | sort)
```

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the crate has exactly one build configuration. The default
build is the only combination; `--no-default-features` is equivalent to it.
Symbol parity therefore holds for every feature combination trivially.
