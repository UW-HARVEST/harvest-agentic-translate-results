# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-bn22YD.so
nm -D --defined-only translation/target/release/libdequantize_granule_lib.so
```

## C source inventory

`c_src` contains exactly two files that carry code:

| file | contents |
|------|----------|
| `c_src/include/lib.h` | `bs_t`, `L12_scale_info` typedefs, `dequantize_granule` prototype. No macros, no inline functions. |
| `c_src/src/lib.c` | `static uint32_t get_bits(bs_t*, int)` (internal, **not** exported), `int dequantize_granule(float*, bs_t*, L12_scale_info*, int)` (exported). |

No other translation unit exists, so no C module was skipped in translation.
`CMakeLists.txt` builds exactly one target (`SHARED` library from `src/lib.c`);
there is **no binary/driver executable**, so the "compare stdout of the two
binaries" gate is not applicable.

## Exported (dynamic, defined) symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `dequantize_granule` | `T` (0x11d1) | `T` (0x116d0) | **present in both** |

## Internal (non-exported) symbols

| symbol | C | Rust | note |
|--------|---|------|------|
| `get_bits` | `static` → local `t`, not in `nm -D` | private `unsafe fn get_bits` | Correctly *not* exported on either side. Exercised indirectly through `dequantize_granule`. |

## Symbol diff

```
$ diff <(nm -D --defined-only .../libharvest-work-bn22YD.so        | awk '{print $3}' | sort) \
       <(nm -D --defined-only .../libdequantize_granule_lib.so | awk '{print $3}' | sort)
(empty)
```

**Missing from Rust `.so`: 0.**
**Undefined non-libc symbols in Rust `.so`: 0.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section** and no optional
dependencies, therefore the only build configuration is the default one.
`--no-default-features` is equivalent to the default build. The Phase D
"every feature combination" gate collapses to a single combination, which is
verified by `scripts/check_all.sh` (which iterates the powerset of `[features]`,
falling back to `default` + `--no-default-features` when the table is absent).

## Type layout parity

| type | C | Rust | checked |
|------|---|------|---------|
| `bs_t` | `{ const uint8_t*; int; int; }` → size 16, align 8 | `#[repr(C)] { *const u8, c_int, c_int }` | yes (`layout_parity` test) |
| `L12_scale_info` | `{ float[192]; uint8_t; uint8_t; uint8_t[64]; uint8_t[64]; }` → size 900, align 4 | `#[repr(C)] { [f32;192], u8, u8, [u8;64], [u8;64] }` | yes (`layout_parity` test) |

## Verification matrix (actual results)

`./scripts/check_all.sh` builds the C `.so`, then for every
(profile x feature-combination) pair rebuilds the Rust `cdylib`, diffs the
exported symbol sets, and runs the whole differential suite:

| profile | features | symbol diff | tests |
|---------|----------|-------------|-------|
| debug   | default              | empty | 58 passed, 0 failed |
| debug   | no-default-features  | empty | 58 passed, 0 failed |
| release | default              | empty | 58 passed, 0 failed |
| release | no-default-features  | empty | 58 passed, 0 failed |

(58 = 24 Phase B + 28 Phase C + 6 Phase D.)

The `debug` profile matters independently of `release`: it enables integer
overflow checks, so it also proves that no arithmetic in the translation relies
on an implicit wrap that Rust would panic on. Every wrapping operation in
`src/lib.rs` is spelled out with `wrapping_*`.

## Undefined symbols in the Rust `.so`

```
$ nm -D --undefined-only translation/target/release/libdequantize_granule_lib.so
```

yields only glibc-versioned imports (`memcpy@GLIBC_*`, `mmap64@GLIBC_*`,
`__errno_location@GLIBC_*`, …) pulled in by the Rust standard library.
**0 undefined non-libc symbols** — asserted by
`phase_d_parity::no_unresolved_non_libc_symbols_in_rust_so`.
