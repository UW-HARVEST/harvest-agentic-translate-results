# CONFIGS.md — Phase A configuration-surface table (valid inputs)

## Axes derived from the C source

`c_src/src/lib.c` has no runtime options struct, no global state and no
setters — every axis is an *argument* or an *input shape*. The axes the C code
actually branches on:

**A. Entry point** (both public, both in `nm -D`; `extractFilename` is the
low-level one, `FIO_createFilename_fromOutDir` the composed one)
- `extractFilename(path, separator)`
- `FIO_createFilename_fromOutDir(path, outDirName, suffixLen)`

**B. Compile-time separator selection** (`#if defined(_MSC_VER) || __MINGW32__
|| __MSVCRT__`): `'\\'` + a second `extractFilename(.., '/')` pass on Windows,
plain `'/'` elsewhere. The reference CMake/Linux build takes the `'/'` branch,
so that is the only reachable configuration and the Rust mirrors it.

**C. `separator` argument of `extractFilename`** — the full `char` domain,
widened to `int` at the `strrchr` call: `'/'`, other printable, `'\0'`, and
high-bit/negative bytes (`0x80`, `0xFF`).

**D. `path` shape** — the `strrchr` outcome: separator absent / present once /
present many times / at index 0 only / as the last byte (empty basename) /
`path` empty / long / non-ASCII bytes.

**E. `outDirName` shape** — the `line 45` branch: last byte `== '/'` vs `!= '/'`;
plus length 0 / 1 / many, embedded separators, non-ASCII bytes.

**F. `suffixLen`** — feeds the allocation size and the guaranteed zero tail:
`0` / `1` / small / large-but-allocatable / wrapping (`SIZE_MAX`).

## Rows (pruned cross-product of the combinations the C distinguishes)

Every row is driven with **many randomized inputs** (`SplitMix64`, fixed seed
`0x5DEECE66D` per row) through **both** `.so` exports and compared
byte-for-byte, including the full `calloc`ed buffer (used bytes *and* the
zero tail).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `extractFilename` | `separator='/'`, random path **containing no `/`** (random length 0..64, random printable+non-ASCII bytes) | `cfg_01_extract_no_separator` | [x] |
| 2 | `extractFilename` | `separator='/'`, random path with **exactly one** `/` at a random interior index | `cfg_02_extract_one_separator` | [x] |
| 3 | `extractFilename` | `separator='/'`, random path with **many** `/` at random indices (last-occurrence semantics) | `cfg_03_extract_many_separators` | [x] |
| 4 | `extractFilename` | `separator='/'`, path where `/` is the **final byte** (empty basename result) | `cfg_04_extract_trailing_separator` | [x] |
| 5 | `extractFilename` | `separator='/'`, path where `/` is at **index 0** only ("/name") | `cfg_05_extract_leading_separator` | [x] |
| 6 | `extractFilename` | `separator='/'`, **empty** path `""` | `cfg_06_extract_empty_path` | [x] |
| 7 | `extractFilename` | random **non-`/` separator** byte (0x01..0x7F), random path that may or may not contain it | `cfg_07_extract_random_ascii_separator` | [x] |
| 8 | `extractFilename` | random **high-bit / negative separator** byte (0x80..0xFF) — exercises `char`→`int` widening — with random paths containing high-bit bytes | `cfg_08_extract_highbit_separator` | [x] |
| 9 | `extractFilename` | `separator='\0'` on random non-empty paths (matches the terminator, returns one-past-the-end) | `cfg_09_extract_nul_separator` | [x] |
| 10 | `extractFilename` | long paths (256..1024 bytes) with dense random separators | `cfg_10_extract_long_paths` | [x] |
| 11 | `FIO_createFilename_fromOutDir` | `outDirName` **ends with `/`**, path with **no** separator, `suffixLen=0` | `cfg_11_fio_outdir_slash_path_plain_suffix0` | [x] |
| 12 | `FIO_createFilename_fromOutDir` | `outDirName` **ends with `/`**, path **with** separators, `suffixLen=0` | `cfg_12_fio_outdir_slash_path_nested_suffix0` | [x] |
| 13 | `FIO_createFilename_fromOutDir` | `outDirName` **does not end with `/`**, path with **no** separator, `suffixLen=0` | `cfg_13_fio_outdir_plain_path_plain_suffix0` | [x] |
| 14 | `FIO_createFilename_fromOutDir` | `outDirName` **does not end with `/`**, path **with** separators, `suffixLen=0` | `cfg_14_fio_outdir_plain_path_nested_suffix0` | [x] |
| 15 | `FIO_createFilename_fromOutDir` | `outDirName` ends with `/`, path ends with `/` (**empty basename**), random `suffixLen` 0..8 | `cfg_15_fio_empty_basename` | [x] |
| 16 | `FIO_createFilename_fromOutDir` | `outDirName` of length **1** (`"/"` → separator branch, and one random non-`/` byte → insert branch), random path | `cfg_16_fio_outdir_len1_both_branches` | [x] |
| 17 | `FIO_createFilename_fromOutDir` | `outDirName` containing **multiple** embedded separators (`a/b/c` and `a/b/c/`), random path | `cfg_17_fio_outdir_multi_component` | [x] |
| 18 | `FIO_createFilename_fromOutDir` | `suffixLen = 1` (smallest non-zero tail), both `outDirName` branches, random path | `cfg_18_fio_suffixlen_one` | [x] |
| 19 | `FIO_createFilename_fromOutDir` | random `suffixLen` in 0..=64 (zero-tail length sweep), both branches, random path | `cfg_19_fio_suffixlen_sweep` | [x] |
| 20 | `FIO_createFilename_fromOutDir` | large-but-allocatable `suffixLen` (1 MiB..4 MiB), both branches — verifies the entire multi-MiB zero tail matches | `cfg_20_fio_suffixlen_large` | [x] |
| 21 | `FIO_createFilename_fromOutDir` | `path` = `""` (empty) with both `outDirName` branches, random `suffixLen` | `cfg_21_fio_empty_path` | [x] |
| 22 | `FIO_createFilename_fromOutDir` | **non-ASCII / high-bit** bytes throughout `path` and `outDirName` (byte-transparency), both branches | `cfg_22_fio_highbit_bytes` | [x] |
| 23 | `FIO_createFilename_fromOutDir` | long inputs: `outDirName` 128..512 bytes, `path` 128..512 bytes with dense separators, random `suffixLen` | `cfg_23_fio_long_inputs` | [x] |
| 24 | `FIO_createFilename_fromOutDir` | fully randomized fuzz over **all** axes at once (random outDir/path bytes incl. `/`, random terminal byte, random `suffixLen` 0..=256) × 4096 iterations | `cfg_24_fio_full_random_fuzz` | [x] |
| 25 | composed pipeline | `extractFilename` result fed by the caller into `FIO_createFilename_fromOutDir` (real-consumer usage: basename-of-basename), randomized | `cfg_25_pipeline_extract_then_fio` | [x] |

## Beyond content equality

Matching buffer *contents* does not by itself pin the allocation the C code
makes. Two further checks close that gap:

* a deterministic under-allocation guard in the harness
  (`malloc_usable_size(p) >= required`, which glibc always satisfies), and
* `tests/phase_e_alloc.rs`, which `LD_PRELOAD`s a `calloc` interposer
  (`tests/support/calloc_probe.c`) and compares the **exact** `(nmemb, size)`
  request both implementations make across 320+ configurations, including the
  wrapping (`SIZE_MAX`) and allocation-failure (`1<<63`) cases.

## Feature combinations

`Cargo.toml` has no `[features]` table → the single configuration is the
default. `run_all_feature_combos.sh` enumerates it (plus the explicit
`--no-default-features` build) and runs the whole suite for each.
