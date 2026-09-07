# SYMBOLS.md — exported-symbol parity

Derived mechanically from `nm -D` on both shared objects.

- C:    `c_src/build/libharvest-work-MX0GHr.so`
- Rust: `translation/target/release/libread_side_info_lib.so`

## C `.so` defined dynamic symbols

```
$ nm -D --defined-only c_src/build/libharvest-work-MX0GHr.so
T read_side_info
```

The C translation unit (`c_src/src/lib.c`) contains exactly two functions:

| C function | linkage | exported? |
|------------|---------|-----------|
| `read_side_info` | external | yes — `T read_side_info` |
| `get_bits` | `static` (internal) | no — not in `nm -D`, by design |

`c_src/include/lib.h` declares exactly one function (`read_side_info`) and two
types (`bs_t`, `L3_gr_info_t`). There are no macros that generate additional
symbols, no additional `.c` files in `CMakeLists.txt` (`add_library(... src/lib.c)`),
and no data objects with external linkage (the three `g_scf_*` tables are
function-local `static const`, so they have local linkage and no dynamic symbol).

## Rust `.so` defined dynamic symbols

```
$ nm -D --defined-only translation/target/release/libread_side_info_lib.so
T read_side_info
```

## Parity table

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `read_side_info` | `T` | `T` | MATCH |

**Symbol diff (C-exported minus Rust-exported): EMPTY.**

No symbol needed a new `#[no_mangle]` wrapper, and no C module was left
untranslated — `src/lib.c` is the only source file and both of its functions
(`read_side_info`, plus the `static` helper `get_bits`, translated as a private
Rust `fn`) are present in `translation/src/lib.rs`.

## Undefined (imported) symbols

The C `.so` imports only libc/toolchain symbols:

```
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
__cxa_finalize@GLIBC_2.2.5
__gmon_start__
```

The Rust `.so` imports only libc / Rust-runtime symbols. **0 missing or
undefined non-libc symbols in the Rust `.so`.**

## Internal (non-exported) table data — verified separately

Because the three `g_scf_*` tables are `static` inside the C function they are
not comparable via `nm`. They are instead verified two ways:

1. Textually, by parsing the array initialisers out of both sources and
   diffing them (including C's implicit zero-fill of short row initialisers):
   `g_scf_long` 8x23, `g_scf_short` 8x40, `g_scf_mixed` 8x40 — all IDENTICAL.
2. Behaviourally, through the FFI boundary: every differential test dereferences
   the `sfbtab` pointer each library returns and compares the pointed-to bytes
   (23 bytes for the long table, 40 for short/mixed). See
   `tests/differential.rs::cmp_gr`.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so there is exactly one build configuration
(`--no-default-features` and the default build are identical). There is also no
`[[bin]]` target and no `src/main.rs`, so the project builds **no binary
driver** — the "compare C and Rust stdout" clause is not applicable.
