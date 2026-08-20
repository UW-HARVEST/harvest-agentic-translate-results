# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D --defined-only <so> | awk '$2=="T"'`.

- C  `.so`: `c_src/build/libtranslated_rust.so` (cmake, `-DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
- Rust `.so`: `target/debug/libarity_lib.so` (`crate-type = ["cdylib"]`)

## Public (text) symbols

| # | C symbol | C signature (from `src/lib.c`) | in Rust `.so` | Rust export |
|---|----------|-------------------------------|---------------|-------------|
| 1 | `apply_bitmask` | `int apply_bitmask(int value, int operation)` | YES | `#[unsafe(no_mangle)] pub extern "C"` |
| 2 | `arity` | `int arity(unsigned char len, int *params)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 3 | `arity2` | `int arity2(int p1, int p2)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 4 | `arity3` | `int arity3(int p1, int p2, int p3)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 5 | `arity4` | `int arity4(int param1, int param2, int param3, int param4)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 6 | `compare_allocations` | `int compare_allocations(int val1, int val2)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 7 | `init_matrix` | `void init_matrix(int matrix[3][4])` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 8 | `process_string` | `int process_string(const char *str)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |
| 9 | `shift_array` | `void shift_array(int *arr, int size, int positions)` | YES | `#[unsafe(no_mangle)] pub unsafe extern "C"` |

There are no macro-generated symbols, no exported data objects, no `static`
(internal-linkage) functions, and no additional translation units in `c_src`
(`CMakeLists.txt` compiles exactly one file: `src/lib.c`). Nothing was skipped
by the translation: all 9 C functions have real Rust bodies — no stubs, no
`unimplemented!()`.

## Symbol diff

```
$ comm -23 c_syms.txt rust_syms.txt     # in C, missing from Rust
<empty>
```

**RESULT: 0 missing symbols. Parity is exact (9 C symbols / 9 Rust symbols).**

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
Rust-std runtime imports (`malloc`, `free`, `memcpy`, `memmove`, `memset`,
`__errno_location`, `_Unwind_*`, `pthread_*`, `dl_iterate_phdr`, …) — zero
undefined non-libc application symbols.

## Note on the header vs. the definition (real ABI subtlety, verified)

`include/lib.h` declares `int arity(int len, int *params)` but `src/lib.c`
defines `int arity(unsigned char len, int *params)`. This is not cosmetic: the
compiled callee only ever inspects the **low 8 bits** of the incoming argument.
Verified by disassembly of the shipped C `.so`:

```
arity:
  mov  %edi,%eax
  mov  %al,-0x4(%rbp)      <-- argument truncated to 1 byte
  cmpb $0x1,-0x4(%rbp)
  ja   ...                 <-- UNSIGNED byte comparison
```

The Rust `arity` therefore takes `c_int` (matching the public header used by
callers) and truncates internally via `(len as u32 & 0xFF) as u8`, reproducing
both the truncation and the unsigned comparison. See `CONFIGS.md` rows 30–36 and
`ERRORS.md` rows E7–E9 for the differential tests that pin this down
(e.g. `len = 256` → `-1`, `len = -1` → dispatches to `arity4`).
