# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from the C source (the mirror of `ERRORS.md`, for VALID
inputs).

## Mechanical derivation of the axes

### Axis 1 — runtime options / modes / flags

Grep of the public header and the implementation for anything settable:

* `c_src/include/lib.h` declares **one** function and **zero** option setters,
  zero global variables, zero context/handle-init functions, zero flag enums.
* `c_src/src/lib.c` contains **zero** `if`, `switch`, `#if`, or `#ifdef`
  branches (see the grep transcript in `ERRORS.md`).

⇒ **This axis is empty.** There are no runtime options and no conditional
compilation, so there is no option cross-product to enumerate. Every call takes
the same single code path.

### Axis 2 — public entry points (full set, including the lowest level)

| entry point | level | in table below |
|-------------|-------|----------------|
| `md5_digest(const tflac_md5 *m, tflac_u8 out[16])` | lowest level; there is no wrapper above it and no helper below it | yes, all rows |

⇒ The full public API is one function. There are no convenience/one-shot
wrappers to distinguish from lower-level primitives; `md5_digest` *is* the
low-level entry point, and it is called directly through the `.so` in every row.

### Axis 3 — input shapes the code touches

The body reads four `tflac_u32` fields (`a`, `b`, `c`, `d`) and writes 16 bytes.
The shape axes that actually exist:

* **which word** — 4 distinct source words, each mapped to a fixed 4-byte output
  window (`a`→0..4, `b`→4..8, `c`→8..12, `d`→12..16). A per-word test is what
  catches a swapped-field or wrong-offset translation.
* **byte position within a word** — 4 distinct shift amounts (`0, 8, 16, 24`),
  i.e. little-endian serialization. A per-byte-lane test is what catches an
  endianness or shift-amount error.
* **word value** — full `uint32_t` domain: zero, all-ones, one-hot bits,
  single-byte-hot patterns, boundary values, random values.
* **struct image** — the 16-byte memory image of `tflac_md5` (exercises
  `#[repr(C)]` layout: size 16, align 4, field offsets 0/4/8/12).
* **output buffer placement** — alignment/offset of `out`, and whether `out`
  overlaps `m`.

## Configuration-surface table

One row per meaningful combination the C treats differently (cross-product of
the axes above, pruned to distinguishable cases). Every row is executed against
BOTH the C `.so` and the Rust `.so` via `libloading` and compared byte-for-byte;
rows marked "randomized" use many property-style inputs from a fixed-seed PRNG.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `md5_digest` | no options exist; all four words zero (`0,0,0,0`) — minimal image | [x] |
| 2 | `md5_digest` | all four words `0xFFFFFFFF` — maximal image | [x] |
| 3 | `md5_digest` | word `a` varied, `b=c=d=0` — isolates field `a` → out[0..4] | [x] |
| 4 | `md5_digest` | word `b` varied, `a=c=d=0` — isolates field `b` → out[4..8] | [x] |
| 5 | `md5_digest` | word `c` varied, `a=b=d=0` — isolates field `c` → out[8..12] | [x] |
| 6 | `md5_digest` | word `d` varied, `a=b=c=0` — isolates field `d` → out[12..16] | [x] |
| 7 | `md5_digest` | one-hot: exactly one of the 128 struct bits set, all 128 positions — pins every (word, shift) pair | [x] |
| 8 | `md5_digest` | byte-lane hot: each word set to `0x000000FF`, `0x0000FF00`, `0x00FF0000`, `0xFF000000` in turn — pins the 4 shift amounts per word | [x] |
| 9 | `md5_digest` | distinct-per-lane sentinel image (`a=0x03020100 … d=0x0F0E0D0C`) — a swapped field or lane shows up immediately | [x] |
| 10 | `md5_digest` | boundary word values: `0`, `1`, `0x7F`, `0x80`, `0xFF`, `0x100`, `0x7FFF`, `0x8000`, `0xFFFF`, `0x10000`, `0x7FFFFFFF`, `0x80000000`, `0xFFFFFFFE`, `0xFFFFFFFF`, full cross-product over the 4 fields | [x] |
| 11 | `md5_digest` | randomized: uniform random 128-bit struct images, fixed seed, many iterations | [x] |
| 12 | `md5_digest` | randomized struct image built from a raw 16-byte buffer transmuted to `tflac_md5` — validates `#[repr(C)]` size 16 / align 4 / offsets 0,4,8,12 | [x] |
| 13 | `md5_digest` | `out` at every byte offset 0..8 of a larger arena (aligned and unaligned destinations), randomized words | [x] |
| 14 | `md5_digest` | repeated calls reusing the same `out` buffer with different `m` — checks there is no hidden state / no accumulation (the C is stateless) | [x] |
| 15 | `md5_digest` | same `m` called twice — idempotence / purity, byte-identical both times, in both libs | [x] |
| 16 | `md5_digest` | `out` pointing into a pre-poisoned buffer (`0xAA` fill) — confirms all 16 bytes are written, none left stale | [x] |
| 17 | `md5_digest` | `out` aliasing the struct storage: (a) exactly (`out == (u8*)m`), (b) partially overlapping at +1..+15, (c) reverse overlap at -1..-15; randomized. `tflac_u8*` is exempt from strict aliasing, so each store can feed the next load — this row pins the per-byte reload cascade. Case (a) alone is a fixed point and proves nothing; (b)/(c) are the discriminating cases | [x] |
| 18 | `md5_digest` | `m` read through a misaligned pointer, randomized words | [x] |

**Row count: 18.** No feature-flag axis multiplies this table (see below).

## Feature-combination axis

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so the only build configuration is the default one. Enumerated
mechanically by `scripts/check_features.sh`, which parses `Cargo.toml`, finds
zero declared features, and therefore runs the single combination
(`--no-default-features` and default are equivalent here). The C side likewise
has no `#ifdef`s, so there is no matching C build variant.

## Divergence found and fixed by this table

Row 17 exposed the one real translation bug. The C body is sixteen separate
statements, each of which re-reads `m->a` / `m->b` / … before storing one byte.
Because `out` is `tflac_u8 *` (i.e. `unsigned char *`), it is exempt from the
strict-aliasing rule, so a store through `out` can legally modify `*m` and the
compiler may not cache the word across the stores. When `out` partially overlaps
`m`, each store feeds the next load and the output is a byte-by-byte cascade.

The original Rust read each word once and wrote it with a 4-byte
`copy_from_slice`, which produced different bytes under partial overlap:

```
image:  2c 8b b9 c8 c1 9d 61 74 26 43 85 ce f6 16 e9 7a   (out = m + 1)
C   :   2c 2c 2c 2c 2c 2c 2c 2c 2c 2c 2c 2c 2c 2c 2c 2c 2c
Rust:   2c 2c 8b b9 c8 c8 9d 61 74 74 43 85 ce ce 16 e9 7a
```

Fixed in `src/lib.rs` by re-loading the source word immediately before each of
the sixteen stores, using volatile byte-granularity loads and volatile stores so
the optimizer cannot re-cache the loads, widen the stores, or reorder them.

Verified robust against the C compiler's own freedom here: the Rust matches the
C compiled at `-O0`, `-O1`, `-O2`, `-O3` and `-Os` (each built to a temp
directory; `c_src/` untouched), so the per-byte reload is the standard-mandated
semantics rather than an unoptimized-build artifact.

Note that FULL aliasing (`out == (u8*)m`) is a fixed point and cannot detect
this bug — the partial-overlap sub-cases 17b/17c are the discriminating ones.

## Verification status

All 18 rows checked off. Run:

```
cd translation && cargo build --release && cargo test --release
# or the full Phase D driver (feature powerset + symbol diff + tests):
cd translation && ./scripts/verify_all.sh
```

Result: 16/16 valid-path tests pass, 10/10 error-path tests pass (+6 death-test
payloads invoked as children), across the single existing feature combination.

The suite is non-vacuous: it was mutation-tested. Swapping two struct fields is
caught by 13 Phase-B rows and 6 Phase-C rows; reversing the intra-word byte
order is caught by 14 Phase-B and 7 Phase-C tests; reinstating the original
non-reloading implementation is caught by the overlap rows.
