# CONFIGS.md — Configuration surface table (Phase B gate)

Derived mechanically from the branches the C code actually takes.

## Public entry points (the FULL set)

| symbol | linkage | reachable how |
|--------|---------|---------------|
| `dequantize_granule(float*, bs_t*, L12_scale_info*, int)` | exported | called directly through the `.so` |
| `get_bits(bs_t*, int)` | `static` | **not exported**; the only way to drive it is through `dequantize_granule`, so its axes (`n`, `pos&7`, `limit`) appear below as axes of the public call |

There is no convenience/one-shot wrapper and no second layer: the single
exported function *is* the lowest-level entry point. All rows drive it directly
through `dlsym`, with the full `bs_t` / `L12_scale_info` state set up by hand.

## Axes the C branches on

No `#ifdef`, no runtime flag/option/mode setter, no global state exists. Every
axis is carried by the input structs:

* **A. `sci->total_bands`** (`uint8_t`) — bound of `i < 2*total_bands`. Special
  values: `0` (loop empty), `1`, `32` (`i` max 63 = last in-array index),
  `33` (`i` reaches 64 → reads into `scfcod`), `64` (`i` max 127 = last
  `scfcod` byte), `65` (`i` reaches 128 → past the struct), `255` (max, `i` to 509).
* **B. `sci->bitalloc[i]`** — three-way branch: `== 0` (skip), `1..=16`
  (linear path, `half = (1<<(ba-1))-1`), `>= 17` (grouped path,
  `mod = (2<<(ba-17))+1`). Sub-cases inside the grouped path: `17..=21`
  (`n <= 32`), `22..=31` (`n > 32` → shift-count UB), `48` (`mod == 1`),
  `49..=255` (shift count `>= 32` → wraps mod 32).
* **C. `group_size`** — bound of both `k` loops and the `dst` base
  (`grbuf + group_size*j`) and the return value. Values: `< 0`, `0`, `1`, `2`,
  `3` (Layer I), `12` (Layer II), large (`64`).
* **D. `bs->pos`** — bit alignment `pos & 7` ∈ 0..7 selects the `255 >> s`
  mask and the `p` start byte; also `pos > limit` at entry.
* **E. `bs->limit`** — the exhaustion guard: generous / exactly on the boundary
  / one past / mid-run / `0` / negative.
* **F. `bs->buf` contents** — random, all `0x00`, all `0xFF`.
* **G. bytes following `bitalloc`** (`scfcod`, then past-struct memory) — become
  `ba` values once axis A pushes `i >= 64`.
* **H. `choff` phase** — `576 / -558` alternation persists *across* the `j`
  loop; only observable when `2*total_bands` is walked more than once, i.e. all
  rows with `total_bands >= 1` (`2*tb` is always even, so `choff` resets to 576
  each `j` — this is asserted implicitly by the full-`grbuf` comparison).

`sci->scf` and `sci->stereo_bands` are **never read** by the C; they are filled
with random bytes in every row so that any spurious Rust read would diverge.

## Rows (pruned cross-product of the distinguished combinations)

Every row is run with **many randomized inputs** (fixed seed, `SplitMix64`);
randomization covers `buf` bytes, the concrete `bitalloc` values inside the
row's stated class, `scf`/`stereo_bands`/`scfcod` filler, and the past-struct
padding. Both `.so`s get bit-identical inputs; `grbuf` (raw `u32` bit patterns),
the returned `int`, and the final `bs->pos` are all compared.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `dequantize_granule` | `total_bands=0`; any `bitalloc`; `group_size=3`; limit generous — empty-loop degenerate | [x] |
| C2 | `dequantize_granule` | `total_bands=1`; all `bitalloc=0`; `group_size=3` — skip-every-band | [x] |
| C3 | `dequantize_granule` | `total_bands=1`; `bitalloc ∈ 1..=16` random; `group_size=1`; aligned `pos=0`; limit generous | [x] |
| C4 | `dequantize_granule` | `total_bands=1`; `bitalloc ∈ 1..=16`; `group_size=12`; limit generous | [x] |
| C5 | `dequantize_granule` | `total_bands=32`; `bitalloc ∈ 1..=16` (all 64 in-array indices used); `group_size=12` | [x] |
| C6 | `dequantize_granule` | `total_bands=32`; `bitalloc` mixed `{0} ∪ 1..=16`; `group_size=3` — sparse bands | [x] |
| C7 | `dequantize_granule` | `total_bands=1`; `bitalloc = 16` exactly (max linear, `half=32767`); `group_size=12` | [x] |
| C8 | `dequantize_granule` | `total_bands=1`; `bitalloc = 1` exactly (min, `half=0`, 1 bit/sample); `group_size=12` | [x] |
| C9 | `dequantize_granule` | `total_bands=8`; `bitalloc ∈ 17..=21` (grouped, `n<=32`, `mod ∈ {3,5,9,17,33}`); `group_size=3` | [x] |
| C10 | `dequantize_granule` | `total_bands=8`; `bitalloc = 17` exactly (`mod=3`, `n=5`); `group_size=12` — grouped, code exhausted by `code/=mod` | [x] |
| C11 | `dequantize_granule` | `total_bands=4`; `bitalloc ∈ 22..=31` (grouped, `n>32` → masked-shift UB path); `group_size=3`; big `buf`, generous limit | [x] |
| C12 | `dequantize_granule` | `total_bands=2`; `bitalloc = 48` (`mod == 1`, all-zero output, `n=3`); `group_size=12` | [x] |
| C13 | `dequantize_granule` | `total_bands=2`; `bitalloc ∈ 49..=255` (shift count wraps mod 32); `group_size=3`; `limit` small so huge `n` early-outs | [x] |
| C14 | `dequantize_granule` | `total_bands=1`; `bitalloc` sweeping **all 256 byte values** one at a time; `group_size=3`; `limit` small | [x] |
| C15 | `dequantize_granule` | `total_bands=33`; `bitalloc` in-array random, `scfcod` random → `ba` read from `scfcod` (`i` 64..65) | [x] |
| C16 | `dequantize_granule` | `total_bands=64`; `i` up to 127 → whole `scfcod` consumed as `ba`; `group_size=3` | [x] |
| C17 | `dequantize_granule` | `total_bands=65`; `i` up to 129 → reads **past the struct** into controlled padding | [x] |
| C18 | `dequantize_granule` | `total_bands=255`; `i` up to 509 → 510 bands, deep past-struct read; `group_size=3`, `limit` moderate | [x] |
| C19 | `dequantize_granule` | `pos & 7` swept 0..7 (unaligned start), `total_bands=8`, `bitalloc ∈ 1..=16`, `group_size=3` | [x] |
| C20 | `dequantize_granule` | `pos` large but `< limit` (deep into a big buffer), `total_bands=8`, `bitalloc ∈ 1..=16` | [x] |
| C21 | `dequantize_granule` | `limit` exactly at the last bit consumed (boundary, guard `>` not taken), `total_bands=4`, `bitalloc=8`, `group_size=3` | [x] |
| C22 | `dequantize_granule` | `limit` mid-run (prefix decodes, suffix all-zero), `total_bands=16`, `bitalloc ∈ 1..=16`, `group_size=12` | [x] |
| C23 | `dequantize_granule` | `buf` all `0x00`, `total_bands=16`, `bitalloc ∈ 1..=16` and `17..=21`, `group_size=12` | [x] |
| C24 | `dequantize_granule` | `buf` all `0xFF`, `total_bands=16`, `bitalloc ∈ 1..=16` and `17..=21`, `group_size=12` | [x] |
| C25 | `dequantize_granule` | `group_size=0`, `total_bands=8`, `bitalloc` mixed linear+grouped — grouped path still consumes one `get_bits` per band | [x] |
| C26 | `dequantize_granule` | `group_size<0` (`-1`, `-7`, `-33`), `total_bands=8`, mixed `bitalloc` | [x] |
| C27 | `dequantize_granule` | `group_size=64` (oversized), `total_bands=8`, `bitalloc ∈ 1..=16` — `dst` walks far past 576 | [x] |
| C28 | `dequantize_granule` | `group_size=2` (even, non-Layer size), `total_bands=17` (odd band count → `2*tb` still even), mixed `bitalloc` | [x] |
| C29 | `dequantize_granule` | full fuzz: every axis randomized jointly (`total_bands ∈ 0..=70`, `bitalloc` any of 0..=255 restricted to the safe-`n` classes, `group_size ∈ -8..=16`, `pos` any, `limit` any) | [x] |
| C30 | binary/driver | `CMakeLists.txt` defines **no executable target** and the crate has no `[[bin]]` — there is no driver, so the stdout-comparison item is vacuous (verified, not skipped) | [x] |

## Status

All 30 rows have a passing differential test in `tests/phase_b_configs.rs`
(30 tests, `30 passed; 0 failed`). Row → test mapping is `Cn` → `fn c0n_*`,
verified mechanically (0 rows without a test).

Per-row randomization (fixed seeds, `SplitMix64`): 400 inputs for the plain
rows, 512 for C14 (all 256 `ba` byte values × 8 bitstreams), 512 for C19
(8 bit alignments × 64), 192 for C21 (3 limit offsets × 64), 192 for C26
(3 negative `group_size` values × 64), and 6000 for the C29 joint fuzz --
roughly 20k differential calls, each one a `fork`ed C call plus a `fork`ed Rust
call on byte-identical input.

What is compared after every call, for both libraries:

* the returned `int`;
* `bs->pos` and `bs->limit` (so a mis-advanced bitstream cursor is caught even
  when the decoded samples happen to agree);
* every one of the 16896 `grbuf` words as **raw bit patterns** (so `-0.0`, NaN
  and out-of-range writes are all distinguished);
* every byte of the `L12_scale_info` region and of the bitstream buffer, to
  catch a spurious write neither library should make;
* whether the call completed at all, and if not, the exact fatal signal.

`grbuf` is handed out 512 floats into a larger reservation so a negative
`group_size` can walk backwards, and the whole working region is bracketed by a
512 MiB `PROT_NONE` reservation -- wider than the `buf ± 256 MiB` that a 32-bit
`bs->pos` can address -- so the wild dereference the C performs when its `int`
overflow makes the exhaustion guard miss faults *deterministically and
identically* for both libraries instead of landing in some unrelated mapping.
