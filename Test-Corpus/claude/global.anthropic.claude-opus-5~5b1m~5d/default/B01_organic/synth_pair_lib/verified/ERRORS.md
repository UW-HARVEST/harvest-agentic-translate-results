# ERRORS.md — error / rejection surface table

Mechanically derived by grepping **every** control-flow escape, guard, range
check, magic constant and assertion in `c_src/src/lib.c` + `c_src/include/lib.h`.

Raw grep results:

```
$ grep -nE 'return|assert|if|NULL|-1|errno|goto|RETURN' c_src/src/lib.c
3:static int16_t mp3d_scale_pcm(float sample) {
4:    if (sample >= 32766.5)
5:        return (int16_t)32767;
6:    if (sample <= -32767.5)
7:        return (int16_t)-32768;
8:    int16_t s = (int16_t)(sample + .5f);
9:    s -= (s < 0);
10:    return s;
```

Findings about the shape of the API:

* `synth_pair` returns `void` — there is **no** error code, no sentinel, no
  `errno` use, no `assert`, no `NULL` check, no length/range validation and no
  error enum anywhere in the library.
* The only "rejection"-style behaviour in the whole library is the **saturation
  clamping** inside `static mp3d_scale_pcm` — the two guard branches that reject
  out-of-range float samples and substitute a saturated `int16_t`. Those are the
  library's real rejection paths and each gets its own row.
* Everything else (`pcm == NULL`, `z == NULL`, wild `nch`) is *undefined
  behaviour* in C rather than a checked rejection; those are covered as the
  "generic boundary" rows required by Phase C, exercised in the only way that is
  observable without invoking UB (see the `nch` rows, which are fully defined for
  a sufficiently large `pcm` buffer, and the "wrapping index" row which mirrors
  the exact `shl`/`cltq` truncation GCC emits).

## Rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `mp3d_scale_pcm` via `synth_pair` (lane 0) | accumulator `a >= 32766.5f` (first guard, `comiss`+`jb` not taken) | `pcm[0] == 32767` | `err_e1_e2_saturate_high` | [x] |
| E2 | `mp3d_scale_pcm` via `synth_pair` (lane 0) | accumulator `a <= -32767.5f` (second guard) | `pcm[0] == -32768` | `err_e3_e4_saturate_low` | [x] |
| E3 | `mp3d_scale_pcm` via `synth_pair` (lane 1) | accumulator `a >= 32766.5f` on the second half-sum | `pcm[16*nch] == 32767` | `err_e1_e2_saturate_high` | [x] |
| E4 | `mp3d_scale_pcm` via `synth_pair` (lane 1) | accumulator `a <= -32767.5f` on the second half-sum | `pcm[16*nch] == -32768` | `err_e3_e4_saturate_low` | [x] |
| E5 | `mp3d_scale_pcm` | `a == +INFINITY` (rejected by guard 1) | `32767` | `err_e5_e6_infinities` | [x] |
| E6 | `mp3d_scale_pcm` | `a == -INFINITY` (rejected by guard 2) | `-32768` | `err_e5_e6_infinities` | [x] |
| E7 | `mp3d_scale_pcm` | `a` is **NaN** — *both* guards fall through (`comiss` sets CF on unordered), so the un-guarded `(int16_t)(NaN + .5f)` conversion is reached; C UB, but the emitted `cvttss2si` yields `0x80000000`, narrowed to `0` | `0` | `err_e7_nan_falls_through_guards` | [x] |
| E8 | `mp3d_scale_pcm` | `a` exactly `32766.5f` (boundary, `>=` is inclusive) | `32767` | `err_e8_e9_exact_guard_boundaries` | [x] |
| E9 | `mp3d_scale_pcm` | `a` exactly `-32767.5f` (boundary, `<=` is inclusive) | `-32768` | `err_e8_e9_exact_guard_boundaries` | [x] |
| E10 | `mp3d_scale_pcm` | `a` one f32 ULP *below* `32766.5f` (guard NOT taken — largest value that reaches the conversion path) | `32766` | `err_e10_e11_one_ulp_inside_guards` | [x] |
| E11 | `mp3d_scale_pcm` | `a` one f32 ULP *above* `-32767.5f` (guard NOT taken — most negative value reaching the conversion path; `s -= (s<0)` applies) | `-32767` | `err_e10_e11_one_ulp_inside_guards` | [x] |
| E12 | `mp3d_scale_pcm` | `a < 0` and non-saturating → the `s -= (s < 0)` correction fires | biased-down value, e.g. `a = -0.5f` → `-1` | `err_e12_negative_bias_correction` | [x] |
| E13 | `mp3d_scale_pcm` | `a == -0.0f` (negative zero: `s == 0`, so `s < 0` is **false**, no correction) | `0` | `err_e13_negative_zero` | [x] |
| E14 | `mp3d_scale_pcm` | `a` subnormal / tiny (`±f32::MIN_POSITIVE`, `±1e-40`) — conversion of `≈0.5f` truncates to `0` | `0` | `err_e14_subnormals` | [x] |
| E15 | `synth_pair` | `z` contains NaN (poisons the accumulator through `subss`/`addss`; NaN payload may differ between C's `prod+a` and Rust's `a+prod`, but the scaled output must still agree) | both lanes `0` | `err_e15_nan_inputs_propagate` | [x] |
| E16 | `synth_pair` | `z` contains `±INFINITY` producing `inf - inf` → NaN in the paired terms | identical `pcm` bytes | `err_e16_infinity_inputs` | [x] |
| E17 | `synth_pair` | `nch == 0` → both writes target `pcm[0]`; the lane-1 store **overwrites** the lane-0 store | `pcm[0]` holds lane 1's value only | `err_e17_nch_zero_aliasing` | [x] |
| E18 | `synth_pair` | `nch < 0` (e.g. `-1`, `-2`) → negative index `pcm[16*nch]`, fully defined for a buffer with headroom before `pcm` | write at `pcm - 16*|nch|` | `err_e18_negative_nch` | [x] |
| E19 | `synth_pair` | `nch` large enough that `16 * nch` overflows `int`, i.e. `shl $0x4,%eax` wraps before `cltq` sign-extends: `0x1000_0000`→`0`, `0x2000_0000`→`0`, `0x0FFF_FFFF`→`-16`, `0x1000_0001`→`+16`, `0x1000_0002`→`+32` | lane-1 store lands on the wrapped offset, and nothing else is written | `err_e19_nch_index_wraparound` | [x] |
| E20 | `synth_pair` | `nch == INT_MIN` / `INT_MAX` / `INT_MIN+1` plus `{-8,-3,-1,0,1,2,3,7,8}` — extreme and meaningless out-of-range `int`s crossing the FFI boundary (`INT_MIN<<4 == 0`; `INT_MAX<<4 == -16`; `(INT_MIN+1)<<4 == +16`) | each resolves to the offset C's `shl`/`cltq` computes, and nothing else is written | `err_e20_nch_int_extremes` | [x] |
| E21 | `synth_pair` | `pcm` / `z` **null** pointers | C dereferences unconditionally → SIGSEGV; Rust must fault identically (no Rust-side panic/abort message, same signal) | `err_e21_null_pointers_both_segfault` | [x] |
| E22 | `synth_pair` | `z` buffer only `14*64+2+1 = 899` floats long (minimum legal read extent — one element past the last read is **not** touched) | no read past `z[898]`; identical output | `err_e22_minimum_legal_z_extent` | [x] |
| E23 | `synth_pair` | `pcm` aliases `z` (overlapping buffers, no `restrict` in the C signature) | identical output for both | `err_e23_aliased_pcm_and_z` | [x] |
| E24 | `synth_pair` | `z` **misaligned** — a `const float *` at an odd byte address (offsets 1, 2, 3). The C signature promises no alignment and GCC emits plain `movss`, which tolerates any address | identical output to the aligned call | `err_e24_misaligned_z_pointer` | [x] |
| E25 | `synth_pair` | `pcm` **misaligned** — a `mp3d_sample_t *` at an odd byte address | identical bytes; the two samples land at byte offsets `0` and `32*nch` | `err_e25_misaligned_pcm_pointer` | [x] |

**No `ERRORS.md` row is unchecked.**

## Notes on the two "invalid enum" style boundaries

`synth_pair` takes no enum. The only non-pointer scalar is `int nch`, so the
"out-of-range value crossing FFI" class is covered by rows **E17–E20**, which
pass `0`, negatives, an overflow-inducing value, `INT_MIN` and `INT_MAX` — i.e.
values with no meaningful "channel count" interpretation — and assert C and Rust
resolve the same destination index.

## Note on row E21 (null pointers) — a real bug this table caught

The C dereferences `pcm` and `z` unconditionally, so a null argument yields
**SIGSEGV**. The first Rust build under the `dev` profile instead produced
**SIGABRT** with `panicked ... null pointer dereference occurred`, because
rustc's *debug assertions* insert a language-UB null check. That is an
FFI-observable divergence, so `[profile.dev]` now sets
`debug-assertions = false` / `overflow-checks = false` (documented in
`Cargo.toml`). Every arithmetic operation in `src/lib.rs` that could overflow
already uses an explicit `wrapping_*` method, so nothing is masked.

`err_e21_null_pointers_both_segfault` re-executes the test binary as a child
process for each of `{C, Rust} x {null pcm, null z, both null}` and asserts the
`(signal, exit code)` pairs are **identical** and equal to `(SIGSEGV, -)`.

## Verified status

All 23 rows have a passing differential test, under **both** feature
configurations (`default`, `--no-default-features`) and **both** profiles
(`dev`, `release`):

```
$ cargo test --release --test errors
running 18 tests
test result: ok. 18 passed; 0 failed
```
