# CONFIGS.md — Phase A: configuration-surface table

The mirror of `ERRORS.md`: every **valid** input configuration the C actually
distinguishes.

## Axes derived from the C source (not guessed)

`md5_digest` has no options, flags, modes, or `#ifdef`s. Grepping the header and
source for branches yields nothing (see `ERRORS.md`). So the configuration
surface is **not** made of option toggles — it is made of the two things the code
does branch-free work over, plus the pointer geometry the signature permits:

**Axis 1 — source field.** Four distinct source offsets, hard-coded:
`a`@0, `b`@4, `c`@8, `d`@12. A wrong offset in the port only shows up if the
four fields hold *different* values.

**Axis 2 — shift / truncation.** Each field is emitted with shifts
`0, 8, 16, 24` and truncated by `(tflac_u8)`. A wrong shift or a signed shift
only shows up for values whose bytes differ and whose high bit is set.

**Axis 3 — value shape** of each `tflac_u32`. The cast `(tflac_u8)(m->x >> k)`
is value-dependent, so these shapes are distinguished:
`0x00000000`, `0xFFFFFFFF`, high-bit-set (`0x80000000`, sign-extension trap),
single-byte-isolated (`0x000000FF`, `0x0000FF00`, `0x00FF0000`, `0xFF000000`,
each pins one shift), byte-distinct ascending, and uniform random.

**Axis 4 — pointer geometry.** The signature is
`(const tflac_md5 *, tflac_u8 *)` with **no `restrict`**, so aliasing is legal
input, and neither pointer is required by the ABI to be well-aligned in practice
on x86-64. The C compiler therefore **reloads the source field before every
single byte store** (verified in the disassembly at both `-O0` and `-O2`), which
makes overlapping buffers *defined and observable*. Each overlap displacement is
a genuinely different code path through the data.

**Axis 5 — write extent.** Exactly 16 bytes, no length parameter: the port must
write all 16 and never a 17th, and never read a 17th source byte.

## Table

One row per combination the C treats differently. All rows are driven through
the `.so` exports of **both** implementations and compared byte-for-byte. Rows
marked *randomized* run many property-style iterations with a fixed seed
(`SplitMix64`, seed `0x243F6A8885A308D3`) so they cover value-dependent paths
rather than one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | `md5_digest` | disjoint buffers, both aligned; `a=b=c=d=0` | [x] |
| C2 | `md5_digest` | disjoint, aligned; `a=b=c=d=0xFFFFFFFF` | [x] |
| C3 | `md5_digest` | disjoint, aligned; byte-distinct ascending `0x04030201,0x08070605,0x0C0B0A09,0x100F0E0D` (pins all 4 offsets × all 4 shifts at once) | [x] |
| C4 | `md5_digest` | disjoint, aligned; each field `0x80000000` (high bit set — catches signed/arithmetic shift) | [x] |
| C5 | `md5_digest` | disjoint, aligned; single-byte-isolated sweep: for each field f in {a,b,c,d} × each byte k in {0,1,2,3}, only that byte non-zero (16 sub-cases — pins every field/shift pair independently) | [x] |
| C6 | `md5_digest` | disjoint, aligned; **randomized** full-range `u32` × 4 (2000 iters) | [x] |
| C7 | `md5_digest` | disjoint, aligned; **randomized** but each field drawn from a byte-sparse pool (values built from `{0x00,0x01,0x7F,0x80,0xFE,0xFF}` bytes, 2000 iters — boundary bytes in every position) | [x] |
| C8 | `md5_digest` | disjoint; `out` **misaligned** at every offset 0..8 within its allocation, `m` aligned; randomized values | [x] |
| C9 | `md5_digest` | disjoint; `m` **misaligned** at every offset 1..8 (unaligned 32-bit source loads), `out` aligned; randomized values | [x] |
| C10 | `md5_digest` | disjoint; **both** misaligned, independent odd offsets; randomized values | [x] |
| C11 | `md5_digest` | `out == (tflac_u8 *)m` — exact full self-overlap; randomized values | [x] |
| C12 | `md5_digest` | forward partial overlap: `out = (u8*)m + d` for every `d` in 1..=15 (source partly clobbered mid-copy; each `d` is a distinct data path) ; randomized values | [x] |
| C13 | `md5_digest` | backward partial overlap: `out = (u8*)m - d` for every `d` in 1..=15 ; randomized values | [x] |
| C14 | `md5_digest` | overlap at every `d` in 16..=31 and -16..=-31 (adjacent-but-disjoint boundary — must behave as plain disjoint) ; randomized values | [x] |
| C15 | `md5_digest` | `out` immediately followed by guard bytes; assert exactly bytes 0..15 change and byte 16.. untouched (write-extent = 16) | [x] |
| C16 | `md5_digest` | `out` pre-filled with a non-zero sentinel (`0xAA`) and input all-zero — proves all 16 bytes are actually *stored*, not skipped | [x] |
| C17 | `md5_digest` | 16-byte source at the very end of a mapped page followed by `PROT_NONE` guard — proves no 17th source byte is read | [x] |
| C18 | `md5_digest` | 16-byte `out` at the very end of a mapped page followed by `PROT_NONE` guard — proves no 17th byte is written | [x] |
| C19 | `md5_digest` | repeated invocation on the same `out` with different `m` (no hidden state between calls); randomized, 500 iters | [x] |
| C20 | `md5_digest` | `m` and `out` both taken from a heap allocation with randomized alignment AND randomized overlap displacement in -31..=31 (cross-product of axes 4 and 3, 4000 iters) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** — there are zero
cargo features, hence exactly one feature combination (the default, which is
also `--no-default-features`). The C likewise has no `#ifdef` configuration.
Phase D's "repeat B–C for every feature combination" therefore reduces to the
single default combination, but the suite is still executed under
`--no-default-features`, `--all-features`, and both `dev` and `release` profiles
to prove the claim rather than assume it (see `run_all_configs.sh`).
