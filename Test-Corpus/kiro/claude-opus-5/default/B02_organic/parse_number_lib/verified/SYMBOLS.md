# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## Source of truth

The C library is a single translation unit, `c_src/src/lib.c`, built by
`c_src/CMakeLists.txt` as `add_library(driver SHARED src/lib.c)`.
There is no second module, so there is no un-translated C file.

## `nm -D --defined-only` on the C `.so`

```
$ nm -D --defined-only c_src/build/libdriver.so
0000000000001139 T parse_number
```

## `nm -D --defined-only` on the Rust `.so`

```
$ nm -D --defined-only translation/target/release/libdriver.so
0000000000011980 T parse_number
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | action |
|---|--------|-----------|---------------|--------|
| 1 | `parse_number` | T (global text) | T (global text) | none — exported via `#[unsafe(no_mangle)] pub unsafe extern "C"` |

**Missing from Rust: none.** Symbol diff is empty.

## Undefined (imported) symbols

The C `.so` imports only libc: `malloc`, `free`, `memcpy`, `strtod`
(plus the usual `_ITM_*` / `__gmon_start__` / `__cxa_finalize` weak stubs).
The Rust `.so` imports the same `strtod` plus the Rust allocator's
`malloc`/`free`/`realloc`/`memcpy` and libc/unwind runtime symbols. All
undefined symbols on both sides resolve against `libc`/`libgcc_s` — there are
**0 missing/undefined non-libc symbols** in the Rust `.so`.

```
$ ldd -r translation/target/release/libdriver.so   # no "undefined symbol" lines
```

## Types crossing the ABI (must match `c_src/include/lib.h`)

| C declaration | Rust declaration | size / align (x86-64) |
|---|---|---|
| `typedef int cJSON_bool` | `pub type cJSON_bool = c_int` | 4 / 4 |
| `struct parse_buffer { const unsigned char *content; size_t length; size_t offset; size_t depth; }` | `#[repr(C)] pub struct parse_buffer` | 32 / 8 |
| `struct cJSON { int type; int valueint; double valuedouble; }` | `#[repr(C)] pub struct cJSON` (`type` → `type_`) | 16 / 8 |

Struct sizes/offsets are asserted at runtime in
`tests/differential.rs::abi_struct_layout_matches_c`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and no optional
dependencies, so the only build configuration is the default one. The
"repeat for every feature combination" requirement is satisfied by the single
combination `--no-default-features` ≡ default (verified in
`check_all_feature_combos.sh`).
