# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-h08ZJq.so`
  (CMake names the library after the parent directory, see `c_src/CMakeLists.txt`:
  `cmake_path(GET parent FILENAME project_name)`), built with
  `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
* Rust `.so`: `translation/target/release/libhatch_lib.so`
  (`[lib] name = "hatch_lib"`, `crate-type = ["cdylib"]`), built with
  `cargo build --release`

## Source inventory

The whole C library is a single translation unit: `c_src/src/lib.c` (177 lines).
`c_src/include/lib.h` declares exactly one function (`hatch`), but the C file has
**no** `static` functions besides the two file-scope variables, so every function
in `lib.c` receives external linkage and lands in the dynamic symbol table.
There is no other C source file, so no module was skipped.

| C construct | linkage | exported? |
|---|---|---|
| `static int global_counter` | internal | no (data, not a symbol in `.dynsym`) |
| `static int global_accumulator` | internal | no |
| `void increment_counter(int,int)` | external | yes |
| `void update_accumulator(int,int)` | external | yes |
| `int apply_operation(operation_func,int,int,int)` | external | yes |
| `int add_three(int,int,int)` | external | yes |
| `int multiply_add(int,int,int)` | external | yes |
| `int complex_calc(int,int,int)` | external | yes |
| `void shift_array_data(int*,int,int)` | external | yes |
| `int process_pointer_data(int*,int)` | external | yes |
| `int compute_with_dynamic_memory(int,int)` | external | yes |
| `int get_time_based_value(int)` | external | yes |
| `int manipulate_records(DataRecord*,int,int)` | external | yes |
| `int hatch(int,int,int,int)` | external | yes |

## Symbol parity table

12 symbols exported by the C `.so`; all 12 exported by the Rust `.so` with the
exact same names. No macro-generated symbols exist in this library.

| # | symbol | C `.so` | Rust `.so` | Rust definition |
|---|--------|---------|------------|-----------------|
| 1 | `increment_counter`          | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn increment_counter` |
| 2 | `update_accumulator`         | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn update_accumulator` |
| 3 | `apply_operation`            | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn apply_operation` |
| 4 | `add_three`                  | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn add_three` |
| 5 | `multiply_add`               | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn multiply_add` |
| 6 | `complex_calc`               | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn complex_calc` |
| 7 | `shift_array_data`           | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn shift_array_data` |
| 8 | `process_pointer_data`       | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_pointer_data` |
| 9 | `compute_with_dynamic_memory`| T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn compute_with_dynamic_memory` |
| 10 | `get_time_based_value`      | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn get_time_based_value` |
| 11 | `manipulate_records`        | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn manipulate_records` |
| 12 | `hatch`                     | T | T | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn hatch` |

**Missing from Rust: none.** Nothing was stubbed; every symbol is a real
translation of the corresponding C body.

## Undefined (imported) symbols

The C `.so` imports only libc: `malloc`, `free`, `memmove`, `memset`, `time`,
`snprintf` (plus the usual `__*` glibc/CRT helpers). The Rust `.so` declares and
imports exactly the same libc entry points via `extern "C"` (see `src/lib.rs`)
so that allocation and byte-moving semantics are literally the same code. There
are **0 undefined non-libc symbols** in the Rust `.so`.

## ABI facts pinned down (verified on this host, x86-64 Linux / LP64)

| item | C (`gcc 11.5.0`) | Rust |
|---|---|---|
| `sizeof(int)` | 4 | `c_int` = `i32` |
| `sizeof(time_t)` | 8 | `type time_t = i64` |
| `sizeof(size_t)` | 8 | `usize` |
| `sizeof(DataRecord)` | 48 | `#[repr(C)] DataRecord` = 48 |
| `_Alignof(DataRecord)` | 8 | 8 |
| `offsetof(id, value, timestamp, name)` | 0, 4, 8, 16 | 0, 4, 8, 16 |

The Rust side of that table is asserted by `cfg_datarecord_layout_matches_c` in
`tests/valid_paths.rs`, and the *agreement* with the C side is what the full
post-`memmove` byte-image comparisons in `cfg_c22`–`cfg_c25` actually prove: a
wrong element size or field offset shows up as a differing buffer byte (see the
`records_memmove_wrong_elem_size` and `records_reads_id_not_value` mutants in
`mutation_check.sh`, both killed).

## Reproduce

```sh
nm -D --defined-only c_src/build/libharvest-work-h08ZJq.so | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libhatch_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must be empty
```

`translation/check_symbols.sh` automates this; `tests/symbols.rs` asserts it
from the test suite.
