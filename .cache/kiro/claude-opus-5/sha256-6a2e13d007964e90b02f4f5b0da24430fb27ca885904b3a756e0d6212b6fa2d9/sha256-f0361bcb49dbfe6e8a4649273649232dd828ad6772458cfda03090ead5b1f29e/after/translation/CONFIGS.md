# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

The library's whole branch surface, taken from the source (not from docs):

```
$ grep -cE 'if|switch|#if' c_src/src/lib.c c_src/include/lib.h   ->  0, 0
```

There are **no runtime options, modes, or flags** — the public header declares a
single function taking a single struct pointer, and there is no global state, no
setter, no context object, and no compile-time `#ifdef`. So the configuration
surface is driven entirely by the *input shape* carried in `cp_image_t`.

Axes the C code actually distinguishes:

| axis | why the C branches on it | distinct values |
|------|--------------------------|-----------------|
| `h` (parity) | `flips = h / 2` — for odd `h` the middle row is never touched | even, odd |
| `h` (magnitude) | `i < flips` gate | 0, 1, 2, 3, 4, 5, many |
| `w` | `j < w` gate and the row stride `w*i` | 0, 1, 2, many |
| `pix` contents | swap is value-moving; value-dependent bugs only show with varied bytes | randomized per row, all 4 channels distinct |
| buffer slack | reading/writing exactly `w*h` pixels vs. touching beyond | guard bytes appended after `w*h` and checked for both |
| row overlap | `a` and `b` alias when `w*i == w*(h-i-1)`, i.e. impossible for `i < h/2`; adjacent rows (`h == 2,3`) are the closest case | `h` = 2, 3 |

Public entry points: `flip_horizontal` is the *only* one — it is simultaneously
the lowest-level and the highest-level entry point; there is no convenience
wrapper to prefer or to skip. The tests call it directly through `dlsym` on both
`.so`s.

The project builds **no binary/driver executable** (`c_src/CMakeLists.txt` has
only `add_library(... SHARED src/lib.c)`; `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` and has no `[[bin]]` / `src/main.rs`). The
"compare-stdout-of-two-binaries" item is therefore not applicable, and is
asserted structurally by `phase_b_no_binary_target`.

Feature combinations: `translation/Cargo.toml` declares no `[features]` table,
so the only combination is the default (empty) one. Verified by script across
`--no-default-features` and `--all-features` as well.

## Table

Each row is run with **many randomized inputs** (fixed seed `0x2545F4914F6CDD1D`,
xorshift64* PRNG, ≥32 randomized buffers per row unless noted), comparing the
full post-call pixel buffer of C vs. Rust byte-for-byte, plus the guard region
and the `cp_image_t` struct bytes themselves.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `flip_horizontal` | `w=0, h=0` — empty image, no options | [x] |
| 2 | `flip_horizontal` | `w=1, h=0` — zero rows | [x] |
| 3 | `flip_horizontal` | `w=0, h=1` — zero-width single row | [x] |
| 4 | `flip_horizontal` | `w=0, h` random in 2..64 — zero-width, many rows (outer loop runs, inner does not) | [x] |
| 5 | `flip_horizontal` | `w=1, h=1` — single pixel, `flips=0` | [x] |
| 6 | `flip_horizontal` | `w=1, h=2` — minimum image that performs a swap; adjacent-row case | [x] |
| 7 | `flip_horizontal` | `w=1, h=3` — odd `h`, middle row must be preserved | [x] |
| 8 | `flip_horizontal` | `w=2, h=2` — multi-pixel rows, even `h` | [x] |
| 9 | `flip_horizontal` | `w=2, h=3` — multi-pixel rows, odd `h` | [x] |
| 10 | `flip_horizontal` | `w=3, h=4` — even `h`, non-power-of-two width | [x] |
| 11 | `flip_horizontal` | `w=5, h=5` — odd `h`, odd `w` | [x] |
| 12 | `flip_horizontal` | `w` random 1..17, `h` random 1..17 (full small cross-product, property-style) | [x] |
| 13 | `flip_horizontal` | exhaustive cross-product `w` in 0..8 × `h` in 0..8, randomized contents | [x] |
| 14 | `flip_horizontal` | large: `w=64, h=64` randomized | [x] |
| 15 | `flip_horizontal` | tall/thin: `w=1, h=257` (odd, large) | [x] |
| 16 | `flip_horizontal` | wide/flat: `w=257, h=2` | [x] |
| 17 | `flip_horizontal` | `w=1, h` odd large (`h=1023`) — middle row preservation at scale | [x] |
| 18 | `flip_horizontal` | pixel-content shape: all-zero buffer (channel values must not be special-cased) | [x] |
| 19 | `flip_horizontal` | pixel-content shape: all-`0xFF` buffer | [x] |
| 20 | `flip_horizontal` | pixel-content shape: per-channel distinct patterns (r/g/b/a set from independent counters) to catch channel-order and per-field-copy bugs | [x] |
| 21 | `flip_horizontal` | buffer with trailing guard bytes: asserts neither impl writes past `w*h` pixels | [x] |
| 22 | `flip_horizontal` | `w` larger than the actual allocated row stride is NOT tested as valid (would be UB in C); instead: allocation exactly `w*h`, no slack, verifies no over-read | [x] |
| 23 | `flip_horizontal` | struct-mutation check: `img->w`, `img->h`, `img->pix` must be unchanged after the call in both impls | [x] |
| 24 | `flip_horizontal` | idempotence/involution shape: calling twice must restore the original buffer, identically in both | [x] |
| 25 | `flip_horizontal` | negative `w` (`-1`, `-5`, `INT_MIN`) with `h` in {0,1,2,7} — valid `int` inputs the C accepts and no-ops on | [x] |
| 26 | `flip_horizontal` | negative `h` (`-1`, `-2`, `-7`, `INT_MIN`) with `w` in {0,1,4} | [x] |
| 27 | `flip_horizontal` | `h=INT_MAX`/`INT_MIN` and `w=INT_MAX`/`INT_MIN` boundary combinations that provably do no memory access | [x] |
| 28 | `flip_horizontal` | randomized fuzz sweep: 4000 iterations of random `w`,`h` in `-4..=24` with randomized buffers sized for the max, differential on every byte | [x] |

All rows checked: **28/28** — `cargo test --test phase_b_valid` → 30 passed
(28 rows + the no-binary assertion + the two-distinct-libraries assertion),
0 failed, under both the release and the debug artifact.

## Note on allocation sizing

`pixel_count` allocates the full `w * h` pixels only when `w >= 1 && h >= 2` —
the only shapes where the C dereferences anything (the inner loop is gated on
`j < w`, the outer on `i < h / 2`). Every other shape gets a one-pixel buffer,
which is what makes `w == INT_MAX` testable without an 8 GiB allocation while
still driving the real code path. This was found the hard way: the first run of
row 27 aborted with `memory allocation of 8589934620 bytes failed`.

## Rows that would be added if the C changed

`phase_b_no_binary_target` and `phase_c_row15_no_enum_surface_and_arbitrary_ints`
assert structurally that no binary target and no `enum` exist in the C source, so
the two "not applicable" claims above fail loudly instead of going stale.
