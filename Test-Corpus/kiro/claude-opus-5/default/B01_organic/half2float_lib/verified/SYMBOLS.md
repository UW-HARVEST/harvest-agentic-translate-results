# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C `.so`:    `c_src/build/libharvest-work-EoMR1V.so`
- Rust `.so`: `translation/target/release/libhalf2float_lib.so`

Reproduce with:

```sh
./check_symbols.sh
```

## C source inventory (completeness check)

`c_src` contains exactly one translation unit and one public header:

| C file | contents | translated? |
|--------|----------|-------------|
| `c_src/include/lib.h` | `#include <stdint.h>`; declares `float half2float(uint16_t h);` | yes |
| `c_src/src/lib.c` | 3 `static` tables (`m__mantissa[2048]`, `m__offset[64]`, `m__exponent[64]`) + `half2float` | yes |

No C module/file is missing from the Rust crate, so no additional translation
work was required for symbol parity.

## Defined (exported) symbols

`nm -D --defined-only`, filtering out the linker/toolchain weak symbols that
are not part of the library API (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`).

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `half2float` | `T` | `T` | `#[unsafe(no_mangle)] pub extern "C" fn half2float(h: c_ushort) -> c_float` |

**Symbol diff (C exported minus Rust exported): EMPTY.**

The three lookup tables are `static` in C (internal linkage) and are therefore
*not* exported by the C `.so`. The Rust equivalents are private `static`s and
are likewise not exported. This matches.

Table contents were additionally verified value-by-value against the C source
(all 2048 + 64 + 64 entries identical) — see `check_symbols.sh`.

## Undefined symbols

The C `.so` imports nothing but weak toolchain symbols. The Rust `.so` imports
only libc / libgcc-unwind symbols (`malloc`, `memcpy`, `_Unwind_*`,
`pthread_key_create`, …) pulled in by the Rust standard library. There are
**0 missing/undefined non-libc symbols** in the Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table, so the only build
configuration is the default (empty) feature set. `check_features.sh`
enumerates the feature list mechanically and loops `cargo check` / `cargo test`
over every combination, so this stays correct if features are added later.

## Completion status

- [x] `nm -D` shows 0 missing/undefined non-libc symbols in Rust
- [x] Every symbol exported by the C `.so` is exported by the Rust `.so`, same name
- [x] No stubbed / `unimplemented!()` symbols
