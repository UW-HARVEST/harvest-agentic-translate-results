# CONFIGS.md — Phase A: configuration-surface table

Derived **mechanically** from the branches `c_src/src/lib.c` actually takes.

## Axis derivation (from the source, not from guesses)

### Runtime options / modes / flags — **none**

| candidate | finding |
|---|---|
| public header declarations | `c_src/include/lib.h` is 1 line: `void rgb_to_hsv(float*, const float*)`. No option struct, no context handle, no setter, no flag argument. |
| `#ifdef` / `#if` / `#define` in library source | 0 occurrences (see `ERRORS.md` grep audit). No compile-time configuration. |
| global / `static` state | 0 — the function is pure over its arguments. |
| Cargo features | `translation/Cargo.toml` has **no `[features]` section** → the only combination is the default (empty) feature set. Verified in Phase D. |

So every axis below is an **input-shape** axis. That is the entire
configuration surface this library has.

### Public entry points — the full set

There is exactly **one** exported entry point, `rgb_to_hsv`, and it *is* the
lowest level (there is no convenience wrapper layered over a lower-level core).
Every row therefore drives `rgb_to_hsv` directly through the `.so` export.

### Shape axes the C branches on

| axis | source line(s) | distinct values the code treats differently |
|---|---|---|
| **H1** hue-branch selection | 26, 28, 30 | `r == max` / `g == max ∧ r ≠ max` / neither |
| **H2** degenerate early-out | 19 | `delta == 0` / `max == 0` / neither |
| **H3** hue wrap correction | 33 | `h < 0` (wrap `+360`) / `h ≥ 0` |
| **H4** min/max ternary tie direction | 13–16 | operands `<`/`>` **false** on equality *and* on unordered → the ternary returns the **second** operand. Observable via signed zeros and NaN. |
| **H5** channel tie multiplicity | 26/28 interaction with 13–16 | 3 distinct / one pair tied (3 placements) / all 3 tied |
| **H6** IEEE class per channel | all arithmetic | normal-in-range, normal-out-of-range, negative, `±0.0`, subnormal, `±Inf`, `NaN` (quiet + signalling + custom payload) |
| **H7** magnitude interaction | 17, 25, 27/29/31 | `delta` overflow→`Inf`, `delta/max` overflow→`Inf`, `delta/max` underflow, `Inf/Inf`→`NaN` |
| **H8** pointer aliasing | 4–6 loads vs. 20–22/35–37 stores | `dest` disjoint from `src` / `dest == src` / forward overlap / backward overlap (all three loads precede all stores, confirmed in `objdump`) |

## The table — one row per combination the C distinguishes

Every row: **many randomized inputs** (fixed seed `0x5EED_C0DE`), both `.so`s
called through `libloading`, outputs compared **bit-for-bit** (`to_bits()`, so
`-0.0 ≠ +0.0` and NaN payloads must match exactly).

| # | entry point(s) | configuration (options set + input shape) | axes | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `rgb_to_hsv` | no options (none exist) · branch A: `r` strict max, `r > g > b`, in-range `[0,1]` → `h ≥ 0`, no wrap | H1a H3b | [x] |
| 2 | `rgb_to_hsv` | branch A: `r` strict max, `r > b > g` (so `g < b`) → `h < 0` → **wrap `+360`** | H1a H3a | [x] |
| 3 | `rgb_to_hsv` | branch A with `g == b` (both `< r`) → `h` exactly `0`, wrap not taken | H1a H3b H5b | [x] |
| 4 | `rgb_to_hsv` | branch B: `g` strict max, `g > r > b`, in-range | H1b H3b | [x] |
| 5 | `rgb_to_hsv` | branch B: `g` strict max, `g > b > r` | H1b H3b | [x] |
| 6 | `rgb_to_hsv` | branch B with `r == b` (both `< g`) → `h` exactly `120` | H1b H5b | [x] |
| 7 | `rgb_to_hsv` | branch C: `b` strict max, `b > r > g`, in-range | H1c H3b | [x] |
| 8 | `rgb_to_hsv` | branch C: `b` strict max, `b > g > r` | H1c H3b | [x] |
| 9 | `rgb_to_hsv` | branch C with `r == g` (both `< b`) → `h` exactly `240` | H1c H5b | [x] |
| 10 | `rgb_to_hsv` | tie `r == g == max > b` → **branch A wins** (`r == max` tested first) | H1a H5b | [x] |
| 11 | `rgb_to_hsv` | tie `r == b == max > g` → branch A wins | H1a H5b | [x] |
| 12 | `rgb_to_hsv` | tie `g == b == max > r` → **branch B wins** (`r ≠ max`, `g == max`) | H1b H5b | [x] |
| 13 | `rgb_to_hsv` | early-out via `delta == 0`: `r == g == b ≠ 0` (valid gray), in-range | H2a H5c | [x] |
| 14 | `rgb_to_hsv` | early-out with all channels exactly `+0.0` → `dest = {0,0,+0.0}` | H2a H2b | [x] |
| 15 | `rgb_to_hsv` | **signed-zero ternary asymmetry**: all 8 sign combinations of `±0.0` across `(r,g,b)` → sign bit of `v` follows the C ternary's "return 2nd operand on equality" | H4 H6d | [x] |
| 16 | `rgb_to_hsv` | negative channels with `max > 0` (out of documented `[0,1]`) → `delta > max`, so `s > 1` unclamped | H6c H6b | [x] |
| 17 | `rgb_to_hsv` | negative channels with `max == ±0.0`, `min < 0` → early-out **second** disjunct; short-circuit avoids `delta/0` | H2b H6c | [x] |
| 18 | `rgb_to_hsv` | **all** channels negative (`max < 0`) → `s = delta/max` is **negative** | H6c | [x] |
| 19 | `rgb_to_hsv` | channels scaled to `[0, 255]` (out of range, integral values → frequent exact ties) | H6b H5 | [x] |
| 20 | `rgb_to_hsv` | huge magnitudes / `±FLT_MAX` such that `max - min` **overflows to `+Inf`** → `s = Inf/finite = Inf`, ratio `finite/Inf = ±0` | H7a H6b | [x] |
| 21 | `rgb_to_hsv` | all channels subnormal (`FLT_TRUE_MIN` … `FLT_MIN`) | H6e | [x] |
| 22 | `rgb_to_hsv` | subnormal `delta` with large `max` → `s` **underflows** (subnormal or `+0`) while `h` ratio is `O(1)` | H7c H6e | [x] |
| 23 | `rgb_to_hsv` | subnormal `max` with subnormal `delta` → `delta/max` normal or **overflows to `Inf`** | H7b H6e | [x] |
| 24 | `rgb_to_hsv` | `±Inf` channels, incl. `+Inf` with `-Inf` → `delta = Inf`, `s = Inf/Inf = NaN`, `Inf - Inf = NaN` | H6f H7d | [x] |
| 25 | `rgb_to_hsv` | `NaN` in `r` only → ternaries **discard** it (`NaN < g` false ⇒ `min = g`; `NaN > g` false ⇒ `max = g`); `r == max` false ⇒ branch B or C | H6g H4 H1 | [x] |
| 26 | `rgb_to_hsv` | `NaN` in `g` only → `max` transiently `NaN` then replaced by `b`; `min` becomes `NaN`-free only if `b < NaN`-slot ⇒ distinct path | H6g H4 | [x] |
| 27 | `rgb_to_hsv` | `NaN` in `b` only → `NaN` is the **last** ternary operand ⇒ `min = max = NaN`, `delta = NaN`, both `==` tests false ⇒ **branch C** | H6g H4 H1c | [x] |
| 28 | `rgb_to_hsv` | `NaN` in 2 and in all 3 channels; signalling `NaN` and custom payloads → quieting + payload propagation must be bit-identical | H6g | [x] |
| 29 | `rgb_to_hsv` | **aliasing** `dest == src` (in-place), over randomized in-range and special inputs | H8b | [x] |
| 30 | `rgb_to_hsv` | **aliasing** forward overlap `dest == src + 1` (4-float buffer) | H8c | [x] |
| 31 | `rgb_to_hsv` | **aliasing** backward overlap `dest == src - 1` (4-float buffer) | H8d | [x] |
| 32 | `rgb_to_hsv` | property sweep: uniform random `[0,1]³`, 20 000 vectors | H1×H3 | [x] |
| 33 | `rgb_to_hsv` | property sweep: **uniform random 32-bit patterns** per channel (every IEEE class incl. `NaN`/`Inf`/subnormal/`±0`), 50 000 vectors | H6 all | [x] |
| 34 | `rgb_to_hsv` | property sweep: channels from a small integer grid `{-2..2}/2` → dense exact ties & zeros, 8 000 vectors | H5 H4 | [x] |
| 35 | `rgb_to_hsv` | **exhaustive** cross product of a 24-value curated special set over `(r,g,b)` = 13 824 vectors | H4×H5×H6 | [x] |
| 36 | `rgb_to_hsv` | statelessness / call-sequence independence: the same vector replayed after 1 000 interleaved random calls yields identical bits (no hidden global state) | — | [x] |

## Build-artifact axis (applied on top of every row above)

The 36 rows are not run once but **five times**, against every combination of
build artifact, because codegen choices are exactly where float results could
silently drift:

| # | Rust artifact | C reference | why |
|---|---|---|---|
| A1 | `target/debug` (unoptimised, `debug_assertions` on) | CMake build (`-fPIC`, no `-O`) | the graded reference pair |
| A2 | `target/release` (optimised, `panic = "abort"`) | CMake build | the artifact that ships; proves optimisation perturbs no result bit |
| A3 | `target/debug`, `--no-default-features` | CMake build | feature-flag parity |
| A4 | `target/release`, `--no-default-features` | CMake build | feature-flag parity |
| A5 | `target/release` | **`cc -O2`** build of the same untouched `c_src/src/lib.c` | proves the agreement is not an artefact of one C codegen choice (no dependence on x87 excess precision or a compiler-chosen NaN operand order) |

All five are driven by `./verify_all.sh`.

## Gate

- [x] Every row passes across its randomized inputs, bit-for-bit
      (`translation/tests/phase_b_configs.rs`).
- [x] Every row passes under all five build-artifact combinations above.
