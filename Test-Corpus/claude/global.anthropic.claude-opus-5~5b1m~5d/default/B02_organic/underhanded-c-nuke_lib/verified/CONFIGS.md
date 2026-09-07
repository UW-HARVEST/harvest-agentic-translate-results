# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## How the axes were derived (mechanically)

Public header `c_src/include/match.h` is the entire API:

```c
#define N_SMOOTH 16
typedef double float_t;
int    match(float_t *test, float_t *reference, int bins, double threshold);
double spectral_contrast(float_t *a, float_t *b, int length);
```

* **Runtime options / modes / flags:** there are **none**. No global state, no
  init function, no setter, no context struct, no `#ifdef`
  (`grep -c '#if' c_src -r` -> 0), no environment variable. The only tunable is
  the compile-time `#define N_SMOOTH 16`, which is fixed in the built `.so`.
  So the "options" axis collapses; the whole configuration space is
  **entry point × input shape × value class**.
* **Entry points (both, including the low-level one):**
  * `spectral_contrast` — the *low-level* entry point. It is exported and is
    what `match` itself calls, and it mutates both of its arguments in place,
    so it must be driven directly, not only through `match`.
  * `match` — the composed pipeline
    (`total` → gate → `preprocess`×2 → `spectral_contrast`).
* **Branch points the C actually distinguishes** (`grep` of every `if`/`for`
  guard, see ERRORS.md):
  1. `i < length` in `total`, `dot_product`, `normalize`, `smoothen` outer —
     ⇒ `length` `<= 0` vs `> 0`.
  2. `j < N_SMOOTH && i + j < length` in `smoothen` — ⇒ `length < 16`,
     `== 16`, `> 16` behave differently (window clamping, attenuated tail).
  3. `i < length - 1` in `differentiate` — ⇒ `length` `0`, `1`, `>= 2`.
  4. `if(total < threshold * total)` in `match` — ⇒ gate taken /
     not taken / unordered, driven by the `threshold` **value class**.
  5. `spectral_contrast(...) >= threshold` — ⇒ `threshold` value class again.
  6. `magnitude` zero vs non-zero vs NaN in `normalize` (unguarded divide).
  7. The `float_t` split: `spectral_contrast` indexes as **f32**, `match`'s
     buffers are **f64**. ⇒ `bins` parity (odd/even) changes which half of
     which `double` is the last f32 read; and the f32 view of arbitrary f64
     bytes routinely decodes to NaN/inf/denormal, so the **bit pattern** of the
     input, not just its magnitude, is a real axis.
* **Input value classes that the code shape makes distinct:** all-zero
  (`±0.0`), constant (which -- MEASURED -- is NOT all-zero after preprocessing, because `smoothen`'s clamped tail breaks the constancy; see ERRORS.md E18), small positive
  "realistic spectrum", mixed sign, denormal, huge (f32 view overflows),
  `±inf`, quiet NaN, signalling NaN, adversarial raw bit patterns whose f32
  halves are NaN.
* **Aliasing:** no `restrict` anywhere, so `a == b`, and partial overlap, are
  valid configurations of `spectral_contrast`.
* **Byte order / element type:** single fixed ABI (x86-64 little-endian,
  IEEE-754 binary64/binary32). Not an axis, but it is *why* the f32/f64 split
  is observable, so it is pinned by comparing raw bit patterns rather than
  float values in every row.

No binary/driver target exists in `CMakeLists.txt` (library only), so there is
no stdout comparison row.

## The table

Every row is driven with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, SplitMix64) unless the row is inherently a single point.
"compare" always means: the returned value's **raw IEEE-754 bits** (or the
`int`), **plus** the raw bits of every element of every buffer after the call,
byte-for-byte between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `spectral_contrast` | `length = 0`; buffers non-empty but untouched; also `length = 0` with `NULL` pointers | [x] |
| C2 | `spectral_contrast` | `length = 1`; random f64 bit patterns behind the pointer (1 f32 read out of 2 slots) | [x] |
| C3 | `spectral_contrast` | `length = 2 .. 8` exhaustive lengths, random realistic f32-valued data written through an f32 view | [x] |
| C4 | `spectral_contrast` | `length = 15, 16, 17` (N_SMOOTH boundary is irrelevant here, but these are the shared boundary lengths), random data | [x] |
| C5 | `spectral_contrast` | `length` = 31, 32, 33, 64, 127, 128, 1024, 4096; random data | [x] |
| C6 | `spectral_contrast` | all elements `+0.0` ⇒ `magnitude == 0` ⇒ unguarded `0.0/0.0` (E9); and all `-0.0`; and mixed `±0.0` | [x] |
| C7 | `spectral_contrast` | data containing `+inf` / `-inf` ⇒ `dot_product` `+inf`, `sqrt(+inf) = +inf`, `x/inf` | [x] |
| C8 | `spectral_contrast` | data containing quiet NaN with **assorted payloads** (payload-propagation order through `mulss`→`cvtss2sd`→`addsd`) | [x] |
| C9 | `spectral_contrast` | data containing **signalling** NaN (`0x7FBFFFFF`, `0xFFA00001`) ⇒ quieting via `mulss`/`cvtss2sd` | [x] |
| C10 | `spectral_contrast` | subnormal f32 data (`1e-42`) and data that makes `dot_product` underflow to `0` while elements are non-zero ⇒ divide by `0` | [x] |
| C11 | `spectral_contrast` | huge f32 data (`1e38`) ⇒ `mulss` overflows to `+inf` in **single** precision even though an f64 dot product would be finite — the direct signature of the `float_t` confusion | [x] |
| C12 | `spectral_contrast` | **fully aliased**: `spectral_contrast(v, v, length)` (E12), `length` 1/2/16/17/64, random data | [x] |
| C13 | `spectral_contrast` | **partially overlapping**: `spectral_contrast(v, v.add(1), length)` and `spectral_contrast(v.add(1), v, length)` (E13) | [x] |
| C14 | `spectral_contrast` | `length < 0`: `-1`, `-7`, `-1000`, `INT_MIN` (E7) | [x] |
| C15 | `match` | ~~`bins = 0`~~ — **RECLASSIFIED after measurement.** This is not a valid configuration: C `match` with `bins == 0` always dies with SIGSEGV, because `differentiate`'s unguarded `v[length-1] = 0` writes `%rsp - 8`, the slot holding `preprocess`'s return address (verified standalone: exit 139). Moved to the error surface as `ERRORS.md` row E5 and tested in `tests/errors.rs::e5_match_bins_zero_crashes_in_c`, which asserts the C crash in a child process. | [x] |
| C16 | `match` | `bins = 1`, random f64 data, `threshold` swept over the value classes (E16 zero-magnitude path) | [x] |
| C17 | `match` | `bins = 2 .. 8` exhaustive (odd **and** even, E17), random realistic non-negative spectra, `threshold` random in `[-0.5, 1.5]` | [x] |
| C18 | `match` | `bins = 15, 16, 17` — straddles `N_SMOOTH`: `<16` ⇒ **every** output attenuated by the clamped window (E18); `==16` ⇒ only element 0 gets a full window; `>16` ⇒ full windows exist | [x] |
| C19 | `match` | `bins` = 31, 32, 33, 63, 64, 100, 127, 128, 256, 1024; random realistic spectra; `threshold` random | [x] |
| C20 | `match` | gate **taken** (`return 0` before preprocessing, E1): `test` scaled far below `reference`, `threshold = 1.0` | [x] |
| C21 | `match` | gate **not taken**: `test` energy `>=` `threshold *` `reference` energy | [x] |
| C22 | `match` | gate **unordered** (E2/E3): `threshold = NaN`; `threshold = inf` with `total(reference) = 0`; `total(test) = NaN` | [x] |
| C23 | `match` | `test` and `reference` are the **same pointer** (identical spectra) — contrast should be ≈1 | [x] |
| C24 | `match` | **constant** spectra (all elements equal), plus all-`+0.0`, all-`-0.0` and mixed-`±0.0` spectra. **MEASURED correction:** a constant spectrum does *not* preprocess to all zeros — `smoothen`'s clamped tail (E18) turns `[7.5, 7.5]` into `[0.9375, 0.46875]`, then `[-0.46875, 0]`, then `[-0.029296875, 0]` — so it yields a finite contrast. It is the **all-zero** spectrum that reaches the zero-magnitude/NaN-contrast path (E9). Both are covered. | [x] |
| C25 | `match` | monotone ramp / impulse / step / alternating-sign spectra (shapes that make `differentiate` output large, and that exercise the smoothing tail) at `bins` 16, 17, 33 | [x] |
| C26 | `match` | spectra with `±inf`, quiet NaN, signalling NaN elements; `threshold` random | [x] |
| C27 | `match` | spectra of **arbitrary raw f64 bit patterns** (`u64` from the PRNG, no filtering) at `bins` 1..8, 16, 17, 33, 64 — the highest-yield row: the f32 reinterpretation of random f64 mantissas hits NaN/inf/denormal f32 values at high probability | [x] |
| C28 | `match` | spectra whose f64 values are huge (`1e300`) so that the **high** 32-bit half of each `double` decodes to an f32 NaN/inf, and tiny (`1e-300`) so the high half decodes to an f32 denormal/zero | [x] |
| C29 | `match` | `threshold` at exact boundaries: `contrast == threshold` exactly (`threshold` = the contrast value returned by a previous C call) ⇒ tests `>=` vs `>` | [x] |
| C30 | `match` | `threshold` = `-0.0` and `+0.0` (distinguishable bit patterns, equal under `>=`), and `threshold` = `f64::MIN_POSITIVE`, `f64::MAX` | [x] |
| C31 | `match` + `spectral_contrast` | composed check: run `match`'s pipeline manually via the exported `spectral_contrast` on the *same* buffers `match` would build, confirming the exported low-level entry point and the internal call agree | [x] |
| C32 | both | idempotence / no-side-channel: call each entry point twice in a row on the same buffer and compare both the return values and the twice-mutated buffers (catches state left in statics — there are none, but this pins it) | [x] |

## Row → test mapping

Each row is checked off only after its test passes across the randomized inputs.
Tests live in `tests/configs.rs`; `REPS` (default 120) inputs per row × shape.

| rows | test |
|---|---|
| C1 | `c1_spectral_length_zero` |
| C2 | `c2_spectral_length_one` |
| C3 | `c3_spectral_small_lengths` |
| C4, C5 | `c4_c5_spectral_boundary_and_large_lengths` |
| C6 | `c6_spectral_zero_magnitude` |
| C7–C11 | `c7_to_c11_spectral_special_values` |
| C12 | `c12_spectral_aliased_same_pointer` |
| C13 | `c13_spectral_partial_overlap` |
| C14 | `c14_spectral_negative_length` |
| C15 | reclassified → `tests/errors.rs::e5_match_bins_zero_crashes_in_c` |
| C16 | `c16_match_bins_one` |
| C17 | `c17_match_small_bins` |
| C18 | `c18_match_nsmooth_boundary` |
| C19 | `c19_match_large_bins` |
| C20 | `c20_match_gate_taken` |
| C21 | `c21_match_gate_not_taken` |
| C22 | `c22_match_gate_unordered` |
| C23 | `c23_match_same_pointer` |
| C24 | `c24_match_constant_spectra` |
| C25 | `c25_match_shapes` |
| C26 | `c26_match_special_spectra` |
| C27 | `c27_match_raw_bit_patterns` |
| C28 | `c28_match_huge_and_tiny` |
| C29 | `c29_match_threshold_equals_contrast` |
| C30 | `c30_match_threshold_boundaries` |
| C31 | `c31_pipeline_via_exported_low_level_entry_point` |
| C32 | `c32_repeated_calls_no_hidden_state` |
| all, crossed | `tests/soak.rs` — 500,000 fixed-seed random cases per entry point, mixing every value class, every shape and every aliasing mode |

## No binary target

`c_src/CMakeLists.txt` declares only `add_library(... SHARED ...)`; there is no
`add_executable`, and `translation/Cargo.toml` declares only `[lib]` with
`crate-type = ["cdylib"]` (no `src/main.rs`, no `[[bin]]`). There is therefore
no driver executable and no stdout to compare. That completion-gate item is
vacuous for this project, not skipped.

## Compiler-sensitivity note (measured, affects how to read these rows)

The documented build (`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON` with no
`CMAKE_BUILD_TYPE`) compiles at `-O0`, and that is the artifact every row above
is checked against. Re-running the entire suite with the C sources rebuilt at
`-O1`, `-O2`, `-O3` and `-Os` (see `run_all.sh`) leaves every `match` row and
every `ERRORS.md` row passing unchanged; the only difference is the payload bits
of a NaN returned directly out of `spectral_contrast`, which the C source does
not determine. See the `VERIFICATION` block in `src/lib.rs`.
