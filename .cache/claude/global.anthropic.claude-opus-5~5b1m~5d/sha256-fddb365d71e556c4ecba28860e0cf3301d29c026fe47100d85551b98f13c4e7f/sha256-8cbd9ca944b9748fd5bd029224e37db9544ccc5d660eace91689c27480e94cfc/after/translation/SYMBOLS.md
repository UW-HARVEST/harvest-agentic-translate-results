# SYMBOLS.md — Public symbol surface

Source of truth: `nm -D --defined-only` on the C shared library
`c_src/build/libharvest-work-18lsCO.so`, compared against the Rust cdylib
`translation/target/release/libconfusion_lib.so`.

## C `.so` exported symbols (all)

| # | symbol | C signature (`c_src/src/lib.c`) | exported by Rust `.so`? | Rust item |
|---|--------|----------------------------------|--------------------------|-----------|
| 1 | `create_state`   | `ProcessState* create_state(int initial_val, int capacity)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn create_state` |
| 2 | `destroy_state`  | `void destroy_state(ProcessState* state)`                   | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn destroy_state` |
| 3 | `process_buffer` | `int process_buffer(ProcessState* state, char target)`       | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_buffer` |
| 4 | `update_flags`   | `void update_flags(ProcessState* state, int param)`          | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn update_flags` |
| 5 | `confuse_types`  | `int confuse_types(ProcessState* state, int operation)`       | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn confuse_types` |
| 6 | `confusion`      | `int confusion(int, int, int, int)` (the only symbol declared in `include/lib.h`) | YES | `#[unsafe(no_mangle)] pub unsafe extern "C" fn confusion` |

No macro-generated exports exist: `STRINGIFY`, `DEBUG_VAR` and `LOG_OPERATION`
are expression/statement macros that expand into `printf` calls inside existing
functions; they define no symbols.

There are no `static` (internal-linkage) helpers in the C source, so the table
above is the complete translation unit surface. Every C source file
(`src/lib.c` is the only one) is fully translated — no module was skipped and
no symbol is a stub.

## Symbol diff

```
$ comm -23 c_symbols.txt rust_symbols.txt      # in C, missing from Rust
(empty)
```

**0 missing symbols.**

## Undefined (imported) symbols

The Rust `.so` imports only libc / libgcc-unwind / Rust-std runtime symbols
(`malloc`, `free`, `printf`, `snprintf`, `strlen`, `memchr`, plus std runtime:
`__cxa_*`, `_Unwind_*`, `pthread_key_*`, `mmap64`, ...). There are **0 undefined
non-libc/non-runtime symbols**, i.e. nothing is left dangling.

The C `.so` additionally imports `puts`: GCC rewrites the argument-less
`printf("literal\n")` calls into `puts("literal")`. This is a compiler
optimisation only — the bytes written to stdout are identical, which the
stdout-capturing differential tests confirm.

## Type layout parity (needed because `ProcessState` crosses the FFI boundary)

| C type | layout | Rust model |
|--------|--------|------------|
| `PackedFlags` (bit-fields `flag1:1, flag2:1, flag3:1, counter:5, mode:3, status:5, reserved:16`) | one 32-bit allocation unit, SysV LE order: flag1@0, flag2@1, flag3@2, counter@3..8, mode@8..11, status@11..16, reserved@16..32 | `#[repr(C)] struct PackedFlags { bits: u32 }` + `bitfield!` accessors |
| `TypeConfusion` (`union { int; float; unsigned; char[4] }`) | 4 bytes, align 4 | `#[repr(C)] struct TypeConfusion { raw: u32 }` + reinterpreting accessors |
| `ProcessState` | `sizeof == 24`, `alignof == 8`; offsets flags 0, data 4, buffer 8, capacity 16 | `#[repr(C)] struct ProcessState { flags, data, buffer: *mut c_char, capacity: c_int }` |

Verified at runtime by `tests/differential.rs::phase_a_struct_layout_parity`,
which has C `create_state` build a state and reads the fields back through the
Rust-side layout (and vice versa).
