# SYMBOLS.md — Exported symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only` on both shared objects.

- C  `.so`: `c_src/build/libtranslated_rust.so`
  (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
- Rust `.so`: `target/debug/libmatrixsum_lib.so`
  (`[lib] name = "matrixsum_lib"`, `crate-type = ["cdylib"]`)

## Build-time configuration surface

- `Cargo.toml` has **no `[features]` section** → exactly ONE valid feature
  combination (the empty set). `--no-default-features` and the default build are
  therefore identical. Both were checked (Phase D).
- `c_src/CMakeLists.txt` has no `option()`, no `add_definitions`, no `#ifdef`
  driven configuration; a single `SHARED` target from one TU (`src/lib.c`).
  There are no conditional-compilation axes on either side.

## Symbol table comparison

| # | symbol | C type | Rust type | in C `.so` | in Rust `.so` | status |
|---|--------|--------|-----------|-----------|---------------|--------|
| 1 | `init_array`                | `T` (text) | `T` | yes | yes | MATCH |
| 2 | `expand_array`              | `T` (text) | `T` | yes | yes | MATCH |
| 3 | `add_element`               | `T` (text) | `T` | yes | yes | MATCH |
| 4 | `free_array`                | `T` (text) | `T` | yes | yes | MATCH |
| 5 | `process_flags`             | `T` (text) | `T` | yes | yes | MATCH |
| 6 | `calculate_matrix_checksum` | `T` (text) | `T` | yes | yes | MATCH |
| 7 | `matrixsum`                 | `T` (text) | `T` | yes | yes | MATCH |
| 8 | `matrix`                    | `D` (data, 48 B) | `D` (data, 48 B) | yes | yes | MATCH |

`matrix` is `int matrix[3][4]` — a non-`static` global in C, so it is an
exported, writable `.data` symbol. The Rust side mirrors this with
`#[unsafe(no_mangle)] pub static mut matrix: [[c_int; 4]; 3]`, same size
(48 bytes) and same section class (`D`). This matters because
`calculate_matrix_checksum` reads it live, so an external caller can mutate the
symbol between calls and change the result of both it and `matrixsum`
(exercised in the differential tests).

## Diff result

```
comm -23 c_syms.txt r_syms.txt   # in C, missing from Rust
    <empty>
comm -13 c_syms.txt r_syms.txt   # extra in Rust
    <empty>
counts: C=8  Rust=8
```

**0 symbols missing from the Rust `.so`. 0 extra. No stubs were added** —
every symbol is backed by a real translation of the corresponding C function.

## Undefined (imported) symbols

The C `.so` imports only `malloc`, `realloc`, `free` (+ CRT/ITM boilerplate).
The Rust `.so` imports those same three allocator symbols — the translation
deliberately binds libc `malloc`/`realloc`/`free` rather than using Rust's
allocator, so that glibc-specific behaviours are reproduced exactly (notably
`malloc(0)` returning a unique non-NULL pointer and `realloc(p, 0)` freeing `p`
and returning NULL — both are reachable through this API and are covered in
`ERRORS.md`).

All remaining Rust undefined symbols are libc / libgcc runtime imports pulled in
by `std` and the panic unwinder (`_Unwind_*`, `__cxa_*`, `mem*`, `pthread_key_*`,
file/`syscall` shims). `ldd` resolves the Rust `.so` fully against
`libgcc_s.so.1`, `libc.so.6` and the dynamic loader — **no missing/undefined
non-libc symbols**.

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust.
