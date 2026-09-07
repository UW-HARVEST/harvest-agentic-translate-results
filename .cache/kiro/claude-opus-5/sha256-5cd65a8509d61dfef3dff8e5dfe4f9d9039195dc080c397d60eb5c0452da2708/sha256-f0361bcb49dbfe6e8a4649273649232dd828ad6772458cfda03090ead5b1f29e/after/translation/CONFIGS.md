# CONFIGS.md — Configuration / valid-input surface table

## Axes, derived from the branches the C actually takes

The library has **no** runtime options, modes, flags or `#ifdef`s
(`grep -c '#if\|switch' c_src/src/*.c c_src/include/*.h` → 0). Its entire
configuration surface is therefore the cross-product of *entry point* ×
*integer size parameter* × *input value shape* × *`threshold`*, restricted to
the combinations the code distinguishes:

**A. Entry points (both public dynamic symbols; the lower-level one is exercised
directly, not only through the `match` wrapper):**

* `A1` = `double spectral_contrast(float *a, float *b, int length)` — lowest-level
  public entry point; element type `float` (see `SYMBOLS.md`).
* `A2` = `int match(double *test, double *reference, int bins, double threshold)`
  — composed pipeline: `total` → gate → `preprocess`(`memcpy`+`smoothen`+
  `differentiate`+`smoothen`) ×2 → `spectral_contrast` on the *reinterpreted*
  `float` view → final compare.

**B. Size parameter, against the constants the code branches on
(`N_SMOOTH == 16`, `length - 1`, `length`):**

* `B0` `n = 0` — every loop degenerates.
* `B1` `n = 1` — `differentiate`'s loop is skipped entirely (`i < 0`).
* `B2` `n = 2` — smallest non-degenerate difference.
* `B3` `2 < n < 16` — `smoothen`'s inner loop is bounded by `i + j < length`
  for **every** `i`, so *all* outputs are attenuated.
* `B4` `n = 15` — last size where no window is full.
* `B5` `n = 16` — exactly one full window (`i = 0` only).
* `B6` `n = 17` — first size with both full and truncated windows.
* `B7` `n = 31 / 32 / 33` — around `2 * N_SMOOTH`.
* `B8` `n` large (`64 … 4096`) — majority of windows full.

**C. Value shape. For `A2` this is doubly significant: `spectral_contrast`
re-reads the buffer with a **4-byte stride**, so lane `2k` is the low half of
`double` `k` and lane `2k+1` is its high half. `bins` lanes therefore span only
the first `ceil(bins/2)` doubles, and both the mantissa tail *and* the
sign/exponent word of each `double` become `float` data.**

* `C1` uniform random positive doubles in `[0, 1)` — dense mantissas ⇒ pseudo-random
  `float` lanes.
* `C2` random doubles with sign (positive and negative mixed).
* `C3` exact small integers / powers of two ⇒ the low-half lanes are all
  `±0.0` while the high-half lanes carry the sign/exponent word, i.e. a strictly
  alternating zero / large-magnitude lane pattern that no random input produces.
* `C3'` the all-zero vector ⇒ *every* lane is `±0.0` ⇒ `magnitude == 0` ⇒
  division by zero inside `normalize` (the valid-input mirror of `ERRORS.md` #12).
* `C4` all elements equal (constant vector) ⇒ `differentiate` yields all zeros.
* `C5` `±0.0` mixture.
* `C6` denormal / tiny doubles (`1e-320`) and huge doubles (`1e300`).
* `C7` `±inf` present.
* `C8` NaN present, with **distinct non-canonical payloads** (this pins the
  `ADDSD`/`MULSS` destination-operand NaN-propagation order).
* `C9` monotone ramp (realistic spectrum) — the intended use.
* `C10` single spike / impulse.
* `C11` `test` aliases `reference` (same pointer).
* `C12` (`A1` only) bit-pattern-random `float` lanes, including the full
  exponent range.
* `C13` (`A1` only) `a` aliases `b`.

**D. `threshold` (`A2` only) — decides both the gate and the final compare:**

* `D1` `threshold < 0`
* `D2` `threshold == 0.0`, `threshold == -0.0`
* `D3` `0 < threshold < 1` (gate normally passes)
* `D4` `threshold == 1.0`
* `D5` `threshold > 1` (gate normally rejects)
* `D6` `threshold = ±inf`
* `D7` `threshold = NaN`

## Rows (one per combination the C distinguishes)

Each row is checked off only after **both** `.so`s agree bit-for-bit on the
returned value *and* on every byte of the caller-visible output buffers, over
many seeded-random inputs (100–2000 draws per row, seed fixed in the test).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `A1` | `B0` `length = 0`, plus `length = -1` and `INT_MIN`; buffers non-`NULL` and `NULL` | [x] |
| 2 | `A1` | `B1` `length = 1`, `C1` random finite floats | [x] |
| 3 | `A1` | `B2` `length = 2`, `C1` | [x] |
| 4 | `A1` | `B3` `length ∈ 3..15`, `C1` | [x] |
| 5 | `A1` | `B5`/`B6` `length ∈ {15,16,17}`, `C1` | [x] |
| 6 | `A1` | `B7`/`B8` `length ∈ {31,32,33,64,257,1024}`, `C1` | [x] |
| 7 | `A1` | any `B`, `C12` fully bit-random `f32` lanes (all exponents, denormals, NaNs, infinities) | [x] |
| 8 | `A1` | any `B`, `C3`/`C5` all-zero and `±0.0` lanes ⇒ `magnitude == 0` ⇒ `0/0` and `x/0` | [x] |
| 9 | `A1` | any `B`, `C6` denormal-only lanes (magnitude underflows) and huge lanes (`dot_product` overflows to `+inf`) | [x] |
| 10 | `A1` | any `B`, `C7` `±inf` lanes ⇒ `inf*inf`, `inf*0` | [x] |
| 11 | `A1` | any `B`, `C8` NaN lanes with distinct payloads in `a` **and** `b` (pins `MULSS`/`ADDSD` operand roles) | [x] |
| 12 | `A1` | any `B`, `C13` `a == b` aliased | [x] |
| 13 | `A1` | `B8`, `C2` mixed-sign lanes ⇒ negative `dot_product`, unit vectors with negative components | [x] |
| 14 | `A2` | `B0` `bins = 0` × all of `D1…D7` — **C undefined behaviour, verified out-of-process** (see note) | [x] |
| 15 | `A2` | `B1` `bins = 1`, `C1` × `D1…D7` (`differentiate` loop skipped) | [x] |
| 16 | `A2` | `B2` `bins = 2`, `C1` × `D2`,`D3`,`D4`,`D5` | [x] |
| 17 | `A2` | `B3`/`B4` `bins ∈ 3..15`, `C1` × `D3` (all `smoothen` windows truncated) | [x] |
| 18 | `A2` | `B5` `bins = 16`, `C1` × `D3` (exactly one full window) | [x] |
| 19 | `A2` | `B6` `bins = 17`, `C1` × `D3` | [x] |
| 20 | `A2` | `B7` `bins ∈ {31,32,33}`, `C1` × `D3` | [x] |
| 21 | `A2` | `B8` `bins ∈ {64,128,1000,4096}`, `C1` × `D3` | [x] |
| 22 | `A2` | `B8`, `C2` mixed-sign doubles × `D1`,`D2`,`D3`,`D5` (gate can reject on negative totals) | [x] |
| 23 | `A2` | any `B`, `C3` exact integers / powers of two ⇒ alternating `±0` / exponent-word `float` lanes; and `C3'` the all-zero vector ⇒ `magnitude == 0` | [x] |
| 24 | `A2` | any `B`, `C4` constant vectors ⇒ `differentiate` yields a constant, and the trailing truncated `smoothen` windows a ramp of exact binary fractions | [x] |
| 25 | `A2` | any `B`, `C5` `±0.0` mixtures × `D2` (gate `0 < 0` boundary) | [x] |
| 26 | `A2` | any `B`, `C6` denormals and `1e±300` magnitudes (`total` overflow, `float`-lane denormals) | [x] |
| 27 | `A2` | any `B`, `C7` `±inf` present ⇒ `inf - inf` in `differentiate`, `inf` in `total` ⇒ `inf*0` gate | [x] |
| 28 | `A2` | any `B`, `C8` NaN payloads in `test` and/or `reference` | [x] |
| 29 | `A2` | any `B`, `C9` monotone ramp (intended spectral use), `D3`/`D4` | [x] |
| 30 | `A2` | any `B`, `C10` impulse (single non-zero bin), `D3` | [x] |
| 31 | `A2` | any `B`, `C11` `test == reference` (aliased), all `D` | [x] |
| 32 | `A2` | `D6` `threshold = ±inf` with random finite data (gate `total < ±inf * total`) | [x] |
| 33 | `A2` | `D7` `threshold = NaN` with random finite data (both compares unordered) | [x] |
| 34 | `A2` | fully bit-random `f64` payloads (every axis simultaneously; 3000 draws over `bins ∈ 1..40`, plus 3000 for `A1` over `length ∈ 0..40`) — the un-pruned fuzz row | [x] |
| 35 | `A1` + `A2` | full pipeline consistency: `A2` invoked on data whose exact interleaved low/high `float` lane sequence is separately fed to `A1`, confirming the composed reinterpretation path and the standalone path agree over identical bytes | [x] |
| 36 | `A1` + `A2` | env-gated deep fuzz: every axis of both entry points randomised together, `FUZZ_ITERS` draws from `FUZZ_SEED`. Executed for 3 × 200 000 iterations (seeds `0x1`, `0x3ADE68B1`, `0x2A`) with no divergence | [x] |

## Row → test mapping

Every row `N` is implemented by `tests/differential.rs::rowNN_*`; run
`cargo test --test differential` to execute all of them. Row counts and the seed
per row are fixed in the source, so every run is reproducible.

## Note on `bins <= 0` (row 14)

`bins == 0` and `bins < 0` are **undefined behaviour in the C and reproducibly
kill the C `.so`**, so they carry no value to be byte-identical to:

* `bins == 0` — GCC allocates a zero-length VLA, leaving `rsp` unchanged, so `t`
  aliases the stack top. `differentiate` then executes `v[length - 1] = 0`, i.e.
  `v[-1] = 0`, which lands on `preprocess`'s saved return address and makes the
  library `ret` to address 0 ⇒ SIGSEGV.
* `bins < 0` — the VLA byte count `(bins*8 + 15) / 16 * 16` is computed with an
  *unsigned* `div`, so `rsp` is decremented by ~2^64 ⇒ SIGSEGV.

Both are therefore driven in a `fork()`ed child, where the compared observable is
the termination outcome; see `ERRORS.md` rows 4/5 and
`tests/error_paths.rs::err04_bins_zero` / `err05_bins_negative_out_of_process`.
The smallest in-process size for `match` is `bins == 1`. The same applies to
`bins` large enough to overflow the stack (`ERRORS.md` row 19).

## Binary executable

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED ...)` and no
`add_executable`, so the project builds **no** driver binary; there is no stdout
to compare. `translation/Cargo.toml` likewise declares only `[lib]` with
`crate-type = ["cdylib"]` and has no `src/main.rs` / `[[bin]]`.

## Feature combinations

No `[features]` table exists, so `{default}` == `{--no-default-features}` is the
only combination. `run_all.sh` derives the list from `Cargo.toml` rather than
hard-coding it, and runs the full suite for the cross-product of

* feature combination: `{default}`, `{--no-default-features}`
* Rust build profile of the loaded `.so`: `release`, `debug`

i.e. four configurations, each running all 36 Phase-B rows and all 20 Phase-C
rows against the same C `.so`.

`[profile.dev]` sets `debug-assertions = false` / `overflow-checks = false`. This
crate deliberately mirrors the C's raw-pointer semantics, and Rust's debug-only
UB checks would otherwise turn the `NULL`-with-positive-length segfault
(`ERRORS.md` row 18) into a `SIGABRT`, making the dev-profile `.so` diverge from
both the release `.so` and the C. With the checks off, all four configurations
are trap-identical.
