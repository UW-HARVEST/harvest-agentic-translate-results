# CONFIGS.md — Phase A: Configuration-surface table

## Axes, derived mechanically from the C source

Public API surface (`c_src/include/driver.h`, cross-checked against
`nm -D --defined-only c_src/build/libdriver.so` which lists exactly one symbol):

```c
void driver(const char *s1, const char *s2);
```

`driver` is simultaneously the highest-level *and* the lowest-level public entry point —
there is only one, so "test the low-level entry points, not just the convenience
wrapper" collapses to testing `driver` itself. Its body is
`printf("%zu\n", strcspn(s1, s2))`.

### Axis 1 — runtime options / modes / flags: **none**

`grep -nE '\b(if|switch|else)\b|#if' src/*.c include/*.h` matches only the `DRIVER_H_`
include guard. There is no options struct, no context/handle object, no global or
`static` state, no environment-variable lookup, no `#ifdef` feature toggle, and no
integer/enum/flag parameter. Nothing the caller can set changes the code path. The
only inputs are the *contents* of the two strings, so all remaining axes are input
**shape** axes.

### Axis 2 — shape of `s1` (the scanned string)

`empty` · `1 byte` · `short (2..16)` · `crosses SIMD block boundaries (15/16/17, 31/32/33, 63/64/65 bytes)` · `long (hundreds)` · `very long (≥ 1 MiB)`

### Axis 3 — shape of `s2` (the reject set)

`empty` · `1 byte` · `few bytes` · `many bytes` · `all 255 non-NUL bytes` · `contains duplicates` · `longer than s1`

### Axis 4 — position of the first match (the value the result depends on)

`no match → result = strlen(s1)` · `match at index 0 → result 0` · `match in the middle` · `match at the last byte → result = strlen(s1)-1` · `every byte matches`

### Axis 5 — byte-value domain (the only place C `char` signedness can leak)

`ASCII printable (0x20..0x7E)` · `0x01..0x1F control bytes` · `0x7F` · `0x80..0xFF high bytes (negative as signed char)` · `full 0x01..0xFF sweep` · `mixed ASCII + high`

### Axis 6 — memory placement (over-read sensitivity of the scan loops)

`heap, ordinary` · `NUL flush against an unmapped page boundary` · `misaligned start offsets 0..15`

Result-value axis: the printed text is `%zu` of a `size_t`, so `result = 0`,
single digit, multi-digit, and ≥ 2^20 all exercise different `printf` conversions.

## Configuration table

One row per combination the C code treats differently (cross-product of axes 2–6,
pruned to the distinguishable cases). Every row is driven through **both** `.so`s via
`libloading` and compared byte-for-byte on stdout; every row marked *randomized* uses
many seeded pseudo-random inputs (fixed seed, SplitMix64 in `tests/common/mod.rs`), not
a single hand-picked value. Tests live in `tests/valid_paths.rs`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | `s1 = ""`, `s2 = ""` — both empty (result 0) | [x] |
| 2 | `driver` | `s1 = ""`, `s2` non-empty (result 0; `s2` never dereferenced) | [x] |
| 3 | `driver` | `s1` non-empty, `s2 = ""` — empty reject set ⇒ result = `strlen(s1)`; *randomized* over `s1` length 1..256 and random bytes | [x] |
| 4 | `driver` | `s1` 1 byte, `s2` 1 byte, equal ⇒ 0 / unequal ⇒ 1; *randomized* over all byte pairs incl. high bytes | [x] |
| 5 | `driver` | match at index 0 (first byte of `s1` is in `s2`) ⇒ result 0; *randomized* | [x] |
| 6 | `driver` | match strictly in the middle of `s1`; *randomized* over length and match index | [x] |
| 7 | `driver` | match at the **last** byte of `s1` ⇒ result = `strlen(s1)-1`; *randomized* | [x] |
| 8 | `driver` | **no match at all** with non-empty `s2` ⇒ result = `strlen(s1)`; *randomized* with disjoint byte alphabets | [x] |
| 9 | `driver` | **every** byte of `s1` is in `s2` ⇒ result 0; *randomized* | [x] |
| 10 | `driver` | `s1` length exactly 15/16/17, 31/32/33, 63/64/65 (SIMD block boundaries) × {no match, match at last byte, match at index 0}; *randomized* content | [x] |
| 11 | `driver` | `s2` = all 255 non-NUL bytes (maximal reject set) ⇒ result 0 for any non-empty `s1`; *randomized* `s1` | [x] |
| 12 | `driver` | byte domain = ASCII printable only, both strings; *randomized* | [x] |
| 13 | `driver` | byte domain = high bytes `0x80..0xFF` only (negative as signed `char`) — signedness boundary; *randomized* | [x] |
| 14 | `driver` | byte domain = full `0x01..=0xFF` sweep, both strings; *randomized* | [x] |
| 15 | `driver` | `s2` contains duplicate bytes / is longer than `s1` (no early exit) ; *randomized* | [x] |
| 16 | `driver` | `s1` long (256..4096 bytes) with a match at a random index; *randomized* | [x] |
| 17 | `driver` | `s1` very long (≥ 1 MiB) with no match ⇒ large `%zu` value (multi-digit conversion) | [x] |
| 18 | `driver` | misaligned buffers: `s1` and `s2` started at every offset 0..15 inside an over-allocated block; *randomized* content | [x] |
| 19 | `driver` | `s1` NUL-terminated flush against an unmapped page boundary (over-read detector), match / no match | [x] |
| 20 | `driver` | `s2` NUL-terminated flush against an unmapped page boundary (over-read detector) | [x] |
| 21 | `driver` | result-digit-count sweep: `s1` lengths chosen so the printed value has 1, 2, 3, 4, 5, 6, 7 digits (`%zu` formatting) | [x] |
| 22 | `driver` | repeated / interleaved invocation of C then Rust then C in one process — checks the function is stateless and leaves no residue in stdio state; *randomized* sequence | [x] |
| 23 | `driver` | full uniform random fuzz over both strings (lengths 0..64, bytes `0x01..=0xFF`), 20 000 seeded iterations | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so the feature power-set is
`{ default } = { ∅ }`: `--no-default-features`, `--all-features` and the default build
are the same library. `check_features.sh` enumerates the (empty) feature list from
`cargo metadata` and re-runs the whole suite for each combination, and
`tests/feature_matrix.rs::no_cargo_features_declared` fails if a feature is ever added
without extending this matrix.
