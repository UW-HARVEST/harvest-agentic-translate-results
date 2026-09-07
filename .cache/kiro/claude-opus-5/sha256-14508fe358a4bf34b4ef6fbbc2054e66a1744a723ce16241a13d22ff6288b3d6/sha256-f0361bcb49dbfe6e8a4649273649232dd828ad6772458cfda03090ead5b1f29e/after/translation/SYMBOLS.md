# Phase A.1 — Symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-HnSgP8.so
nm -D --defined-only translation/target/release/libcharinbuf_lib.so
```

The C build is a single translation unit (`c_src/src/lib.c`, per
`c_src/CMakeLists.txt`), so there is exactly one C module and no macro-generated
symbol names. `c_src/include/lib.h` declares only `charinbuf`; the other nine
symbols have external linkage but no prototype in the public header, and all ten
appear in `nm -D`.

## Exported symbol table

| # | C symbol | C type | Rust `.so` exports it | Rust definition site |
|---|----------|--------|-----------------------|----------------------|
| 1 | `increment_counter`   | T | yes | `src/counter.rs` |
| 2 | `decrement_counter`   | T | yes | `src/counter.rs` |
| 3 | `multiply_counter`    | T | yes | `src/counter.rs` |
| 4 | `reset_counter`        | T | yes | `src/counter.rs` |
| 5 | `is_string_empty`      | T | yes | `src/helpers.rs` |
| 6 | `find_char_in_buffer`  | T | yes | `src/helpers.rs` |
| 7 | `create_buffer`        | T | yes | `src/helpers.rs` |
| 8 | `validate_uint16_range`| T | yes | `src/helpers.rs` |
| 9 | `apply_operation`      | T | yes | `src/helpers.rs` |
| 10 | `charinbuf`           | T | yes | `src/charinbuf.rs` |

`static int counter` and `typedef int (*operation_func)(int)` have internal
linkage / are types, so they are correctly absent from both `.so` files. The
counter state lives in `src/counter.rs` as a non-exported `UnsafeCell<c_int>`.

## Symbol diff

```
comm -23 <(C defined symbol names) <(Rust defined symbol names)   ->   (empty)
```

**Missing symbols: 0.** No module of `lib.c` was skipped, so no additional C
source had to be translated and no stub was introduced.

## Undefined symbols

The C `.so` needs only glibc: `free malloc memchr printf puts strcpy strlen`
(plus the usual weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).
Note `puts`: GCC rewrites the argument-less `printf("...\n")` calls into
`puts`, which is why it appears even though `lib.c` never names it.

The Rust `.so` needs the same glibc set plus the Rust `std` runtime's own libc
and `_Unwind_*` imports (`abort`, `calloc`, `dl_iterate_phdr`, `mmap64`,
`pthread_key_create`, `write`, ...). Every Rust undefined symbol resolves out of
`libc`/`libgcc_s`; there are **0 undefined non-libc symbols**, i.e. nothing that
would have to be supplied by a missing translated module.

Verified with `ldd -r` producing no "undefined symbol" lines for either object.

## Machine-checked

`d1_symbol_parity` in `tests/phase_d_parity.rs` re-derives both symbol lists
with `nm -D --defined-only --format=posix` at test time, asserts the C surface is
exactly the ten names above, and fails if any of them is absent from the Rust
`.so`. `d2_every_symbol_is_callable_through_the_rust_so` then resolves and
*invokes* all ten through `libloading`, so an exported-but-hollow symbol cannot
satisfy the gate. `run_all_phases.sh` repeats the `comm -23` diff for every
build configuration.

Result: **10 of 10 C symbols exported by the Rust `.so`, 0 missing, 0 undefined
non-libc symbols** (`ldd -r` reports no undefined symbols for either object).
