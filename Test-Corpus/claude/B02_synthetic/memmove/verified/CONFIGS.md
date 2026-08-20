# CONFIGS.md — configuration-surface table (Phase A → Phase B)

## Entry points

`c_src/src/lib.c` exports exactly one function, so the *whole* public API is:

```c
size_t process_buffer(uint8_t *buffer, size_t length, uint32_t flags,
                      int param1, int param2);
```

The five lowest-level routines (`rotate_buffer`, `compact_runs`,
`remove_duplicates`, `interleave_halves`, `reverse_segments`) are `static` and
therefore **only** reachable through `process_buffer`'s flag bits.  Each of them
is driven in isolation (single flag bit) *and* inside the composed pipeline
(multi-bit `flags`), because `param1` is shared between three different
operations and the later stages consume the length produced by the earlier ones.

The second entry point is the CLI (`c_src/src/main.c` → `main`), covered by
`tests/driver_cli.rs`.

## Axes the C code actually branches on

| axis | values the source distinguishes |
|------|--------------------------------|
| `flags` bit 0 `0x01` | rotate (`rotate_buffer`) |
| `flags` bit 1 `0x02` | compact runs (`compact_runs`) — the only operation that can **change** the length upward |
| `flags` bit 2 `0x04` | remove duplicates (`remove_duplicates`) — can shrink the length |
| `flags` bit 3 `0x08` | interleave halves (`interleave_halves`), gated on `new_len >= 2` |
| `flags` bit 4 `0x10` | reverse segments (`reverse_segments`), gated on `new_len >= 4 && seg_size <= new_len` |
| `flags` bits 5-31 | ignored (no valid meaning) |
| `param1` as rotate offset | `0`; `+k` with `k < len/2` (small-offset `memmove` branch, single chunk); `k == len/2`; `k > len/2` (large-offset branch); `k >= 256` with `k < len/2` (**multi-chunk** loop, needs `len > 512`); `k >= len` (folded by `%`); negative (normalised by `+= len`); `INT_MIN`; `INT_MAX` |
| `param1` as compact threshold | `1` (**grows** the buffer), `2`, `3`, `4…254`, `255`, and the out-of-range values `<=0` / `>255` that fall back to `3` |
| `param1` as segment size | `1` (early return), `2`, `3`, `4`, `5…`, `== new_len` (single segment), `> new_len` (skipped), `<= 0` → default `4`; even vs odd `seg_size`; `new_len % seg_size` equal to `0` / `1` / `>1` |
| `param2` | `0` (unordered de-dup, `seen[256]` table + swap-to-front) vs `!= 0` (order-preserving O(n²) path); `1`, `-1`, `INT_MIN`, `INT_MAX` |
| `length` | `0`; `1`; `2`; `3`; `4`; odd vs even; `255`, `256` (the CLI maximum); `257…512`; `513…1024` (`interleave_halves`' `half > 256` in-place branch needs `len >= 514`) |
| data shape | `Random`, `SmallAlphabet` (1-4 distinct values), `Constant` (single huge run), `AllDistinct` (no runs), `ShortRuns` (run lengths 1-6, straddles thresholds), `LongRuns` (run lengths 250-260, straddles the 255 clamp), `Alternating` (all runs length 1), `TwoBlocks` |

`GUARD = 96` filler bytes are appended behind the `2 * length` write window; the
comparison covers the return value **and** every byte of the scratch buffer
(including the bytes past the returned length and the guard area), so stale-data
and over-write differences are caught too.

Each row below is checked with **many randomised inputs** (fixed seeds, see
`tests/valid_paths.rs`), not a single hand-picked value.

## Section F — full flag cross-product (`tests/valid_paths.rs::flag_cross_product`)

All 32 combinations of the five meaningful bits, each with randomised
`param1 ∈ [-300, 300] ∪ {0, 1, 2, 3, 255, 256, INT_MIN, INT_MAX}`,
`param2 ∈ {0, ±1, ±random, INT_MIN, INT_MAX}`, every data shape and every length
in `{1,2,3,4,5,7,8,9,15,16,17,31,32,33,63,64,65,127,128,129,254,255,256}`
(≥ 512 iterations per row).

| #  | entry point | configuration (options set + input shape) | [x] |
|----|-------------|-------------------------------------------|-----|
| F0  | `process_buffer` | `flags=0x00` — pure pass-through, all shapes × all lengths | [x] |
| F1  | `process_buffer` | `flags=0x01` rotate only | [x] |
| F2  | `process_buffer` | `flags=0x02` compact only | [x] |
| F3  | `process_buffer` | `flags=0x03` rotate + compact (shared `param1`!) | [x] |
| F4  | `process_buffer` | `flags=0x04` de-dup only | [x] |
| F5  | `process_buffer` | `flags=0x05` rotate + de-dup | [x] |
| F6  | `process_buffer` | `flags=0x06` compact + de-dup | [x] |
| F7  | `process_buffer` | `flags=0x07` rotate + compact + de-dup | [x] |
| F8  | `process_buffer` | `flags=0x08` interleave only | [x] |
| F9  | `process_buffer` | `flags=0x09` rotate + interleave | [x] |
| F10 | `process_buffer` | `flags=0x0A` compact + interleave (interleave sees the *grown* length) | [x] |
| F11 | `process_buffer` | `flags=0x0B` rotate + compact + interleave | [x] |
| F12 | `process_buffer` | `flags=0x0C` de-dup + interleave (interleave sees the *shrunk* length) | [x] |
| F13 | `process_buffer` | `flags=0x0D` rotate + de-dup + interleave | [x] |
| F14 | `process_buffer` | `flags=0x0E` compact + de-dup + interleave | [x] |
| F15 | `process_buffer` | `flags=0x0F` rotate + compact + de-dup + interleave | [x] |
| F16 | `process_buffer` | `flags=0x10` reverse-segments only | [x] |
| F17 | `process_buffer` | `flags=0x11` rotate + reverse (`param1` is *both* offset and `seg_size`) | [x] |
| F18 | `process_buffer` | `flags=0x12` compact + reverse (`param1` is *both* threshold and `seg_size`) | [x] |
| F19 | `process_buffer` | `flags=0x13` rotate + compact + reverse | [x] |
| F20 | `process_buffer` | `flags=0x14` de-dup + reverse | [x] |
| F21 | `process_buffer` | `flags=0x15` rotate + de-dup + reverse | [x] |
| F22 | `process_buffer` | `flags=0x16` compact + de-dup + reverse | [x] |
| F23 | `process_buffer` | `flags=0x17` rotate + compact + de-dup + reverse | [x] |
| F24 | `process_buffer` | `flags=0x18` interleave + reverse | [x] |
| F25 | `process_buffer` | `flags=0x19` rotate + interleave + reverse | [x] |
| F26 | `process_buffer` | `flags=0x1A` compact + interleave + reverse | [x] |
| F27 | `process_buffer` | `flags=0x1B` rotate + compact + interleave + reverse | [x] |
| F28 | `process_buffer` | `flags=0x1C` de-dup + interleave + reverse | [x] |
| F29 | `process_buffer` | `flags=0x1D` rotate + de-dup + interleave + reverse | [x] |
| F30 | `process_buffer` | `flags=0x1E` compact + de-dup + interleave + reverse | [x] |
| F31 | `process_buffer` | `flags=0x1F` full pipeline, all five operations | [x] |

## Section R — `rotate_buffer` branch matrix (`flags & 0x01`)

| #  | entry point | configuration | [x] |
|----|-------------|---------------|-----|
| R1 | `process_buffer` | `flags=0x01`, `param1 == 0` → `offset == 0`, rotate skipped, all lengths/shapes | [x] |
| R2 | `process_buffer` | `flags=0x01`, `param1 == ±k*length` → `offset == 0`, rotate skipped | [x] |
| R3 | `process_buffer` | `flags=0x01`, `1 <= param1 < length/2` → small-offset branch, `chunk == offset`, single loop iteration | [x] |
| R4 | `process_buffer` | `flags=0x01`, `param1 == length/2` (even length) → large-offset branch boundary | [x] |
| R5 | `process_buffer` | `flags=0x01`, `length/2 < param1 < length` → large-offset branch | [x] |
| R6 | `process_buffer` | `flags=0x01`, `param1 == length-1` / `param1 == 1` → extreme offsets | [x] |
| R7 | `process_buffer` | `flags=0x01`, `param1 > length` (folded by `%`), incl. `INT_MAX` | [x] |
| R8 | `process_buffer` | `flags=0x01`, `param1 < 0` (`-1`, `-(length-1)`, `-length-3`, `INT_MIN`) → `offset += len` normalisation | [x] |
| R9 | `process_buffer` | `flags=0x01`, `length == 1` → `rotate_buffer` never called (`param1 % 1 == 0`) | [x] |
| R10 | `process_buffer` | `flags=0x01`, `length == 2,3` → `len <= 1` guard just missed, minimal rotations | [x] |
| R11 | `process_buffer` | `flags=0x01`, `length ∈ [514, 3000]` with `256 <= offset < length/2` → **multi-chunk** small-offset loop; the test asserts the `i += chunk` loop really ran exactly 1, 2, 3, 4 *and* 5 times somewhere in the sweep | [x] |
| R12 | `process_buffer` | `flags=0x01`, `length ∈ [257, 512]`, offsets on both sides of `length/2` (large branch keeps `len-offset <= 256`) | [x] |

## Section C — `compact_runs` branch matrix (`flags & 0x02`)

| #  | entry point | configuration | [x] |
|----|-------------|---------------|-----|
| C1 | `process_buffer` | `flags=0x02`, `param1 == 1` → `threshold 1`, every run compacted, length **grows** (up to `2*length`); all shapes × all lengths | [x] |
| C2 | `process_buffer` | `flags=0x02`, `param1 == 2` → `threshold 2` (length can only shrink or stay) | [x] |
| C3 | `process_buffer` | `flags=0x02`, `param1 == 3` → `threshold 3` (explicit) | [x] |
| C4 | `process_buffer` | `flags=0x02`, `param1 ∈ [4, 254]` random → mid thresholds vs `ShortRuns`/`SmallAlphabet` | [x] |
| C5 | `process_buffer` | `flags=0x02`, `param1 == 255` → threshold 255, only ≥255-runs compact (needs `LongRuns`, `length >= 255`) | [x] |
| C6 | `process_buffer` | `flags=0x02`, `param1 <= 0` (`0`, `-1`, `INT_MIN`) → default threshold `3` | [x] |
| C7 | `process_buffer` | `flags=0x02`, `param1 > 255` (`256`, `1000`, `INT_MAX`) → default threshold `3` | [x] |
| C8 | `process_buffer` | `flags=0x02`, `Constant` shape with `length > 255` → `run_len > 255` clamp, remainder re-scanned | [x] |
| C9 | `process_buffer` | `flags=0x02`, `LongRuns` shape, `length ∈ [256, 1024]` → repeated clamping, mixed keep/compact | [x] |
| C10 | `process_buffer` | `flags=0x02`, final run ends exactly at `len` → tail `memmove` skipped | [x] |
| C11 | `process_buffer` | `flags=0x02`, `length == 1` (single byte: compacts iff `threshold == 1`) | [x] |
| C12 | `process_buffer` | `flags=0x02`, `param1 == 1`, all shapes × `length ∈ [1, 1024]` → asserts `new_len <= 2*length` *and* that `new_len == 2*length` is actually reached, i.e. the last byte of the write window `src/ffi.rs::view_len` hands out is exercised | [x] |

## Section D — `remove_duplicates` branch matrix (`flags & 0x04`)

| #  | entry point | configuration | [x] |
|----|-------------|---------------|-----|
| D1 | `process_buffer` | `flags=0x04`, `param2 == 0` → unordered path (`seen[256]`, swap-to-front), all shapes × all lengths | [x] |
| D2 | `process_buffer` | `flags=0x04`, `param2 == 1` → order-preserving path, all shapes × all lengths | [x] |
| D3 | `process_buffer` | `flags=0x04`, `param2 ∈ {-1, INT_MIN, INT_MAX, random≠0}` → order-preserving path | [x] |
| D4 | `process_buffer` | `flags=0x04`, `length == 1` → `len <= 1` early return | [x] |
| D5 | `process_buffer` | `flags=0x04`, `AllDistinct` with `length >= 256` → every value present, `write == i` throughout | [x] |
| D6 | `process_buffer` | `flags=0x04`, `Constant` → collapses to length 1 | [x] |

## Section I — `interleave_halves` branch matrix (`flags & 0x08`)

| #  | entry point | configuration | [x] |
|----|-------------|---------------|-----|
| I1 | `process_buffer` | `flags=0x08`, even `length`, `length/2 <= 256` → temp-buffer branch, no odd fix-up | [x] |
| I2 | `process_buffer` | `flags=0x08`, odd `length`, `length/2 <= 256` → temp-buffer branch **plus** `buf[len-1] = buf[half]` | [x] |
| I3 | `process_buffer` | `flags=0x08`, `length == 2` / `3` → smallest accepted sizes | [x] |
| I4 | `process_buffer` | `flags=0x08`, `length == 512` → `half == 256`, boundary of the temp branch | [x] |
| I5 | `process_buffer` | `flags=0x08`, `length ∈ [514, 1024]` → `half > 256`, **in-place** branch (`dst < src` true for `i < half-1`, false for `i == half-1`) | [x] |
| I6 | `process_buffer` | `flags=0x0A`, `param1 == 1`, `length ∈ [258, 512]` → compact grows `new_len` past `513`, so the *in-place* interleave branch is reached through the pipeline | [x] |

## Section V — `reverse_segments` branch matrix (`flags & 0x10`)

| #  | entry point | configuration | [x] |
|----|-------------|---------------|-----|
| V1 | `process_buffer` | `flags=0x10`, `param1 == 1` → `seg_size <= 1` early return | [x] |
| V2 | `process_buffer` | `flags=0x10`, `param1 == 2` (even seg) | [x] |
| V3 | `process_buffer` | `flags=0x10`, `param1 == 3` (odd seg, middle element untouched) | [x] |
| V4 | `process_buffer` | `flags=0x10`, `param1 <= 0` → default `seg_size == 4` | [x] |
| V5 | `process_buffer` | `flags=0x10`, `seg_size` divides `length` exactly (`remainder == 0`) | [x] |
| V6 | `process_buffer` | `flags=0x10`, `length % seg_size == 1` → remainder left un-reversed | [x] |
| V7 | `process_buffer` | `flags=0x10`, `length % seg_size > 1` → remainder reversed separately | [x] |
| V8 | `process_buffer` | `flags=0x10`, `seg_size == length` → single segment (full reverse) | [x] |
| V9 | `process_buffer` | `flags=0x10`, `seg_size == length-1` → 1 segment + remainder 1 | [x] |
| V10 | `process_buffer` | `flags=0x10`, `seg_size > length` → skipped by the `seg_size <= new_len` guard | [x] |
| V11 | `process_buffer` | `flags=0x10`, `length ∈ {1,2,3}` → skipped by the `new_len >= 4` guard | [x] |
| V12 | `process_buffer` | `flags=0x10`, random `seg_size ∈ [2, 300]` × `length ∈ [4, 1024]` | [x] |

## Section X — pipeline interactions (length changes between stages)

| #  | entry point | configuration | [x] |
|----|-------------|---------------|-----|
| X1 | `process_buffer` | `flags=0x06`, `param1 == 1` → compact grows, then de-dup shrinks (both orders of magnitude of `new_len`) | [x] |
| X2 | `process_buffer` | `flags=0x0C`, `Constant`/`SmallAlphabet` → de-dup shrinks `new_len` to `1`, interleave skipped by the `>= 2` guard | [x] |
| X3 | `process_buffer` | `flags=0x14`, de-dup shrinks `new_len` below `4` → reverse skipped | [x] |
| X4 | `process_buffer` | `flags=0x1C`, de-dup shrinks to `2`/`3` → interleave runs, reverse skipped | [x] |
| X5 | `process_buffer` | `flags=0x12`, `param1 == 2` → threshold `2` *and* `seg_size 2` from the same `param1` | [x] |
| X6 | `process_buffer` | `flags=0x13`, `param1 ∈ [2, 40]` → same `param1` used as offset, threshold and `seg_size` | [x] |
| X7 | `process_buffer` | `flags=0x1F`, `param1 == 1` → offset 1, threshold 1 (growth), `seg_size 1` (reverse skipped) | [x] |
| X8 | `process_buffer` | `flags=0x1F`, `param1 ∈ {2,3,4,5,255,256,-7}` × `param2 ∈ {0,1}` × all shapes × all lengths | [x] |
| X9 | `process_buffer` | `flags=0xFFFF_FFFF` and `flags=0xFFFF_FFE0` → unknown bits ignored, identical to `flags & 0x1F` | [x] |
| X10 | `process_buffer` | fully random fuzz: `flags`, `param1`, `param2` uniform over `u32`/`i32` ranges (rotate-safe lengths), 20 000 cases | [x] |

## Feature combinations

`Cargo.toml` declares **no `[features]` table**, and `c_src/CMakeLists.txt`
declares no `option()`/`target_compile_definitions`/`#ifdef` build switches (the
C sources contain zero `#if`/`#ifdef` outside of include guards).  The complete
set of valid build configurations is therefore a single one:

| # | configuration | `cargo check` | tests |
|---|---------------|---------------|-------|
| 1 | `--no-default-features` (≡ default, ≡ all-features) | [x] | [x] |

`check_features.sh` re-derives this list from `Cargo.toml` and runs
`cargo check`/`cargo test` for every combination it finds, so the loop stays
correct if features are ever added.

## Suite-sensitivity validation (mutation testing)

Passing tests only mean something if the suite can *fail*.  Fifteen single-token
mutations were injected into the Rust sources one at a time and the whole suite
re-run for each:

| mutation | detected |
|----------|----------|
| `compact_runs`: `run_len >= threshold` → `>` | yes (37 tests) |
| `rotate_buffer`: `offset < len/2` → `<=` | yes (29 tests) |
| `rotate_buffer`: `offset < len/2` → `< len/2 + 1` | yes |
| `interleave_halves`: `buf[len-1] = buf[half]` → `buf[half-1]` | yes (27 tests) |
| `interleave_halves`: `half <= 256` → `<= 255` | yes |
| `reverse_segments`: `remainder > 1` → `> 2` | yes (27 tests) |
| `reverse_segments`: `seg_size <= 1` → `<= 2` | yes |
| `remove_duplicates`: `swap(write, i)` → `buf[write] = buf[i]` | yes (27 tests) |
| `process_buffer`: `buffer.is_empty() \|\| length == 0` → `&&` | yes |
| `process_buffer`: `param1 % length` → `+ 1` | yes |
| `rotate_buffer`: `offset += len` → `+= len - 1` | yes |
| `process_buffer`: `param1 <= 255` → `< 255` | yes |
| `process_buffer`: `seg_size` default `4` → `5` | yes |
| `main.rs`: `length > 256` → `> 255` | yes |
| `main.rs`: scanf overflow → `Some(0)` instead of `u64::MAX` | yes |
| `ffi.rs`: write window `2 * length` → `length` | yes (SIGABRT) |

Three further mutations were **not** detected because they are *semantically
equivalent* to the original, i.e. no input can tell them apart:

| mutation | why it cannot be detected |
|----------|---------------------------|
| `compact_runs`: `if run_len > 255` → `> 254` | for `run_len == 255` both leave the value at `255`; for `> 255` both clamp to `255` |
| `process_buffer`: interleave guard `new_len >= 2` → `>= 3` | `interleave_halves` on two bytes is the identity (`half == 1`: `buf[1] = buf[1]; buf[0] = temp[0]`) |
| `ffi.rs`: drop `\|\| length == 0` from the NULL guard | `view_len(0, ..) == 0`, and the inner `process_buffer` re-checks `length == 0`, returning `0` either way |
