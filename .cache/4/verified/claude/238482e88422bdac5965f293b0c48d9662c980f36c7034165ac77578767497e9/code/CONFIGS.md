# CONFIGS.md — Phase A: configuration-surface table

## Mechanical derivation of the axes

The C library is one straight-line function, so the axes are derived from what
the emitted code actually distinguishes rather than from `if`/`switch` branches
(there are none). Evidence used:

* `c_src/include/lib.h` — the complete public API: **one** entry point,
  `void md5_digest(const tflac_md5 *m, tflac_u8 out[16])`. There is no
  init/update/finish pipeline, no context object with options, no flags, no
  runtime mode, and no convenience-vs-low-level split: `md5_digest` *is* the
  lowest-level entry point and the only one.
* `c_src/CMakeLists.txt` — no `add_definitions`, no `target_compile_definitions`,
  no `option()`, no `CMAKE_BUILD_TYPE` (so `-O0`); therefore **no build-time
  configuration axis** on the C side.
* `Cargo.toml` — no `[features]` table; therefore exactly **one** Rust feature
  combination exists (the empty set). See `VERIFICATION.md` Phase D.
* `objdump -d` of the built C `.so` — 16 × (4-byte field load → shift → 1-byte
  store), field re-loaded before every store, stores strictly in index order
  `out[0] .. out[15]`.

Axes that the code therefore distinguishes:

| axis | values the C treats differently |
|------|--------------------------------|
| A1 output byte lane | 16 distinct (field, shift) pairs → index mapping `out[4*i + j] = field_i >> 8*j` |
| A2 field value | full `u32` range; per-byte truncation makes each of the 4 lanes of a word independently observable |
| A3 field identity | `a`/`b`/`c`/`d` at struct offsets 0/4/8/12 — order-sensitive |
| A4 `m` alignment | 4-byte load from an arbitrary address; offsets 0,1,2,3 (mod 4) |
| A5 `out` alignment | byte stores; any address |
| A6 `m`/`out` storage class | stack, heap, static/`.data`, `mmap` |
| A7 `m`/`out` overlap | disjoint, fully aliased (`out == (u8*)m`), partial overlap at every offset ±1..±15 |
| A8 exactness of the touched range | exactly bytes `out[0..16]` written, exactly bytes `m[0..16]` read (guard pages / guard bytes) |
| A9 call repetition & reentrancy | repeated calls, interleaved distinct inputs, concurrent calls (function is stateless — no globals exist in either `.so`) |

Rows below are the pruned cross-product of these axes. Every row is driven
through **both** `.so` files via `libloading` and compared byte-for-byte, with
many seeded-random inputs per row (seed printed by the test).

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `md5_digest` | all-zero state; `m` 4-byte aligned, `out` 16-byte aligned, disjoint; sentinel guard bytes around `out` | [x] |
| C2 | `md5_digest` | all-ones state (`0xFFFFFFFF` × 4) — every truncation/shift saturated | [x] |
| C3 | `md5_digest` | one-hot byte lanes: for each of the 16 (field, shift) pairs, exactly that lane is `0xFF` and everything else `0x00` — isolates the index mapping A1 (catches any swapped index/shift) | [x] |
| C4 | `md5_digest` | one-hot *bit* states: single bit set, for all 128 bit positions | [x] |
| C5 | `md5_digest` | boundary word values `{0, 1, 0x7F, 0x80, 0xFF, 0x100, 0x7FFF, 0x8000, 0xFFFF, 0x10000, 0x7FFFFFFF, 0x80000000, 0xFFFFFFFE, 0xFFFFFFFF}` — swept in each field independently and in randomized 4-tuples of boundary values | [x] |
| C6 | `md5_digest` | uniformly random states, many iterations, fixed seed (bulk value fuzz over A2/A3) | [x] |
| C7 | `md5_digest` | field-order sensitivity: four pairwise-distinct byte-distinguishable words, plus all 24 permutations of a fixed 4-tuple (A3) | [x] |
| C8 | `md5_digest` | `m` **unaligned**: struct bytes placed at offsets 1,2,3,5,6,7,...,15 inside a byte buffer (A4) × random values | [x] |
| C9 | `md5_digest` | `out` **unaligned**: destination at offsets 0..15 inside a larger buffer, with sentinel guards before/after (A5) × random values | [x] |
| C10 | `md5_digest` | full aliasing / in-place: `out == (tflac_u8 *)m` (A7) × random values, incl. repeated in-place calls (idempotence of the LE store) | [x] |
| C11 | `md5_digest` | partial forward overlap: `out = (tflac_u8 *)m + k` for `k = 1..15` — later loads observe earlier stores (A7) × random values | [x] |
| C12 | `md5_digest` | partial backward overlap: `out = (tflac_u8 *)m - k` for `k = 1..15` (A7) × random values | [x] |
| C13 | `md5_digest` | write-exactness: `out` positioned so that byte 16 is the first byte of a `PROT_NONE` guard page — proves exactly 16 bytes are written, no 17th (A8) | [x] |
| C14 | `md5_digest` | read-exactness: `m` positioned so that byte 16 is the first byte of a `PROT_NONE` guard page — proves exactly 16 bytes are read, no over-read (A8) | [x] |
| C15 | `md5_digest` | storage-class matrix: `m` in {stack, heap (`Box`), `static`} × `out` in {stack, heap, `static`} — 9 combinations (A6) × random values. The 4th storage class, `mmap`, is covered for both operands by C13/C14, which place `m`/`out` inside an anonymous mapping | [x] |
| C16 | `md5_digest` | statelessness / sequencing: N calls in a row with different random inputs into the *same* `out` buffer, and the same input repeated after other inputs — output must depend only on the current `m` (A9) | [x] |
| C17 | `md5_digest` | reentrancy: 4 threads × many random inputs concurrently against the same loaded symbol (A9) | [x] |
| C18 | `md5_digest` | full random fuzz over the *combined* axes: random values (A2/A3) × random `m` alignment (A4) × random `out` alignment (A5) × random overlap mode ∈ {disjoint-before, disjoint-after, fully aliased, any partial overlap ±15} (A7), 20 000 iterations, fixed seed — the axis cross-product that the per-row tests only cover one dimension at a time | [x] |

18 rows. All boxes are checked by `tests/valid_paths.rs` +
`tests/error_paths.rs` (C13/C14 live with the guard-page harness in
`tests/error_paths.rs`); see `VERIFICATION.md` for the run log.

## Status

All 18 rows PASS against the C `.so` in both the `dev` and `release` Cargo
profiles (`./verify.sh`). Row-to-test mapping:

| rows | test(s) |
|------|---------|
| C1–C12, C15–C18 | `tests/valid_paths.rs::c01_*` … `c18_combined_axis_fuzz` |
| C13, C14 | `tests/error_paths.rs::c13_write_exactness_against_guard_pages`, `c14_read_exactness_against_guard_pages` (need the guard-page/fork harness) |

Feature axis: `Cargo.toml` declares no `[features]`, so the power set is the
single empty combination; `verify.sh` enumerates it from `Cargo.toml` and runs
every row under it, in both profiles.
