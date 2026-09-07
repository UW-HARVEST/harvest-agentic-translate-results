# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-lJlo6Z.so
nm -D --defined-only translation/target/release/libbuffapp_lib.so
```

The whole C library is one translation unit: `c_src/src/lib.c` (155 lines).
The public header `c_src/include/lib.h` declares only `buffapp`, but the `.so`
exports all six non-static functions, so all six are part of the ABI surface
and all six must be exported by the Rust `.so`.

## Exported symbol table

| # | C symbol (`nm -D`) | C type | in Rust `.so`? | Rust definition |
|---|--------------------|--------|----------------|-----------------|
| 1 | `append_to_buffer` | `T` | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn append_to_buffer` |
| 2 | `buffapp`          | `T` | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn buffapp` |
| 3 | `create_buffer`    | `T` | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn create_buffer` |
| 4 | `destroy_buffer`   | `T` | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn destroy_buffer` |
| 5 | `get_operation_name` | `T` | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub extern "C" fn get_operation_name` |
| 6 | `perform_operation` | `T` | YES | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn perform_operation` |

Types (`static` / file-local in C, therefore NOT exported and not required):
none — `lib.c` has no `static` functions and no global variables.

## Diff result

```
comm -23 <(nm -D --defined-only $C_SO | awk '{print $3}' | sort) \
         <(nm -D --defined-only $R_SO | awk '{print $3}' | sort)
# -> (empty)
```

**0 missing symbols.** No module of the C source was skipped; `lib.c` is
translated in full (`StringBuffer` struct + 6 functions).

## Undefined symbols in the Rust `.so`

All undefined symbols in the Rust `.so` are libc / unwinder imports
(`malloc`, `realloc`, `free`, `strlen`, `strcpy`, `strcmp`, `sprintf`,
`printf`, plus the Rust std/`_Unwind_*`/glibc startup set). There are **0
missing/undefined non-libc symbols**.

The Rust translation deliberately calls the platform `malloc`/`realloc`/`free`
and `sprintf`/`printf` so that (a) buffers are interchangeable with the C
library's buffers, (b) allocation-failure behaviour is identical, and (c) the
formatted bytes and stdout buffering are byte-identical.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. Verified with:

```sh
grep -n '\[features\]' translation/Cargo.toml   # -> no match
```

Phase D's "repeat for every feature combination" therefore collapses to the
single default combination; it is still run explicitly as
`cargo test --no-default-features` in addition to `cargo test`.

## Phase D verification record

```
$ comm -23 <(nm -D --defined-only c_src/build/libharvest-work-lJlo6Z.so | awk '{print $NF}' | sort) \
           <(nm -D --defined-only translation/target/release/libbuffapp_lib.so | awk '{print $NF}' | sort)
(empty)
```

Checked for all four artifacts by `translation/run_all.sh` and enforced
automatically by `tests/symbol_parity.rs`:

* `phase_d_every_c_symbol_is_exported_by_rust` — set difference must be empty
  (and pins the C surface at exactly 6 symbols, so a future C addition fails
  the test rather than passing silently).
* `phase_d_rust_so_has_no_undefined_non_libc_symbols` — 0 undefined non-libc
  symbols.
* `phase_d_harness_loads_two_distinct_shared_objects` — proves the tests really
  load two different `.so` files and never call Rust directly.

Result: **0 missing symbols, 0 undefined non-libc symbols**, for
`target/{release,debug}/libbuffapp_lib.so` under both the default and the
`--no-default-features` configuration.
