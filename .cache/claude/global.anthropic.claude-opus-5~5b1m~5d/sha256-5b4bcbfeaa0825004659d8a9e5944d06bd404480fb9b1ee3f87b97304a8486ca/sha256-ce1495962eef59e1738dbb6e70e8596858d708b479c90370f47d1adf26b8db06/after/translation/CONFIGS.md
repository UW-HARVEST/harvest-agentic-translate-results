# CONFIGS.md — configuration / valid-input surface table

## Where the axes come from (mechanical derivation)

The library has **no** runtime flags, no options struct, no environment reads,
no `#ifdef`, no `switch`. `grep`ing `c_src` for `#if|#ifdef|switch|getenv|static
.*=` returns nothing but the single `#define N_SMOOTH 16`. So every branch the C
takes is driven by (1) which entry point you call, (2) the integer length, and
(3) the *values* in the buffers. Those are the axes:

| axis | values the C actually distinguishes | where it branches |
|------|-------------------------------------|-------------------|
| **A. entry point** | `spectral_contrast` (lowest level, exported), `match` (composed: `total`→gate→`preprocess`×2→`spectral_contrast`→cut-off) | `match.h` |
| **B. element type seen** | `f64` stride-8 (`match.c`, `float_t == double`), `f32` stride-4 (`spectral_contrast.c`, `float_t == float`) | the `float_t` split |
| **C. length vs `N_SMOOTH == 16`** | `1`, `2`, `3`, `15`, `16`, `17`, `31`, `32`, `33`, `64`, `1000` — `smoothen`'s inner loop is `j < 16 && i+j < length`, so for `length <= 16` **every** output is clamped, for `length > 16` only the last 15 are | `match.c:16` |
| **D. length parity** | odd vs even — `spectral_contrast` reads `bins` **f32** slots out of a `bins`-**f64** buffer, i.e. `ceil(bins/2)` doubles; for odd `bins` the last touched double contributes only its **low** 4 bytes | interaction of `match.c:40` with the `float_t` split |
| **E. `differentiate` degenerate length** | `length == 1` (loop body never runs, the single element is zeroed) vs `length >= 2` | `match.c:24-25` |
| **F. element value class** | `+0.0`, `-0.0`, subnormal, small normal, `1.0`, large normal, overflow-to-`inf` in f32 (`>= 2e19` squared), `±inf`, quiet NaN, signalling NaN, mixed signs, monotone ramp, constant | every arithmetic op; `normalize` divides by `sqrt(dot)` with no guard |
| **G. `threshold` class** | `-inf`, `-DBL_MAX`, `-1.0`, `-0.0`, `+0.0`, `DBL_MIN`, `0.25`, `0.5`, `1.0`, `2.0`, `DBL_MAX`, `+inf`, `NaN` | `match.c:37` (`mulsd` + ordered `<`) and `match.c:40` (ordered `>=`) |
| **H. test/reference relationship** | identical, positively scaled, negated, shifted, independent random, reference all-zero, test all-zero, one constant | drives which side of both gates you land on |
| **I. pointer relationship** | distinct, fully aliased (`a == b`), partially overlapping (`b == a+1`, `b == a+k`) | C permits it; `normalize` is called twice so aliasing is observable |
| **J. caller's declared element type for `spectral_contrast`** | called with a genuine `float*` buffer (true ABI) **and** called with a `double*` buffer the way `match.h` declares it (the type-confused path a real consumer takes) | `match.h` vs `spectral_contrast.c` |

Rows below are the pruned cross-product: one row per combination the C treats
differently. Each is driven with **many** randomized inputs from a fixed-seed
PRNG (`SplitMix64`, seed `0x9E3779B97F4A7C15`), not a single hand-picked value,
and compared **bit-for-bit** (raw IEEE-754 bits of the return value *and* of
every element of every buffer after the call, since both entry points mutate).

Test file: `tests/configs.rs`. Row ids are the `#[test]` name suffixes.

## `spectral_contrast` — true-ABI `f32` buffers (axis A=low-level, B=f32, J=float\*)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C01 | `spectral_contrast` | `length` ∈ {1,2,3,15,16,17,31,32,33,64,1000}, distinct buffers, random *positive normal* f32 in `[2^-40, 2^40]` (500 draws/len) | [x] |
| C02 | `spectral_contrast` | as C01 but *signed* random normals (mixed signs) | [x] |
| C03 | `spectral_contrast` | as C01 but random f32 drawn from arbitrary **bit patterns** (covers `±0`, subnormals, `±inf`, quiet+signalling NaN, every exponent) — 2000 draws | [x] |
| C04 | `spectral_contrast` | random exponent-biased draws: exponent uniform over the full f32 range, so magnitudes span `1e-45 … 1e38` and the dot product both underflows and overflows | [x] |
| C05 | `spectral_contrast` | `a` all-equal constant (random constant), `b` random | [x] |
| C06 | `spectral_contrast` | `a == b` values (equal contents, distinct buffers) ⇒ contrast should be `1.0`-ish; exercises `dot(v,v)` twice | [x] |
| C07 | `spectral_contrast` | `b[i] == -a[i]` ⇒ contrast ≈ `-1.0` | [x] |
| C08 | `spectral_contrast` | monotone ramps (`i`, `-i`, `i*scale`) | [x] |
| C09 | `spectral_contrast` | all elements subnormal (magnitude underflows to `+0` ⇒ divide-by-zero) | [x] |
| C10 | `spectral_contrast` | all elements `> 2^64` so `x*x` overflows f32 to `+inf` ⇒ magnitude `+inf` | [x] |
| C11 | `spectral_contrast` | exactly one element `±inf`, rest random finite | [x] |
| C12 | `spectral_contrast` | exactly one element quiet NaN with a random payload, rest random finite | [x] |
| C13 | `spectral_contrast` | exactly one element signalling NaN with a random payload | [x] |
| C14 | `spectral_contrast` | **two or more** NaNs with distinct payloads in `a` (payload-precedence in `addsd`, axis F ∩ E28) | [x] |
| C15 | `spectral_contrast` | NaN at the *same* index in both `a` and `b`, distinct payloads (`mulss` destination precedence, E27) | [x] |
| C16 | `spectral_contrast` | `±0.0` mixture (some `+0`, some `-0`), rest zero ⇒ zero magnitude | [x] |
| C17 | `spectral_contrast` | **aliased**: `a == b`, random contents, `length` ∈ {1,2,16,17,64} (axis I) | [x] |
| C18 | `spectral_contrast` | **partially overlapping**: `b = a.add(1)`, and `b = a.add(k)` for random `k < length` (axis I) | [x] |
| C19 | `spectral_contrast` | `length` sweep `1..=64` exhaustively with one fixed random buffer per length (axis C boundary scan around 16 and 32) | [x] |
| C20 | `spectral_contrast` | large `length` (4096, 65536) random normals — accumulation-order sensitivity | [x] |

## `spectral_contrast` — header-declared `double*` caller (axis J=double\*, the type-confused path)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C21 | `spectral_contrast` | buffer of `n` `double`s, called with `length = n` (reads the low half of the bytes ⇒ mixture of mantissa-derived f32s), random positive doubles, `n` ∈ {1,2,3,15,16,17,33,64} | [x] |
| C22 | `spectral_contrast` | buffer of `n` `double`s, called with `length = 2n` (reads *all* the bytes as f32) | [x] |
| C23 | `spectral_contrast` | buffer of `n` `double`s with arbitrary random **bit patterns**, `length = 2n` — dense NaN/inf/subnormal coverage in the reinterpreted f32s | [x] |
| C24 | `spectral_contrast` | doubles chosen so their *low* words are NaN f32 patterns and their high words are finite (odd-`length` corner from axis D) | [x] |

## `match` — composed pipeline (axis A=high-level, B=f64)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C25 | `match` | `bins` ∈ {1,2,3,15,16,17,31,32,33,64,257,1000} × `threshold` ∈ {0.25,0.5,1.0}, random positive spectra (realistic), 300 draws/cell | [x] |
| C26 | `match` | `bins` sweep `1..=80` (axis C/D/E boundary scan: `<16`, `==16`, `>16`, odd/even) × `threshold = 0.5`, fresh random data per `bins` | [x] |
| C27 | `match` | `test == reference` contents ⇒ contrast `1.0`, gate passes ⇒ expect `1` for `threshold <= 1` | [x] |
| C28 | `match` | `reference = test * s` for random `s` ∈ {1e-6, 0.5, 1, 2, 1e6} (gate is a *ratio* test) | [x] |
| C29 | `match` | `reference = -test` | [x] |
| C30 | `match` | `test` all-zero (gate: `0 < threshold*total(ref)`) — hits E1 for `threshold>0`, falls through for `threshold<=0` | [x] |
| C31 | `match` | `reference` all-zero (`threshold*0 = ±0` or `NaN`) | [x] |
| C32 | `match` | both all-zero ⇒ `differentiate` produces all-zero ⇒ zero magnitude ⇒ `NaN` contrast | [x] |
| C33 | `match` | constant (non-zero) spectra ⇒ `smoothen`+`differentiate` yield the clamped tail only | [x] |
| C34 | `match` | monotone ramp spectra (up, down) | [x] |
| C35 | `match` | full `threshold` sweep `{-inf,-DBL_MAX,-1,-0.0,+0.0,DBL_MIN,0.25,0.5,1.0,2.0,DBL_MAX,+inf,NaN}` × `bins` ∈ {1,2,16,17,64} × random data (axis G) | [x] |
| C36 | `match` | random doubles with **arbitrary bit patterns** (NaN/inf/subnormal doubles in the input, dense NaN coverage after reinterpretation), `bins` ∈ {2,16,17,64} | [x] |
| C37 | `match` | exponent-biased random doubles spanning `1e-300 … 1e300` (overflow/underflow inside `total` and the f32 reinterpretation) | [x] |
| C38 | `match` | one `±inf` element, one NaN element (separately), rest random | [x] |
| C39 | `match` | **aliased** `test == reference` (same pointer) (axis I) | [x] |
| C40 | `match` | **partially overlapping** `reference = test.add(1)` with `bins` elements available (axis I) | [x] |
| C41 | `match` | input buffers must be **unmodified** by `match` (it only reads them) — asserted for every row above | [x] |
| C42 | `match` | spectra with a sharp spike (one element `1e9`, rest ~1) — exercises `differentiate`'s large deltas and `smoothen`'s window | [x] |
| C43 | `match` | spectra that are periodic with period 16 (`N_SMOOTH`) so `smoothen` sums a whole period | [x] |
| C44 | `match` | `bins` = 4096 random positive spectra (large-buffer accumulation) | [x] |

## Exhaustive special-value cross-products (added after the randomized rows)

Randomized sampling only hits IEEE-754 special values by chance, and never hits
particular *combinations* of them. These rows enumerate the full cross-product
of one representative bit pattern per class/boundary at the smallest lengths,
which is exactly where the class interactions live (`0*inf`, `inf-inf`,
`inf/inf`, subnormal², sNaN-vs-qNaN payload precedence, `±0` signs, values whose
square overflows, …).

Representatives: 25 f32 patterns (`F32_CLASSES`) and 16 f64 patterns
(`F64_CLASSES`) in `tests/configs.rs`.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C45a | `spectral_contrast` | `length = 1`, all 25×25 `(a[0], b[0])` class pairs, distinct **and** aliased | [x] |
| C45b | `spectral_contrast` | `length = 2`, all 25⁴ = 390 625 `(a[0],a[1],b[0],b[1])` class combinations | [x] |
| C45c | `spectral_contrast` | `length = 2`, `b = a + 1` (partial overlap), all 25³ class triples | [x] |
| C46a | `match` | `bins = 2`, all 16⁴ = 65 536 class combinations × `threshold` ∈ {0.5, 1.0, −1.0, NaN} | [x] |
| C46b | `match` | `bins = 1`, all 16×16 class pairs × the full 13-value `threshold` sweep | [x] |
| C46c | `match` | `bins = 3` (odd — the last touched double contributes only its low word), all 16³ class triples | [x] |

## How the rows were driven

* `./run_tests.sh` rebuilds **both** shared objects and runs the whole suite.
  (`cargo test` alone is not sufficient: it builds test harnesses but does not
  re-emit a `cdylib`, so the Rust `.so` under test would be stale. The test
  harness now refuses to run against a `.so` older than `src/lib.rs`.)
* `./stress.sh N` re-runs everything with `DIFF_SEED=0..N-1`, i.e. `N`
  independent, still fully reproducible random corpora. **45 corpora agree.**
* `./features.sh` runs the suite under every feature combination declared in
  `Cargo.toml`. There is no `[features]` table, so the two configurations are
  *default* and `--no-default-features`; both pass.
* The suite also passes with the **debug-profile** Rust `.so`
  (`RUST_SO=target/debug/libunderhanded_c_nuke_lib.so cargo test --release`),
  which rules out opt-level-dependent NaN handling on the Rust side.
* **Mutation check** — the suite was pointed at a `-O2` build of the *same* C
  sources in the Rust slot (`RUST_SO=…/libo2.so`). 13 of the 48 rows fail,
  proving the rows genuinely discriminate at the bit level rather than passing
  vacuously, and quantifying the `-O0`/`-O2` NaN-payload split documented in
  `src/lib.rs`. `match`-only rows do **not** fail, confirming that `match`'s
  `int` result is payload-insensitive.
