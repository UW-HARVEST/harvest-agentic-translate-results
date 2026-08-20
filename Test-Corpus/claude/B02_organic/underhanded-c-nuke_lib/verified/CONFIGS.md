# CONFIGS.md — Phase A configuration-surface table

## Axes, derived from the branches the C actually takes

There are **no** compile-time options (`c_src` has zero `#ifdef`s; the only
`#define` is `N_SMOOTH 16`) and **no** runtime option/mode/flag structs — the API
is two free functions. So the configuration surface is entirely
*entry point × size/shape of the input × value of `threshold`*.

### A1. Public entry points (from `c_src/include/match.h`)

| entry point | signature | level |
|---|---|---|
| `spectral_contrast` | `double spectral_contrast(float_t *a, float_t *b, int length)` | **lowest level** — normalizes both buffers in place, returns their dot product. Callers see `float_t = double` via `match.h`; the definition sees `float_t = float` (see SYMBOLS.md). Tested directly, not only through `match`. |
| `match` | `int match(float_t *test, float_t *reference, int bins, double threshold)` | composed pipeline: energy gate → `preprocess`×2 (`memcpy`+`smoothen`+`differentiate`+`smoothen`) → `spectral_contrast` → threshold verdict |

### A2. Size branches the code distinguishes

* `smoothen`: `for(j=0; j<N_SMOOTH && i+j<length; j++)` — the `i+j<length` clamp
  fires for **every** `i` when `length <= 16`, and only for the last 15 `i`s when
  `length > 16`. ⇒ `length` **< 16**, **== 16**, **== 17**, **>> 16** are
  distinct shapes.
* `differentiate`: `for(i=0;i<length-1;i++)` then `v[length-1]=0`. For
  `length == 1` the loop never runs and the single element is forced to `0`,
  so `preprocess` always produces an all-zero vector ⇒ zero magnitude ⇒ NaN
  contrast. `length == 1` is its own shape.
* `match` → `spectral_contrast` reads `bins` **floats** over `bins` **doubles**,
  i.e. only the first `ceil(bins/2)` doubles are touched, and each double
  contributes its low half and/or high half. ⇒ **parity of `bins`** is a real
  branch (odd `bins` reads a trailing low-half only).
* `dot_product`/`normalize`: plain `i<length` loops ⇒ `length` `<=0`, `1`, `many`.

### A3. Value branches the code distinguishes

* `match.c:37` energy gate: taken / not taken / NaN-unordered.
* `normalize`: `magnitude` is `0` / normal / `+inf` / NaN ⇒ quotient is NaN /
  finite / `±0` / NaN.
* `dot_product` accumulates `double` but rounds each product to `float` ⇒
  `float` overflow (`>3.4e38`), `float` underflow to denormal/zero, and
  denormal inputs are all distinct value regimes.
* `threshold` is compared with `<` and `>=` and multiplied ⇒ sign, zero,
  infinity and NaN are distinct.
* aliasing: `a == b` in `spectral_contrast` (normalize runs twice on the same
  buffer) and `test == reference` in `match` are unguarded ⇒ distinct.

### A4. Feature combinations

`Cargo.toml` has **no `[features]` section**, and `CMakeLists.txt` has no
options, so there is exactly **one** configuration:
`cargo …  --no-default-features` (≡ default, ≡ all-features). Both the `dev` and
`release` cargo profiles are exercised, because `[profile.release]` overrides
`overflow-checks`/`debug-assertions`.

## Table

`R` = randomized over many seeds (fixed root seed `0x5EED_1234_ABCD_EF01`),
`N` = number of random draws per row. Every row asserts the return value
bit-for-bit (`f64::to_bits` / `i32`) **and** the full post-call byte image of
both buffers (`spectral_contrast` mutates them in place), **and** — for `match` —
that the input buffers were left untouched.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `spectral_contrast` | `length=1`; `a`,`b` = R normal `f32` (±1e±6 range), N=512 | [x] |
| 2 | `spectral_contrast` | `length=2`; R normal `f32`, N=512 | [x] |
| 3 | `spectral_contrast` | `length ∈ 3..=8`; R normal `f32`, N=256 per length | [x] |
| 4 | `spectral_contrast` | `length ∈ {15,16,17}` (N_SMOOTH neighbourhood); R normal `f32`, N=256 | [x] |
| 5 | `spectral_contrast` | `length ∈ {31,32,33,64}`; R normal `f32`, N=128 | [x] |
| 6 | `spectral_contrast` | `length ∈ {255,256,1024}` (long vectors, accumulation order); R normal, N=32 | [x] |
| 7 | `spectral_contrast` | `length ∈ {1,2,3,7,16,17,64}`; **fully random raw 32-bit patterns** (NaN, ±inf, denormals, ±0 all occur naturally), N=512 | [x] |
| 8 | `spectral_contrast` | `a` = all `+0.0` (zero magnitude ⇒ NaN), `b` = R normal; both orders; N=64 | [x] |
| 9 | `spectral_contrast` | `a`,`b` both all `+0.0`; and both all `-0.0`; `length ∈ {1,2,16,17}` | [x] |
| 10 | `spectral_contrast` | denormal-only inputs (`f32` bits `1..=0x7FFFFF`) ⇒ magnitude underflows; N=256 | [x] |
| 11 | `spectral_contrast` | huge inputs `≈3.0e38` ⇒ `float` products overflow to `+inf` ⇒ `sqrt(inf)`; N=256 | [x] |
| 12 | `spectral_contrast` | tiny inputs `≈1e-38` ⇒ products flush to denormal/zero; N=256 | [x] |
| 13 | `spectral_contrast` | random ±0.0 mixture (zero-sign propagation through `divsd`/`cvtsd2ss`); N=256 | [x] |
| 14 | `spectral_contrast` | **aliased**: `a == b` (same pointer), `length ∈ {1,2,3,16,17,64}`, R data, N=128 | [x] |
| 15 | `spectral_contrast` | distinct buffers with bit-identical content (contrast ≈ 1.0); N=128 | [x] |
| 16 | `spectral_contrast` | `b = -a` (anti-parallel, contrast ≈ -1.0); N=128 | [x] |
| 17 | `spectral_contrast` | `±inf` elements mixed with finite ⇒ `inf-inf` NaN inside the sum; N=256 | [x] |
| 18 | `spectral_contrast` | explicit quiet **and signaling** NaN bit patterns mixed with finite; N=256 | [x] |
| 19 | `match` | `bins=1` (degenerate `differentiate` ⇒ all-zero vector ⇒ NaN contrast) × full `threshold` sweep, R data, N=256 | [x] |
| 20 | `match` | `bins=2`; R positive doubles; `threshold=0.5`; N=512 | [x] |
| 21 | `match` | `bins ∈ 3..=8`; R positive doubles; `threshold=0.5`; N=256 | [x] |
| 22 | `match` | `bins ∈ {15,16,17}` — the `smoothen` full-tap / clamped-tap boundary; R data; N=256 | [x] |
| 23 | `match` | `bins ∈ {31,32,33}`; R data; N=128 | [x] |
| 24 | `match` | `bins ∈ {63,64,128,257,1024}` (long, plus odd `257`); R data; N=32; plus `bins ∈ {4096,16384}` (256 KiB of VLA) with boundary recovery | [x] |
| 25 | `match` | **odd vs even `bins`** (`5` vs `6`, `17` vs `18`) with identical prefix data — exercises the half-double `float` reinterpretation tail; N=256 | [x] |
| 26 | `match` | `threshold ∈ {-inf,-1e308,-1.0,-0.0,+0.0,5e-324,1e-300,0.5,1.0,2.0,1e308,+inf,NaN}` × `bins ∈ {1,2,16,17}` × R data | [x] |
| 27 | `match` | **aliased**: `test == reference` (same pointer), `bins ∈ {1,2,3,16,17}`, R data, N=128 | [x] |
| 28 | `match` | distinct buffers, bit-identical content; `bins ∈ {2,3,16,17}`; N=128 | [x] |
| 29 | `match` | `test` all zeros, `reference` R positive ⇒ energy gate rejects (also ERRORS row 1); N=64 | [x] |
| 30 | `match` | `reference` all zeros, `test` R positive ⇒ gate passes (`x < thr*0`) ⇒ full pipeline; N=64 | [x] |
| 31 | `match` | ramp `v[i]=i+1` and reversed ramp — monotone data, exact `smoothen` sums; `bins ∈ {1,2,3,16,17,33,64}` | [x] |
| 32 | `match` | realistic peaked "spectrum" shape (narrow Gaussian peaks on a floor); `bins ∈ {16,17,64,257}`; N=64 | [x] |
| 33 | `match` | denormal doubles (`f64` bits `1..=0xFFFF`); `bins ∈ {2,3,16,17}`; N=256 | [x] |
| 34 | `match` | huge doubles `≈1e308` ⇒ `total` overflows to `+inf`, `smoothen` sums overflow; `bins ∈ {2,16,17}`; N=256 | [x] |
| 35 | `match` | mixed `±inf`/NaN inside the data ⇒ NaN gate product (also ERRORS row 7); `bins ∈ {2,3,16,17}`; N=256 | [x] |
| 36 | `match` | **fully random raw 64-bit patterns** for both buffers; `bins ∈ {1,2,3,16,17,33}`; N=512 | [x] |
| 37 | `match` | all-negative data (negative totals flip the gate's sense); `bins ∈ {2,3,16,17}`; N=256 | [x] |
| 38 | `match` | `test = scale * reference` for R `scale ∈ {0.25,0.5,1,2,4}` (the realistic "same spectrum, different gain" case) × `threshold ∈ {0.25,0.5,0.9,1.0}`; N=128 | [x] |
| 39 | `match` | input buffers must be **unmodified** after the call (C only reads `test`/`reference`); checked on every row above via a byte-image comparison | [x] |
| 40 | both | `length`/`bins` boundary values `0`, `-1`, `i32::MIN`, and `spectral_contrast` with null pointers — see ERRORS.md rows 11–13, 16–17 | [x] |
| 41 | `spectral_contrast` | **NaN payload order**: `a` and `b` both all-NaN with a *distinct payload per element*, plus one-side-finite; `length ∈ {1,2,3,4,8,16,17,33}`, N=256. Identifies which operand of `mulss`/`addsd` survives. | [x] |
| 42 | `match` | all-NaN data with distinct payloads (payload survives `smoothen`'s 16-tap `addsd` chain and the `sum/16` store); `bins ∈ {1,2,3,4,8,15,16,17,18,33,64}` | [x] |
| 43 | `match` | mostly-finite data with distinct-payload NaNs sprinkled in; `bins ∈ {1,2,3,8,15,16,17,18,33}`, N=256 | [x] |
| 44 | `match` | a single NaN (then a single `±inf`) walked through **every** index, for each `bins ∈ {1,2,3,16,17,18,33}`, on one side and on both | [x] |
| 45 | `match` | **decision-boundary recovery**: bisect on `threshold` to recover the full 53-bit `min(total(test)/total(reference), contrast)` instead of comparing the 1-bit verdict; all 10 data shapes × `bins ∈ {1,2,3,5,8,15,16,17,18,33,64}` | [x] |
| 45b | `match` | boundary recovery on scaled copies (`test = scale*reference`), where the boundary *is* the internal contrast; `bins ∈ {2,3,16,17,33,64,257}` × `scale ∈ {1,0.5,2,8,1e6}` | [x] |

## Observability limits (why rows 45/45b exist)

`match` returns a single bit, so a wrong internal `spectral_contrast` value only
changes the answer when `threshold` happens to fall between the C's value and
the Rust's. Injecting a deliberate 1-ulp error into `spectral_contrast` was
caught by rows 26/27/36/45/45b but **missed** by rows 20–25, 29–35 and 37–39.
Row 45 removes that blind spot by bisecting `threshold` to recover the boundary
exactly (both the gate ratio and the contrast are monotone in `threshold`, so the
verdict is a single step whose location is `min(gate ratio, contrast)`).

Conversely, NaN *payload* differences are provably **not** observable through
`match`: once any NaN enters, the reinterpreted `float` view always contains a
NaN (a `f64` NaN's high half always has an all-ones `float` exponent), so the
contrast is NaN and the verdict is `0` regardless of payload. Rows 42–44 are
therefore coverage rather than discrimination; the payload order is pinned by
row 41 through `spectral_contrast`'s `double` return value.

## Harness-integrity guards (`tests/phase_d.rs`)

| # | guard | why |
|---|-------|-----|
| D1 | `symbol_parity_c_exports_are_all_present_in_rust` | re-derives `nm -D` so SYMBOLS.md cannot drift |
| D2 | `symbol_parity_is_exact_in_both_directions` | also asserts neither `.so` leaks the six C `static` helpers |
| D3 | `no_symbol_interposition_between_loaded_libraries` | runs a fixed call batch with both `.so`s resident and again with only one resident (fresh process) and compares digests. Without this, `RTLD_LOCAL` could silently be violated and the C `match` would call the *Rust* `spectral_contrast`, making every `match` row vacuous. |
| D4 | `spectral_contrast_reinterprets_double_buffers_as_float` | pins the underhanded ABI: a `match.h`-following caller passes `double*`, exactly `length*4` bytes are rewritten, and everything at/after byte `length*4` is untouched |
| D5 | `assert_fresh` (in `tests/common/mod.rs`) | `cargo test --test X` does **not** rebuild a `cdylib`-only lib target, so a stale `.so` could pass vacuously. The harness refuses to run against a `.so` older than the newest source file. |
