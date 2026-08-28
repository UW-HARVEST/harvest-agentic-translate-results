# VERIFICATION.md — completion gate

Reproduce everything with `./verify.sh` (add `--quick` for release only).

## Completion gate

- [x] **`SYMBOLS.md`**: `nm -D` shows **0 missing** and **0 undefined non-libc**
      symbols in the Rust `.so`. The C `.so` exports exactly one symbol,
      `premultiply`; the Rust `.so` exports it under the identical name.
      `ldd -r` reports no unresolved relocations for either library.
      Enforced continuously by `tests/symbols.rs` (4 tests).
- [x] **Phase B**: all **32 rows** of `CONFIGS.md` pass across randomized inputs
      (`tests/differential.rs`).
- [x] **Phase C**: all **25 rows** of `ERRORS.md` have a passing error-path
      differential test, plus 3 generic FFI-boundary sweeps
      (`tests/errors.rs`, 28 tests + 1 deliberately-ignored crash child).
- [x] **Every feature combination**: `Cargo.toml` declares no `[features]` table
      and no optional dependencies, so `default` == `--no-default-features` ==
      `--all-features`. All three were run explicitly, under **both** the
      `release` and `debug` profiles — 6 configurations, all green.
      `tests/features.rs` fails if a `[features]` table is ever added, forcing
      the matrix to be extended.

Totals: **66 tests**, 6 configurations, 0 failures.

## What the tests actually do

Both libraries are loaded with `libloading::Library::new` and invoked only
through `dlsym("premultiply")`. The Rust crate is a `cdylib` and is **never**
linked directly by the tests, so every assertion also exercises the
`#[no_mangle] extern "C"` export wrapper.

Each case runs the C `.so` and the Rust `.so` over independently allocated but
identically seeded buffers, then requires:

1. byte-identical pixel payloads,
2. intact 64-byte canaries on both sides of the payload (no over/under-write),
3. an unmodified `cp_image_t` struct,
4. no writes at or beyond the computed `limit`,
5. the alpha byte (`data[i+3]`) preserved — the C stores only `+0/+1/+2`.

Highlights of the coverage:

* **Exhaustive float verification.** The channel maths is value-dependent, so
  row 1 sweeps the *complete* 256x256 `(colour, alpha)` cross-product — all
  65 536 combinations — rather than sampling.
* **Exhaustive dimension sweep.** `generic_out_of_range_dimension_sweep` walks
  every `±2^k ± 1` boundary in both dimensions: **19 709** `(w, h)` pairs
  executed, 1 071 of which actually enter the loop.
* **Crash parity out-of-process.** Null / unmapped pointers are exercised in a
  forked child so the *exact fault signal* can be compared instead of merely
  "both failed somehow".

## The C quirk that dominates this library

```c
int stride = w * sizeof(cp_pixel_t);            /* size_t multiply, truncated to int */
for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))
```

`stride` is a **byte** count that is truncated to `int`, and the bound
`stride * h` is a wrapping 32-bit multiply. Consequences the tests pin down:

| input | effect |
|-------|--------|
| `w < 0 && h < 0` | `limit` becomes **positive** — the loop **runs** |
| `w ∈ {2^30, -2^30, INT_MIN}` | `stride` wraps to `0` — no-op for every `h` |
| `w = 2^29+1, h = 2` | `limit` wraps to `+8` — exactly **2 pixels** processed |
| `w = INT_MAX, h = INT_MAX` | `limit` wraps to `+4` — exactly **1 pixel** processed |
| `w = 1, h = INT_MAX` | `limit` wraps to `-4` — no-op |

A translation that "fixed" any of these (e.g. rejecting negative dimensions, or
using checked/saturating arithmetic) would be **wrong**. Mutation testing
confirms each such "fix" is detected.

## Divergences found and fixed

| # | symptom | root cause | fix |
|---|---------|-----------|-----|
| 1 | Rust aborted with `misaligned pointer dereference` on a misaligned `cp_image_t *`, where C processed the image normally (debug profile) | `let img = &mut *img;` / plain `(*img).w` imposes Rust's alignment and non-null requirements, which the C code does not have | read the three fields with `core::ptr::read_unaligned(addr_of!(...))` |
| 2 | Rust could let the optimiser assume `data + i` stays inside one allocation | `ptr::offset` carries an in-bounds precondition; the C does plain address arithmetic | `wrapping_offset` |

Divergence 1 was a genuine behavioural difference on a valid input, found only
because the matrix includes the debug profile — the release build silently
compiled the check out. Both fixes were re-validated across all 6 configurations.

## Mutation testing (harness validation)

Passing tests only prove something if they can fail. 15 deliberate bugs were
injected into `src/lib.rs`; **all 15 were caught**:

`round-instead-of-trunc`, `ceil-instead-of-trunc`, `saturating-stride`,
`saturating-limit`, `writes-alpha`, `reject-negative-dims`, `off-by-one-bound`,
`swap-g-b`, `no-premultiply`, `row-stride-misread`, `clamp-w-to-zero`,
`alpha-from-wrong-index`, `step-by-3`, `div-by-256`, `aligned-field-read`
(the last one only reproduces in the debug profile, which is why the matrix
covers both), plus `null-guard` which is caught by the crash-parity tests in
*both* profiles.

Two further mutants survived and were **proven mathematically equivalent** over
the entire 65 536-point input domain, so they are not coverage gaps:

* computing alpha in `f64` before narrowing to `f32` — 0 mismatches;
* replacing the float pipeline with integer `(c * a) / 255` — 0 mismatches.

## Known profile-dependent behaviour (documented, not a translation defect)

For `img == NULL` and `img->pix == NULL`, a **release** Rust `.so` faults with
`SIGSEGV(11)`, exactly like C. A **debug** Rust `.so` carries the standard
library's UB pre-condition checks, which turn the same input into a controlled
`abort()` → `SIGABRT(6)` before the faulting load executes. This is Rust
toolchain instrumentation, not translated logic; the shipped artifact is the
release `cdylib`. `tests/errors.rs` therefore demands exact signal parity in
release, and in debug demands the Rust still die from a fatal signal — so a
Rust that wrongly *returns* on NULL is still caught in both profiles (verified
by mutation).

## Layout assumptions asserted at run time

`size_of::<cp_pixel_t>() == 4`, `align_of::<cp_pixel_t>() == 1`,
`size_of::<c_int>() == 4` — checked in `tests/common/mod.rs` before any
comparison, because the C code hard-codes `sizeof(cp_pixel_t)` as its stride
unit and loop step.
