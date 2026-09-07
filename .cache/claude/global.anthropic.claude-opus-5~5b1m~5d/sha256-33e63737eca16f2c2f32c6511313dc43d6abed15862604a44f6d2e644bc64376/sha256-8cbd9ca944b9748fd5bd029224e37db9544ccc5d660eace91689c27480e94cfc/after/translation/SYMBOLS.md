# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

## Build artifacts

| side | path |
|------|------|
| C    | `c_src/build/libharvest-work-kwUWsC.so` |
| Rust | `translation/target/release/libhalf2float_lib.so` |

## C exported dynamic symbols (`nm -D --defined-only`)

| addr | type | symbol |
|------|------|--------|
| `00000000000010f9` | `T` | `half2float` |

Total: **1** exported symbol.

## Rust exported dynamic symbols (`nm -D --defined-only`)

| addr | type | symbol |
|------|------|--------|
| `0000000000013820` | `T` | `half2float` |

Total: **1** exported symbol.

## Diff

```
C symbols not exported by Rust:  (none)
Rust-only extra symbols:         (none)
```

**Symbol diff is EMPTY.** ✅

## Notes on non-exported C entities

The C translation unit also defines three `static` (internal-linkage) tables.
They are NOT dynamic symbols and correctly have no exported counterpart in
Rust; they are `static` items in `translation/src/lib.rs`:

| C entity | linkage | Rust counterpart | table contents verified |
|----------|---------|------------------|--------------------------|
| `static uint32_t m__mantissa[2048]` | internal | `static m__mantissa: [u32; 2048]` | ✅ all 2048 entries identical |
| `static uint16_t m__offset[64]`     | internal | `static m__offset: [u16; 64]`     | ✅ all 64 entries identical |
| `static uint32_t m__exponent[64]`   | internal | `static m__exponent: [u32; 64]`   | ✅ all 64 entries identical |

Table equality was checked mechanically by parsing every `0x...` literal out of
both `c_src/src/lib.c` and `translation/src/lib.rs` and comparing element-wise
(lengths 2048/64/64 matched; zero differing elements).

## Undefined (imported) symbols

The Rust `.so` imports only libc/`std` runtime symbols. There are **0 missing
or undefined non-libc symbols**. The whole C translation unit
(`c_src/src/lib.c`, 376 lines) is fully translated — no module was skipped.

## Verified under both build profiles

`run_all.sh` re-checks parity for each cdylib artifact:

```
[release] C exports: 1, Rust exports: 1
[release] symbol diff EMPTY ✅
[release] 0 undefined non-libc symbols ✅
[debug]   C exports: 1, Rust exports: 1
[debug]   symbol diff EMPTY ✅
[debug]   0 undefined non-libc symbols ✅
```

All undefined symbols in the Rust `.so` are `@GLIBC`/`@GCC`-versioned libc and
runtime imports, plus the standard weak `_ITM_*registerTMCloneTable` and
`__gmon_start__` stubs — i.e. zero non-libc undefined symbols.

## Completeness

- [x] Every C-exported symbol is exported by Rust with the exact same name.
- [x] `half2float` signature matches: `float half2float(uint16_t)` ⇔
      `extern "C" fn half2float(h: u16) -> f32`.
- [x] 0 missing/undefined non-libc symbols in the Rust `.so`.
