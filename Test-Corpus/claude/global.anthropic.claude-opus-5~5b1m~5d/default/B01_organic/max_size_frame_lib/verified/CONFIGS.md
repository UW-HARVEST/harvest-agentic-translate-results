# CONFIGS.md — Configuration-surface table (valid inputs)

## Axes derived from the C source

`c_src/src/lib.c` has no `if`/`switch`/`#ifdef` (verified: `grep -cE '\bif\b|\bswitch\b'` → 0).
All branching is *arithmetic*, via 0/1 multipliers. The predicates the code
actually distinguishes are therefore exactly:

| axis | predicate in C | effect |
|------|----------------|--------|
| **A** — channel mode | `channels == 2` (line 6,7) vs `channels != 2` (line 5) | selects the stereo path (`term2 + term3`) or the independent-channel path (`term1`) |
| **A′** — channel count shape | `channels * (channels != 2)`: `0`, `1`, `2`, `3`, many, `≥2^31`, `u32::MAX` | `channels == 0` zeroes `term1` *and* the `+ channels` outer term; also appears un-gated as `18U + channels` |
| **B** — bitdepth mode | `bitdepth != 32` (line 7) | adds `+1` to the bitdepth used by `term3` — **only observable when `channels == 2`** |
| **B′** — bitdepth shape | `0`, `1`, `8`, `16`, `24`, `31`, `32`, `33`, `≥2^31`, `u32::MAX` | scales `term1`/`term2`/`term3` |
| **C** — blocksize shape | `0`, `1`, small, typical (`4096`), `65535`, `65536`, `≥2^31`, `u32::MAX` | scales all three terms |
| **D** — wrapping | products exceed `2^32` | C unsigned arithmetic wraps mod 2^32; the `+7` and `/8` then act on the wrapped value |
| **E** — rounding | `(sum + 7) / 8` | ceil-to-bytes; residue of `sum mod 8` is the interesting sub-axis |

Public entry points: exactly one, `max_size_frame` (the only symbol in
`nm -D`; there is no lower-level/internal API, no state object, no options
struct, no init/reset call, and no driver binary — `CMakeLists.txt` builds only
`SHARED src/lib.c`, and `Cargo.toml` has no `[[bin]]` and no `[features]`).
So there is a single feature combination: the default (== `--no-default-features`).

## Configuration rows

Every row is driven through **both** `.so` exports with **many seeded-random
inputs** on the axes left free by that row (seed fixed at `0x5EED_C0DE`,
xorshift64* PRNG, 2000 iterations per row unless noted), asserting the returned
`u32` matches byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `max_size_frame` | A: `channels == 2` (stereo path) × B: `bitdepth == 32` × C: `blocksize` random full-range | [x] |
| 2 | `max_size_frame` | A: `channels == 2` × B: `bitdepth != 32`, random full-range × C: `blocksize` random full-range | [x] |
| 3 | `max_size_frame` | A: `channels == 2` × B′: `bitdepth ∈ {0,1,8,16,24,31,33}` (near-32 boundary) × C: `blocksize` random | [x] |
| 4 | `max_size_frame` | A: `channels == 2` × B: `bitdepth == 32` × C: `blocksize ∈ {0,1,16,4096,65535,65536}` (audio-realistic + boundary) | [x] |
| 5 | `max_size_frame` | A′: `channels == 0` (zeroes `term1` and the `+channels` term) × B/B′ random × C random | [x] |
| 6 | `max_size_frame` | A′: `channels == 1` (mono) × B′ random incl. 32 × C random | [x] |
| 7 | `max_size_frame` | A′: `channels == 3` (first count past the stereo special case) × B′ random × C random | [x] |
| 8 | `max_size_frame` | A′: `channels ∈ [4,8]` (multichannel, FLAC-legal) × B′ ∈ {8,16,24,32} × C ∈ [1,65535] — realistic consumer usage | [x] |
| 9 | `max_size_frame` | A′: `channels` random full-range `u32` (incl. `≥2^31`) × B′ random × C random — non-2 path with wrapping | [x] |
| 10 | `max_size_frame` | A: `channels == 2` × B′ `bitdepth ≥ 2^31` (high-bit set) × C random — stereo path with wrapping | [x] |
| 11 | `max_size_frame` | C: `blocksize == 0` × A/A′ random × B′ random — all terms vanish, result is `18 + channels + 0` | [x] |
| 12 | `max_size_frame` | C: `blocksize == 1` × A/A′ random × B′ random | [x] |
| 13 | `max_size_frame` | C: `blocksize == u32::MAX` × A/A′ random × B′ random | [x] |
| 14 | `max_size_frame` | D: overflow-targeted — `blocksize`/`bitdepth`/`channels` drawn from powers of two so products land exactly on / just past `2^32` multiples (both `channels==2` and `channels!=2`) | [x] |
| 15 | `max_size_frame` | E: rounding-targeted — inputs chosen so `sum mod 8` sweeps `0..7` for both channel paths | [x] |
| 16 | `max_size_frame` | A′: `channels == u32::MAX` (outer `18U + channels` wraps past 2^32) × B′/C random | [x] |
| 17 | `max_size_frame` | Fully unconstrained: all three args uniform random `u32` — 200 000 iterations, seeded | [x] |
| 18 | `max_size_frame` | Exhaustive dense cube: `blocksize, channels, bitdepth ∈ [0,40]^3` (68 921 triples) — every small-value interaction | [x] |
| 19 | `max_size_frame` | Structured sweep: `channels ∈ [0,10]` × `bitdepth ∈ [0,40]` × `blocksize ∈ {0,1,7,8,9,4095,4096,65535,2^31,2^32-1}` | [x] |
| 20 | `max_size_frame` | Repeat-call determinism / no hidden state: each row's first input replayed 3× and compared against the C `.so` each time (the C function is pure; a Rust `static mut` regression would show here) | [x] |

All 20 rows checked off — see `translation/tests/differential.rs`
(`config_row_01` … `config_row_20`), all passing against both `.so`s.
