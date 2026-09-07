# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

C library: `c_src/build/libharvest-work-iSUnEc.so`
Rust library: `translation/target/release/libmatrixsum_lib.so`

Extracted with `nm -D --defined-only <so>` (ignoring compiler/toolchain-internal
`a/A/b/B/w/W` entries and the standard `_init`/`_fini`/`_edata`-style linker
symbols that both toolchains synthesize).

## C exports vs. Rust exports

| # | symbol | type | C | Rust | notes |
|---|--------|------|---|------|-------|
| 1 | `add_element`               | T (text) | yes | yes | `int add_element(DynamicArray*, int)` |
| 2 | `calculate_matrix_checksum` | T (text) | yes | yes | `int calculate_matrix_checksum()` (unprototyped in C) |
| 3 | `expand_array`              | T (text) | yes | yes | `int expand_array(DynamicArray*)` |
| 4 | `free_array`                | T (text) | yes | yes | `void free_array(DynamicArray*)` |
| 5 | `init_array`                | T (text) | yes | yes | `DynamicArray* init_array(size_t)` |
| 6 | `matrix`                    | D (data) | yes | yes | `int matrix[3][4]`, 48 bytes, mutable global |
| 7 | `matrixsum`                 | T (text) | yes | yes | `int matrixsum(int,int,int,int)` — the only symbol in `include/lib.h` |
| 8 | `process_flags`             | T (text) | yes | yes | `int process_flags(int)` |

**Missing from Rust: none.** No module of the C source was skipped: `c_src/src/lib.c`
is the only translation unit and all 7 functions + 1 global data object it defines
with external linkage are implemented and exported by the Rust `cdylib`.

Internal-only C constructs (no symbol, nothing to export):
* `FLAG_READ` / `FLAG_WRITE` / `FLAG_EXECUTE` / `FLAG_DELETE` — object-like macros
  (`0b1`, `0b10`, `0b100`, `0b1000`); mirrored as private `const c_int` in Rust.
* `DynamicArray` — `typedef struct { int *data; size_t size; size_t capacity; }`;
  mirrored as `#[repr(C)] pub struct DynamicArray` (24 bytes on LP64: 8 + 8 + 8).

## Undefined (imported) symbols

The Rust `.so` must not import anything outside libc. `nm -D --undefined-only`
on the Rust library resolves to `malloc`, `realloc`, `free` (plus the usual
`__cxa_*` / `_ITM_*` / `__gmon_start__` weak toolchain stubs), exactly the same
allocator entry points the C object imports. **0 missing/undefined non-libc
symbols.**

Using the *same* libc allocator in Rust is required for behavioural parity: the
error paths in `init_array` / `expand_array` / `add_element` are driven purely by
`malloc`/`realloc` returning `NULL`, and `realloc(p, 0)` / `malloc(0)` have
implementation-defined-but-observable results that the tests compare directly.

## Phase D result (mechanical check, `./verify.sh`)

`verify.sh` rebuilds the C `.so`, builds the Rust `cdylib` for every feature
combination in `Cargo.toml` (there are none declared, so the default set is the
only combination) in **both** the `debug` and `release` profiles, and diffs

```
nm -D --defined-only <so> | awk '{print $2, $3}' | grep -Ev ' (_init|_fini|__bss_start|_edata|_end)$' | sort
```

between the two libraries. Result in both profiles:

```
--- symbol diff (C vs Rust) ---
  identical: 8 symbols
--- unresolved (truly undefined) symbols in Rust .so ---
  none — all imports resolve (libc/libgcc only)
    allocator imports: 3 / 3 present
```

The symbol diff is **empty**: same 8 names, same binding, same section class
(7 × `T`, 1 × `D`), and `nm -S` confirms `matrix` is `0x30` = 48 bytes in both.
