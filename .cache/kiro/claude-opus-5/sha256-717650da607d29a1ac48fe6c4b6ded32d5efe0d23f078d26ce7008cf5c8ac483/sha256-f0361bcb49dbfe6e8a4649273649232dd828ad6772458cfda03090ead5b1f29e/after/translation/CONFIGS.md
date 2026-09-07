# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src` (the whole library is `include/lib.h`, 3
lines, and `src/lib.c`, 118 lines) plus `translation/Cargo.toml`.

## Axis 1 — runtime options / modes / flags: NONE

The public header is three lines:

```c
#include <stdint.h>
uint16_t float2half(float flt);
```

There is exactly **one** public entry point, it is also the **lowest-level**
entry point (there is no convenience wrapper and no wrapped internal function),
and it takes **one** argument. Mechanical grep over all of `c_src` returns 0
matches for `if`, `else`, `switch`, `case`, `?:`, `enum`, `#if`/`#ifdef`,
`goto`, and 0 pointer or length parameters. There is no global state, no
init/config function, no mode flag, and no conditional compilation, so there
are no option axes to cross.

## Axis 2 — compile-time / feature configuration: ONE

`translation/Cargo.toml` has no `[features]` section, so there is a single
configuration. `crate-type = ["cdylib"]` only and no `src/main.rs`;
`c_src/CMakeLists.txt` has 0 `add_executable`. **No binary driver exists on
either side**, so the "compare C and Rust stdout" gate is not applicable.

## Axis 3 — input shape: the branch structure hidden in the tables

`float2half` has no control flow; its branching is entirely **data-driven**
through the two 512-entry tables indexed by

```c
j = (n >> 23) & 0x1ff;   // = (sign_bit << 8) | biased_exponent
```

so the code distinguishes exactly the classes below. They were extracted
mechanically by grouping the 512 table entries into maximal contiguous runs of
equal `m__shift[j]`, i.e. the runs the C genuinely treats differently. `m`
below is the 23-bit mantissa `n & 0x007fffff`; result is
`m__base[j] + (m >> m__shift[j])`.

Each row is exercised with **many randomized mantissas and many randomized `j`
values inside the row's range** (fixed seed, SplitMix64), plus that row's
endpoint `j` values and the mantissa endpoints `0`, `1`, `0x7ffffe`,
`0x7fffff`. Rows 1–30 are the cross-product of {sign} x {exponent run}, pruned
to the runs the tables actually distinguish. Rows 31–34 add the whole-domain
sweeps.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `float2half` | no options (none exist); sign `+`, `j` 0..102, `shift 0x18`, `base 0x0000` — zero, float subnormals, and all finite magnitudes that flush to `+0` | [x] |
| 2 | `float2half` | sign `+`, `j` 103, `shift 0x17`, `base 0x0001` — smallest half subnormal | [x] |
| 3 | `float2half` | sign `+`, `j` 104, `shift 0x16`, `base 0x0002` | [x] |
| 4 | `float2half` | sign `+`, `j` 105, `shift 0x15`, `base 0x0004` | [x] |
| 5 | `float2half` | sign `+`, `j` 106, `shift 0x14`, `base 0x0008` | [x] |
| 6 | `float2half` | sign `+`, `j` 107, `shift 0x13`, `base 0x0010` | [x] |
| 7 | `float2half` | sign `+`, `j` 108, `shift 0x12`, `base 0x0020` | [x] |
| 8 | `float2half` | sign `+`, `j` 109, `shift 0x11`, `base 0x0040` | [x] |
| 9 | `float2half` | sign `+`, `j` 110, `shift 0x10`, `base 0x0080` | [x] |
| 10 | `float2half` | sign `+`, `j` 111, `shift 0x0f`, `base 0x0100` | [x] |
| 11 | `float2half` | sign `+`, `j` 112, `shift 0x0e`, `base 0x0200` — last graduated-underflow step | [x] |
| 12 | `float2half` | sign `+`, `j` 113..142, `shift 0x0d`, `base 0x0400..0x7800` — the normal-half range (30 distinct exponents, all swept) | [x] |
| 13 | `float2half` | sign `+`, `j` 143..254, `shift 0x18`, `base 0x7c00` — finite overflow saturating to `+Inf`, mantissa discarded | [x] |
| 14 | `float2half` | sign `+`, `j` 255, `shift 0x0d`, `base 0x7c00` — `+Inf` (`m == 0`) and all `+NaN` payloads (`m != 0`), mantissa honoured | [x] |
| 15 | `float2half` | sign `-`, `j` 256..358, `shift 0x18`, `base 0x8000` — `-0`, negative float subnormals, flush to `-0` | [x] |
| 16 | `float2half` | sign `-`, `j` 359, `shift 0x17`, `base 0x8001` | [x] |
| 17 | `float2half` | sign `-`, `j` 360, `shift 0x16`, `base 0x8002` | [x] |
| 18 | `float2half` | sign `-`, `j` 361, `shift 0x15`, `base 0x8004` | [x] |
| 19 | `float2half` | sign `-`, `j` 362, `shift 0x14`, `base 0x8008` | [x] |
| 20 | `float2half` | sign `-`, `j` 363, `shift 0x13`, `base 0x8010` | [x] |
| 21 | `float2half` | sign `-`, `j` 364, `shift 0x12`, `base 0x8020` | [x] |
| 22 | `float2half` | sign `-`, `j` 365, `shift 0x11`, `base 0x8040` | [x] |
| 23 | `float2half` | sign `-`, `j` 366, `shift 0x10`, `base 0x8080` | [x] |
| 24 | `float2half` | sign `-`, `j` 367, `shift 0x0f`, `base 0x8100` | [x] |
| 25 | `float2half` | sign `-`, `j` 368, `shift 0x0e`, `base 0x8200` | [x] |
| 26 | `float2half` | sign `-`, `j` 369..398, `shift 0x0d`, `base 0x8400..0xf800` — negative normal-half range (30 exponents) | [x] |
| 27 | `float2half` | sign `-`, `j` 399..510, `shift 0x18`, `base 0xfc00` — negative overflow saturating to `-Inf` | [x] |
| 28 | `float2half` | sign `-`, `j` 511, `shift 0x0d`, `base 0xfc00` — `-Inf` and all `-NaN` payloads; produces the max result `0xffff` | [x] |
| 29 | `float2half` | mantissa-shape sweep: for **every** `j` in 0..511, mantissa in {`0`, `1`, `2`, `0x400000`, `0x7ffffd`, `0x7ffffe`, `0x7fffff`} — the boundary shapes at every table index (3584 cases) | [x] |
| 30 | `float2half` | run-boundary sweep: every pair `(j-1, j)` at each of the 30 run transitions above, with mantissa endpoints — catches a table transcribed off by one element | [x] |
| 31 | `float2half` | randomized property sweep: 4,000,000 uniformly random 32-bit argument patterns, seeded SplitMix64 (seed `0x2545F4914F6CDD1D`) | [x] |
| 32 | `float2half` | randomized property sweep restricted to "interesting" floats (random sign x random exponent drawn from the run endpoints x random mantissa), 1,000,000 cases, same seed stream | [x] |
| 33 | `float2half` | ordinary-value sweep driven as a real consumer would: `f32` values built from `i as f32 / d as f32` and `powi` ladders, 200,000 cases | [x] |
| 34 | `float2half` | **exhaustive**: all 2^32 = 4,294,967,296 `f32` bit patterns through both `.so`s. Because the function's entire input domain is its single 32-bit argument, this row is a strict superset of rows 1–33 and of `ERRORS.md` rows 1–16, and constitutes a total proof of equivalence for this ABI. | [x] |

## Note on "drive the library as a real consumer does"

There is no state to set up and no pipeline to compose: the library exposes one
pure, stateless, branch-free function. Rows 1–33 call it through the `.so`
export with the full range of argument shapes; row 34 calls it with **every**
argument the ABI admits. Both sides are loaded with `libloading` and invoked
through their `#[no_mangle]` / C export symbols — the Rust function is never
called directly.

## Result

All 34 rows pass. Driver: `translation/verify_all.sh` (Phase D), which runs
every phase under `{debug, release} x {default, --no-default-features,
--all-features}` and then a cross-profile matrix of
`{debug, release} test binary x {debug, release} cdylib`.

The **debug**-profile cdylib is verified explicitly and separately, because the
dev profile enables `debug_assertions` and `overflow-checks`: the C wraps and
truncates silently, so a Rust translation using checked arithmetic or an
unguarded index would panic there while passing in release. It passes the full
2^32 sweep, which proves no input causes an index-out-of-bounds, a
shift-overflow, or an add-overflow panic.

Row 34 (exhaustive) reports:

```
exhaustive: all 4294967296 f32 bit patterns agree (32 shards)
```

### Harness credibility (mutation testing)

The differential harness was validated by deliberately corrupting the Rust
translation (then restoring `src/lib.rs` byte-identically) to confirm the tests
actually fail on divergence rather than passing vacuously:

| mutation to `translation/src/lib.rs` | detected by |
|-------------------------------------|-------------|
| `M__SHIFT[511]`: `0x0d` -> `0x18` (the single-element discontinuity at the end of the table) | 6 Phase C tests, incl. rows 1, 3, 4, 5, 13 |
| `M__BASE[130]`: `0x4800` -> `0x4801` (mid-run, `+ normal halves`) | Phase B rows 12, 29, 31, 33 and row 34, which reported **exactly 8388608 = 2^23** differing inputs — precisely the inputs with `j == 130`, confirming the sweep's coverage is complete and correctly indexed |
| mantissa mask `0x007f_ffff` -> `0x003f_ffff` | row 34, 335544320 differing inputs |

The two lookup tables were additionally compared element-by-element between
`c_src/src/lib.c` and `translation/src/lib.rs` by mechanical extraction of every
hex literal: 512/512 equal for `m__base`, 512/512 equal for `m__shift`.
