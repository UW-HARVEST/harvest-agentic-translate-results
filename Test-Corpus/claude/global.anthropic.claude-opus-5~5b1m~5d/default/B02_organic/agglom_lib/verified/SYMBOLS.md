# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` on the C shared object
`c_src/build/libharvest-work-NQTXuV.so`, compared against the Rust cdylib
`translation/target/release/libagglom_lib.so`.

Regenerate with:

```sh
nm -D --defined-only c_src/build/libharvest-work-NQTXuV.so      | awk '{print $3}' | sort > /tmp/c.txt
nm -D --defined-only translation/target/release/libagglom_lib.so | awk '{print $3}' | sort > /tmp/r.txt
comm -23 /tmp/c.txt /tmp/r.txt   # must be empty
```

## Symbol table

| # | C symbol | C signature (`c_src/src/lib.c`) | Rust export | status |
|---|----------|--------------------------------|-------------|--------|
| 1 | `agglom` | `double agglom(float x7, int x2, uint64_t x2, uint32_t, tflac_u32 x3, float x8, uint16_t, float x9)` | `agglom` | ✅ present |
| 2 | `c2V` | `c2v c2V(float x, float y)` | `c2V` | ✅ present |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v a, c2v b)` | `c2Maxv` | ✅ present |
| 4 | `c2Minv` | `c2v c2Minv(c2v a, c2v b)` | `c2Minv` | ✅ present |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` | `c2Clampv` | ✅ present |
| 6 | `c2Sub` | `c2v c2Sub(c2v a, c2v b)` | `c2Sub` | ✅ present |
| 7 | `c2Dot` | `float c2Dot(c2v a, c2v b)` | `c2Dot` | ✅ present |
| 8 | `c2CircletoCircle` | `int c2CircletoCircle(c2Circle A, c2Circle B)` | `c2CircletoCircle` | ✅ present |
| 9 | `c2CircletoAABB` | `int c2CircletoAABB(c2Circle A, c2AABB B)` | `c2CircletoAABB` | ✅ present |
| 10 | `c2AABBtoAABB` | `int c2AABBtoAABB(c2AABB A, c2AABB B)` | `c2AABBtoAABB` | ✅ present |
| 11 | `f2` | `int f2(const void *A, C2_TYPE typeA, const void *B, C2_TYPE typeB)` | `f2` | ✅ present |
| 12 | `f3` | `int f3(int v1, int v2)` | `f3` | ✅ present |
| 13 | `f4` | `double f4(cn_rnd_t *rnd)` | `f4` | ✅ present |
| 14 | `f5` | `uint32_t f5(uint32_t a)` | `f5` | ✅ present |
| 15 | `f7` | `tflac_u32 f7(tflac_u32 blocksize, tflac_u32 channels, tflac_u32 bitdepth)` | `f7` | ✅ present |
| 16 | `f9` | `lm_vec2 f9(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p)` | `f9` | ✅ present |
| 17 | `f10` | `float f10(uint16_t h)` | `f10` | ✅ present |
| 18 | `f11` | `void f11(float *dest, const float *src)` | `f11` | ✅ present |
| 19 | `f12` | `void f12(float *dest, const float *src)` | `f12` | ✅ present |
| 20 | `f13` | `void f13(float *dest, const float *src)` | `f13` | ✅ present |

**Missing from Rust `.so`: NONE (symbol diff is empty).**

## Non-exported C entities (`static` — intentionally not in `nm -D`)

These are `static` in the C translation unit and therefore have no dynamic
symbol. They are still translated in Rust (as private items) because the public
functions depend on them; they must NOT be exported (that would be a spurious
extra symbol).

| C entity | kind | Rust counterpart |
|----------|------|------------------|
| `tflac_crc16_tables[8][256]` | `static const tflac_u16` | `tables::tflac_crc16_tables` (dead in C too — never read) |
| `m__mantissa[2048]` | `static uint32_t` | `tables::m__mantissa` |
| `m__offset[64]` | `static uint16_t` | `tables::m__offset` |
| `m__exponent[64]` | `static uint32_t` | `tables::m__exponent` |
| `cn_rnd_next` | `static uint64_t(cn_rnd_t*)` | `cn_rnd_next` (private) |
| `lm_v2` | `static lm_vec2(float,float)` | `lm_v2` (private) |
| `lm_sub2` | `static lm_vec2(lm_vec2,lm_vec2)` | `lm_sub2` (private) |
| `lm_dot2` | `static float(lm_vec2,lm_vec2)` | inlined into `f9_impl` |

All four data tables were verified **element-for-element equal** to the C
initialisers by mechanically extracting every `0x…` literal from
`c_src/src/lib.c` and `translation/src/tables.rs`
(2048 / 2048 / 64 / 64 values, all EQUAL).

## Types / ABI notes

| C type | layout | Rust |
|--------|--------|------|
| `C2_TYPE` (enum, values 0,1) | GCC: `unsigned int`, 4 bytes | `c_uint` |
| `c2v` | `{float x; float y;}` — 8 bytes, returned in XMM0 | `#[repr(C)] c2v` |
| `c2Circle` | `{c2v p; float r;}` — 12 bytes | `#[repr(C)] c2Circle` |
| `c2AABB` | `{c2v min; c2v max;}` — 16 bytes | `#[repr(C)] c2AABB` |
| `lm_vec2` | `{float x, y;}` — 8 bytes | `#[repr(C)] lm_vec2` |
| `cn_rnd_t` | `{uint64_t state[2];}` — 16 bytes | `#[repr(C)] cn_rnd_t` |
| `tflac_u32` | `uint32_t` | `u32` |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only build configuration is the default one (`--no-default-features` is
equivalent). Verified by `grep -n '\[features\]' translation/Cargo.toml` →
no match. Phase D's "every feature combination" therefore collapses to the
single default combination.

## Verification result

```
$ nm -D --defined-only c_src/build/libharvest-work-NQTXuV.so | awk '{print $3}' | sort > c.txt
$ nm -D --defined-only translation/target/release/libagglom_lib.so | awk '{print $3}' | sort > r.txt
$ diff c.txt r.txt && echo IDENTICAL
IDENTICAL   (20 symbols on both sides)
```

- Missing from Rust: **0**
- Rust-only extras: **0**
- No stubs: `src/lib.rs` contains no `unimplemented!` / `todo!` /
  `panic!("not implemented")` (asserted by `phase_d_no_stubs`).
- Enforced continuously by `tests/phase_d_symbols.rs::phase_d_symbol_parity`,
  which re-runs `nm` on both objects and fails on any non-empty diff.
