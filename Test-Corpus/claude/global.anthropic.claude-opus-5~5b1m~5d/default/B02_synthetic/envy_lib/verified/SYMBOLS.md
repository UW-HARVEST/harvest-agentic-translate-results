# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only c_src/build/libharvest-work-cmMBa5.so`
(filtering out `a`/`V`/`W`/`w` linker artifacts).

The C library is a single translation unit (`c_src/src/lib.c`). There is no
second module, so there is no missing/untranslated C source. `c_src/include/lib.h`
declares only `envy`, but the C source gives **external linkage** to all five
functions (none are `static`), so all five are part of the exported ABI surface
and all five must be exported by the Rust `.so`.

## Symbol table

| # | symbol | C `.so` | Rust `.so` | C signature | Rust definition |
|---|--------|---------|------------|-------------|-----------------|
| 1 | `parse_env_numeric`   | T | T | `int parse_env_numeric(const char*, int)`                 | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn parse_env_numeric` |
| 2 | `init_config_from_env` | T | T | `void init_config_from_env(struct ConfigFlags*)`           | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn init_config_from_env` |
| 3 | `perform_operation`    | T | T | `int perform_operation(int, int, struct ConfigFlags*)`     | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn perform_operation` |
| 4 | `apply_bit_operations` | T | T | `int apply_bit_operations(int, struct ConfigFlags*)`       | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn apply_bit_operations` |
| 5 | `envy`                 | T | T | `int envy(int, int, int, int)`                             | `src/lib.rs` `#[no_mangle] pub unsafe extern "C" fn envy` |

**Missing from Rust: NONE.** No stubs, no `unimplemented!()`; every symbol is a
full translation of the corresponding C function.

## Undefined (imported) symbols

The C `.so` imports from libc: `getenv`, `atoi`, `strchr`, `memcpy`, `printf`,
`fprintf`, `snprintf`, `stderr`. The Rust `.so` imports the **same** libc entry
points (declared in the `extern "C"` block in `src/lib.rs`) rather than
reimplementing them, so formatting, `atoi` overflow/parse behaviour and
stdout/stderr buffering are byte-identical by construction.

Rust additionally imports the usual Rust-runtime/libc glue
(`memcpy`/`memmove`/`memset`/`__rust_*` panic shims, `_Unwind_*` when not
`panic=abort`). These are libc/compiler-runtime symbols, not missing C API.

## Verification command

```sh
diff <(nm -D --defined-only c_src/build/*.so        | grep -v ' [aVWw] ' | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libenvy_lib.so \
                                                    | grep -v ' [aVWw] ' | awk '{print $3}' | sort)
```

Result: **empty diff** (see `tests/symbols.rs::c_and_rust_export_identical_symbols`,
which runs this check programmatically).

## ABI-relevant type layout (verified against gcc codegen)

```c
struct ConfigFlags {          /* sizeof == 4, _Alignof == 4 */
    unsigned int verbose       : 1;   /* byte 0, bit 0      */
    unsigned int debug         : 1;   /* byte 0, bit 1      */
    unsigned int optimize      : 1;   /* byte 0, bit 2      */
    unsigned int cache_enabled : 1;   /* byte 0, bit 3      */
    unsigned int log_level     : 3;   /* byte 0, bits 4..6  */
    unsigned int reserved      : 1;   /* byte 0, bit 7      */
};                            /* bytes 1..3: unnamed padding, NEVER written */

struct ProcessState {         /* sizeof == 16, _Alignof == 4 */
    struct ConfigFlags flags; /* offset 0  */
    int  base_value;          /* offset 4  */
    int  multiplier;          /* offset 8  */
    char operation;           /* offset 12; bytes 13..15 tail padding */
};
```

`objdump -d` of `init_config_from_env` confirms gcc emits **byte-sized**
read-modify-write against byte 0 only (`movzbl (%rax),%edx; and $..,%edx;
or $..,%edx; mov %dl,(%rax)`), leaving bytes 1..3 of the allocation unit
untouched. `ConfigFlags` in Rust is modelled as `#[repr(C, align(4))]
{ storage: [u8; 4] }` with byte-0-only RMW accessors, which reproduces this
exactly — including the observable fact that a caller-supplied `0xFF` in bytes
1..3 survives the call.

## Result

`nm -D` diff is **EMPTY**: all five C symbols are exported by the Rust `.so`
under the exact same names, and no symbol needed a new wrapper or a newly
translated module (the C library is a single translation unit and all of it was
already translated). No stubs exist anywhere in `src/lib.rs`.

Every symbol the Rust `.so` *imports* resolves via `dlsym(RTLD_DEFAULT)` out of
libc/libgcc/ld.so, and none of the five API names appears as an import — so the
Rust library implements the logic rather than forwarding to the C one. Asserted
by `tests/symbols.rs`:

* `c_and_rust_export_identical_symbols`
* `every_c_symbol_is_dlsym_resolvable_in_rust`
* `rust_so_has_no_unresolved_non_libc_symbols`
