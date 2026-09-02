# ERRORS.md — Error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep of every error-ish construct in the C source

```
$ grep -nE 'return|assert|RETURN_ERROR|NULL|errno|if *\(|-1|32767|32766|32768' c_src/src/lib.c
4:    if (sample >= 32766.5)
5:        return (int16_t)32767;
6:    if (sample <= -32767.5)
7:        return (int16_t)-32768;
10:    return s;
```

### Findings

* There is **no error-return macro** (`RETURN_ERROR`, `goto fail`, …), **no
  error enum**, **no `assert`**, **no `errno` use**, **no `NULL` check**, and
  **no `-1` / sentinel return** anywhere in the C source.
* `synth_pair` returns `void` — it has **no failure channel at all**.
* The only *explicit range checks* and *min/max constants* in the library are
  the two clipping branches of the `static` helper `mp3d_scale_pcm`. Those are
  the library's entire "rejection" surface: out-of-range samples are not
  reported, they are **saturated**.
* Everything else that an external caller can get wrong (null pointers,
  `nch` values that push `16 * nch` outside the caller's buffer, a `z` buffer
  shorter than `14 * 64 + 3` floats) is **unchecked undefined behaviour** in C.
  Those rows are still tested (out-of-process where the behaviour is a fault)
  so that the Rust translation is UB-for-UB and value-for-value identical.

## The table

One row per distinct rejection / range decision the C code actually makes.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[0]`) | accumulator `a >= 32766.5f` (explicit max range check, line 4) | `pcm[0] == 32767` (`INT16_MAX`), saturated — **no error reported** |
| E2 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[0]`) | accumulator `a <= -32767.5f` (explicit min range check, line 6) | `pcm[0] == -32768` (`INT16_MIN`), saturated — **no error reported** |
| E3 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[16*nch]`) | second accumulator `a >= 32766.5f` | `pcm[16*nch] == 32767` |
| E4 | `mp3d_scale_pcm` (via `synth_pair`, `pcm[16*nch]`) | second accumulator `a <= -32767.5f` | `pcm[16*nch] == -32768` |
| E5 | `mp3d_scale_pcm` | `a` exactly `== 32766.5f` (boundary is `>=`, inclusive: one step *past* the valid non-clipped range) | `32767` (clipped, **not** `32766`) |
| E6 | `mp3d_scale_pcm` | `a` exactly `== -32767.5f` (boundary is `<=`, inclusive) | `-32768` (clipped) |
| E7 | `mp3d_scale_pcm` | `a` = largest `f32` strictly below `32766.5` (`32766.498046875`) — one step *inside* the range | falls through to `(int16_t)(a + .5f)` = `32766`; `s >= 0` so no decrement → `32766` |
| E8 | `mp3d_scale_pcm` | `a` = smallest `f32` strictly above `-32767.5` (`-32767.498046875`) — one step inside the range | `(int16_t)(a + .5f)` = `-32766`; `s < 0` → `-32767` |
| E9 | `mp3d_scale_pcm` | `a` is negative and non-clipped, so the `s -= (s < 0)` branch fires (implicit conditional, line 9) | the cast truncates **toward zero** first, and only a *non-zero negative* `s` is decremented: `a=-1.5 -> -2`, `a=-2.0 -> -2`, `a=-2.5 -> -3`, `a=-100.25 -> -100`, `a=-32767.0 -> -32767` |
| E10 | `mp3d_scale_pcm` | `a` in `[-1.5, 0.0]` (i.e. `a + .5f` truncates to `0`) — `s == 0`, so `s < 0` is false | `0` (no decrement, despite the negative input; note this covers `a == -1.0`, which is **not** `-1`) |
| E11 | `mp3d_scale_pcm` | `a == NaN` (fails **both** range checks, since all NaN comparisons are false → reaches the `(int16_t)` cast, which is C UB) | GCC x86-64 emits `cvttss2si %xmm0,%eax` → `0x80000000`, narrowed to `int16_t` → **`0`**; then `s < 0` is false → `0` |
| E12 | `mp3d_scale_pcm` | `a == +INFINITY` | `+inf >= 32766.5` → `32767` |
| E13 | `mp3d_scale_pcm` | `a == -INFINITY` | `-inf <= -32767.5` → `-32768` |
| E14 | `synth_pair` | `pcm == NULL` (no null check exists) | UB: `SIGSEGV` (signal 11) on the `pcm[0]` store — **confirmed** by running the C `.so` in a forked child |
| E15 | `synth_pair` | `z == NULL` (no null check exists) | UB: `SIGSEGV` (signal 11) on the `z[14*64]` load — confirmed the same way |
| E16 | `synth_pair` | `nch == 0` — degenerate/"zero length": `pcm[16*0]` aliases nothing special but the two stores are 16 elements apart… actually `pcm[0]` and `pcm[0]`, so the **second store overwrites the first** | `pcm[0]` holds the *second* accumulator's value; the first is lost |
| E17 | `synth_pair` | `nch < 0` (e.g. `-1`) — negative index, `pcm[16 * -1]` writes **before** the pointer | UB unless the caller's buffer extends backwards; must use signed pointer arithmetic (`pcm - 16`), **not** `usize` wrap |
| E18 | `synth_pair` | `nch` huge (`INT_MAX`, `INT_MIN`) — `16 * nch` overflows `int` (signed overflow = UB in C) | GCC wraps two's-complement: `16 * INT_MAX == -16`, `16 * INT_MIN == 0`; the *`int`* product is then sign-extended for the pointer add |
| E19 | `synth_pair` | `z` buffer shorter than `14*64 + 1` floats (first block) / `2 + 14*64 + 1` floats (after `z += 2`) | UB: out-of-bounds read, no bounds check |
| E20 | (whole API) | there is **no out-of-range enum parameter** anywhere: `include/lib.h` declares no `enum`, and the only integer parameter is `int nch`, whose entire `int` range is accepted without validation | every `int` value is "valid" input; covered by E16–E18 |

## Checklist (Phase C)

| # | test | status |
|---|------|--------|
| E1 | `err_e1_e2_e3_e4_saturation` | [x] |
| E2 | `err_e1_e2_e3_e4_saturation` | [x] |
| E3 | `err_e1_e2_e3_e4_saturation` | [x] |
| E4 | `err_e1_e2_e3_e4_saturation` | [x] |
| E5 | `err_e5_e6_inclusive_clip_boundaries` | [x] |
| E6 | `err_e5_e6_inclusive_clip_boundaries` | [x] |
| E7 | `err_e7_e8_one_step_inside_range` | [x] |
| E8 | `err_e7_e8_one_step_inside_range` | [x] |
| E9 | `err_e9_e10_negative_decrement` | [x] |
| E10 | `err_e9_e10_negative_decrement` | [x] |
| E11 | `err_e11_nan` | [x] |
| E12 | `err_e12_e13_infinities` | [x] |
| E13 | `err_e12_e13_infinities` | [x] |
| E14 | `err_e14_e15_null_pointers` (out-of-process signal comparison) | [x] |
| E15 | `err_e14_e15_null_pointers` (out-of-process signal comparison) | [x] |
| E16 | `err_e16_nch_zero_second_store_wins` | [x] |
| E17 | `err_e17_negative_nch` | [x] |
| E18 | `err_e18_nch_int_overflow` | [x] |
| E19 | `err_e19_short_z_buffer` (exact-size buffer, ASAN-style tight allocation) | [x] |
| E20 | `err_e20_full_int_range_nch` | [x] |

## Divergences found by these tests, and the fixes applied to the Rust

Both were found by Phase C rows, not by any happy-path test.

### 1. `E18` — `16 * nch` was computed in 64-bit instead of wrapping in `int`

```rust
// before -- SIGSEGV for nch = INT_MAX: offsets by 34_359_738_352 elements
unsafe { *pcm.offset(16isize * nch as isize) = mp3d_scale_pcm(a) };

// after -- matches C, where `16 * nch` is an `int` product that wraps to -16
let offset = 16i32.wrapping_mul(nch) as isize;
unsafe { std::ptr::write(pcm.wrapping_offset(offset), mp3d_scale_pcm(a)) };
```

The C subscript `pcm[16 * nch]` multiplies two `int`s, so the product wraps at
32 bits before being sign-extended for the pointer arithmetic. Promoting `nch`
to `isize` first made the Rust store gigabytes away from the C's target for any
`|nch| >= 2^27`. `mutation_check.sh` re-introduces this exact bug (and a `usize`
variant) as a permanent regression guard.

### 2. `E14`/`E15` — debug builds aborted instead of segfaulting on a null argument

`*p = v` on a raw pointer carries a debug-only "null pointer dereference"
UB check, so the dev-profile `.so` died with `SIGABRT` (signal 6) where the C
died with `SIGSEGV` (signal 11). Switching the accesses to
`std::ptr::read`/`std::ptr::write` (which do not carry that check) makes both
profiles fault identically to C. Verified by forking a child per `.so`:

```
libharvest-work-*.so   null_pcm/null_z/null_both -> signal 11
debug   libsynth_pair_lib.so                     -> signal 11 (was 6)
release libsynth_pair_lib.so                     -> signal 11
```

## Non-rows: things deliberately NOT claimed as errors

* `>=` vs `>` at `32766.5` and `<=` vs `<` at `-32767.5` are **behaviourally
  equivalent**: for `a` in `[32766.5, 32767.5)` the fall-through path computes
  `(int16_t)(a + .5f) == 32767` anyway, and symmetrically at the bottom.
  `tests/exhaustive.rs` proves this over all 2^32 `f32` inputs. The Rust still
  mirrors the C's inclusive comparisons.
* There is no `errno`, no return code, and no output parameter, so "the same
  error code" degenerates to "the same saturated sentinel in the same slot",
  which is what every row above asserts.
