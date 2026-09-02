# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
C_SO=c_src/build/libharvest-work-RCfAKp.so
R_SO=translation/target/release/libhatch_lib.so
nm -D --defined-only "$C_SO" | awk '{print $3}' | sort -u > /tmp/c_syms.txt
nm -D --defined-only "$R_SO" | awk '{print $3}' | sort -u > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # missing from Rust
comm -13 /tmp/c_syms.txt /tmp/r_syms.txt   # extra in Rust
```

The whole library is one translation unit (`c_src/src/lib.c`); the public header
`c_src/include/lib.h` declares only `hatch`, but the C `.so` exports every
non-`static` function in the file. All 12 are part of the ABI surface and all 12
are therefore in scope for verification.

## Defined dynamic symbols

| # | C symbol (`nm -D` on C `.so`) | exported by Rust `.so` | Rust definition |
|---|-------------------------------|------------------------|-----------------|
| 1 | `add_three`                  | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn add_three` |
| 2 | `apply_operation`            | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn apply_operation` |
| 3 | `complex_calc`               | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn complex_calc` |
| 4 | `compute_with_dynamic_memory`| yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn compute_with_dynamic_memory` |
| 5 | `get_time_based_value`       | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn get_time_based_value` |
| 6 | `hatch`                      | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn hatch` |
| 7 | `increment_counter`          | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn increment_counter` |
| 8 | `manipulate_records`         | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn manipulate_records` |
| 9 | `multiply_add`               | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn multiply_add` |
|10 | `process_pointer_data`       | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_pointer_data` |
|11 | `shift_array_data`           | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn shift_array_data` |
|12 | `update_accumulator`         | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn update_accumulator` |

`static` C symbols intentionally NOT exported by either `.so` (file-scope
internal state, so absence from the Rust `.so` is correct, not a gap):
`global_counter`, `global_accumulator`.

There is no macro-generated / conditionally compiled symbol in this C source
(`grep '#if\|#ifdef' c_src/src/lib.c` → no matches), so the symbol set is not
configuration-dependent.

## Diff result

* Missing from Rust `.so`: **0** (`comm -23` produces no output).
* No symbol required translating a skipped C module; the single C translation
  unit is fully translated.
* No stubbed / `unimplemented!()` symbol exists — every export runs a real
  translation of the corresponding C body.

## Undefined (imported) symbols

The C `.so` imports only libc: `difftime`, `free`, `malloc`, `memmove`,
`memset`, `snprintf`, `time` (plus the weak `_ITM_*`, `__cxa_finalize`,
`__gmon_start__` toolchain symbols).

The Rust `.so` imports the same libc set plus Rust-runtime/libc symbols
(`_Unwind_*`, `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`,
`realloc`, `posix_memalign`, `memcpy`, `strlen`, `pthread_key_*`, `dl_iterate_phdr`,
file/`mmap`/`syscall` symbols used by `std`'s panic machinery). All are resolved
by `libc`/`libgcc_s`; there are **0 undefined non-libc symbols**.

Verified by `translation/tests/differential.rs::symbols::symbol_parity_c_vs_rust`,
which shells out to `nm -D` on both libraries and asserts the C→Rust difference
is empty.
