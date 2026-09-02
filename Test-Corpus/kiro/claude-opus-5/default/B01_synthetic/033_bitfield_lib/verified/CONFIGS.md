# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

## How this table was derived

The C library has no runtime option struct, no mode flag, no `setopt`-style
call, and no `#ifdef` other than the header's include guard (verified by the
grep recorded in `ERRORS.md`). Therefore the configuration axes are entirely
**the shape and value class of the arguments**, plus **which entry point** is
used. Both are read off the source and the compiled code, not guessed.

### Axis 1 — entry point (the full public set, lowest level first)

| entry point | signature | level |
|-------------|-----------|-------|
| `print_foo` | `void print_foo(const foo_t *)` | **low-level**: takes the raw 8-byte struct; the only way to reach padding bits and arbitrary bit-field encodings |
| `driver`    | `void driver(unsigned int, unsigned int, bool, int)` | convenience wrapper: packs 4 scalars into a `foo_t`, then calls `print_foo` |

`driver` calls `print_foo` **through the PLT**, so the composed pipeline
(pack → decode → format) is a distinct code path from calling `print_foo`
directly, and is tested as such.

### Axis 2 — `foo_t` field encodings the C distinguishes (from `objdump`)

| field | storage | C decode | distinct shapes |
|-------|---------|----------|-----------------|
| `x` | byte 0, bits 0–1 | `b0 & 3` | in-range 0..3; out-of-range `>3`; `UINT_MAX` |
| `y` | byte 0, bits 2–4 | `(b0 >> 2) & 7` | in-range 0..7; out-of-range `>7`; `UINT_MAX` |
| `b` | byte 0, bit 5 | `(b0 >> 5) & 1` | canonical 0/1; non-canonical byte 2..255; dirty upper argument bits |
| pad | byte 0 bits 6–7, bytes 1–3 | never read | all-zero; all-ones; random garbage |
| `z` | bytes 4–7 | `*(int *)(p + 4)`, unmasked | 0; positive; negative; `INT_MIN`; `INT_MAX`; random 32-bit |

### Axis 3 — format-conversion classes in `printf("%u %u %d %d\n", ...)`

`x` and `y` go through `%u` (unsigned), while `b` and `z` go through `%d`
(signed). Because `z` is the only field that can be negative, the signed/unsigned
split is a real behavioural axis: it is exercised by rows with negative `z`.

### Axis 4 — call sequencing / stream state

`printf` writes to the shared, buffered `stdout`. Repeated and interleaved calls
are a distinct shape from a single call, because output ordering and buffer
flushing must match. Covered by rows 15–17.

## The table

One row per combination the C actually treats differently. Every row is driven
with **many randomized inputs from a fixed-seed PRNG** (seed and count noted per
row) unless the row is exhaustive by construction.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | all in-range: `x` ∈ 0..=3 × `y` ∈ 0..=7 × `b` ∈ {0,1} × `z` = 0 — exhaustive cross-product (64 cases) | [x] |
| 2 | `driver` | all in-range `x`,`y`,`b` as row 1 × randomized 32-bit `z` (seeded, 4096 cases) | [x] |
| 3 | `driver` | `x` out of range (`4..=UINT_MAX`, randomized), `y`/`b`/`z` in range | [x] |
| 4 | `driver` | `y` out of range (`8..=UINT_MAX`, randomized), `x`/`b`/`z` in range | [x] |
| 5 | `driver` | `b` non-canonical byte `2..=255` (exhaustive over the 254 values) × randomized `x`,`y`,`z` | [x] |
| 6 | `driver` | `z` boundary set {`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`} × randomized `x`,`y`,`b` | [x] |
| 7 | `driver` | fully randomized: `x`,`y` uniform over all of `u32`, `b` uniform over all of `u8`, `z` uniform over all of `i32` (seeded, 20000 cases) — the interaction row | [x] |
| 8 | `driver` | boundary cross-product: `x` ∈ {0,3,4,UINT_MAX} × `y` ∈ {0,7,8,UINT_MAX} × `b` ∈ {0,1,2,0xFF} × `z` ∈ {INT_MIN,-1,0,INT_MAX} (256 cases) | [x] |
| 9 | `print_foo` | byte 0 exhaustive `0..=255` (covers every `x`/`y`/`b` encoding incl. padding bits 6–7 set), padding bytes 1–3 zero, `z` = 0 | [x] |
| 10 | `print_foo` | byte 0 exhaustive `0..=255` × padding bytes 1–3 = `0xFF` (garbage padding must not change output) | [x] |
| 11 | `print_foo` | byte 0 randomized × padding bytes 1–3 randomized × `z` randomized (seeded, 20000 cases) — the low-level interaction row | [x] |
| 12 | `print_foo` | `z` boundary set {`INT_MIN`, `-1`, `0`, `INT_MAX`} × byte 0 ∈ {0x00, 0x3F, 0xFF} | [x] |
| 13 | `print_foo` | struct passed as a **misaligned** pointer (offset 1 within an over-aligned buffer), randomized contents | [x] |
| 14 | `driver` → `print_foo` | pipeline equivalence: for the same logical values, `driver(x,y,b,z)` output must equal `print_foo(&packed)` output, in **both** libraries and cross-library (seeded, 8192 cases) | [x] |
| 15 | `driver` | repeated calls in one buffered-stdout session (100 back-to-back calls, randomized) — output concatenation and ordering | [x] |
| 16 | `driver` + `print_foo` | interleaved calls to both entry points in one session (randomized order, seeded) | [x] |
| 17 | `driver` (C) vs `driver` (Rust) | both libraries loaded simultaneously in one process, calls interleaved against the same `stdout` — verifies no global-state or buffering divergence | [x] |
| 18 | `driver` | `b` passed with dirty upper 24 bits in the argument register (whole `u32` written where the ABI expects a byte) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
buildable configurations are the default one and `--no-default-features`, which
are byte-identical. Both are exercised (see `run_all_features.sh`). There are no
`#[cfg(feature = ...)]` sites in `src/` (verified by grep), so no row above has a
feature-dependent variant.

## Test mapping

Each row N is covered by `tests/phase_b_valid_paths.rs::rowNN_*`
(`row01_driver_inrange_exhaustive_z_zero` … `row18_bool_dirty_upper_argument_bits`),
18 tests for 18 rows. Randomized rows use the `SplitMix64` PRNG in
`tests/common/mod.rs` seeded from a per-row constant derived from
`SEED = 0x5EED_1234_ABCD_0001`, so every failure reproduces exactly.

## Divergence found and fixed by Phase B

Row 13 (`print_foo` via a misaligned pointer) initially **failed**: the Rust
`print_foo` formed a `&foo_t` reference, which aborts the process under Rust's
misaligned-reference check, whereas gcc compiles the `z` access to a plain
`mov 0x4(%rax),%esi` that loads the unaligned value and prints it. Fixed in
`src/lib.rs` by reading byte 0 and the 4 bytes of `z` through
`core::ptr::read` / `core::ptr::read_unaligned` instead of constructing a
reference, which also matches gcc's choice of exactly which bytes to touch.
