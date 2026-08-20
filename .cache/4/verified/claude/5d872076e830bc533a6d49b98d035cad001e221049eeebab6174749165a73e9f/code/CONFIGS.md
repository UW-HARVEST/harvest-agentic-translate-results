# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Public entry points (complete set)

| entry point | declared where | level |
|-------------|----------------|-------|
| `extractFilename(const char* path, char separator)` | not in the header, but non-`static` ⇒ exported and callable | low-level primitive |
| `FIO_createFilename_fromOutDir(const char* path, const char* outDirName, size_t suffixLen)` | `c_src/include/lib.h` | high-level wrapper (calls `extractFilename`) |

Both are tested directly through their `.so` exports; the low-level
`extractFilename` is **not** only exercised via the wrapper.

## Axes the C code actually branches on

Runtime axes (there is no options struct / flag / mode in this API — the branches
are driven purely by the argument values):

* `A1` `separator` value (`extractFilename`): found vs. not found → `if (search == NULL)` (line 11); the value itself is passed as a `char`, so all 256 byte values, including negative `c_char`s, reach `strrchr`.
* `A2` `path` shape: empty / no separator / one separator / many separators / leading separator / trailing separator / only separators / bytes ≥ 0x80 / long.
* `B1` `outDirName` last byte == separator or not → `if (outDirName[strlen(outDirName)-1] == separator)` (line 45); plus the `strlen(outDirName) == 0` degenerate shape that makes that index wrap.
* `B2` `path` shape as seen by the wrapper (decides `filenameStart`, incl. the empty-component case).
* `B3` `suffixLen`: 0 / small / large / values that make the `size_t` sum wrap (line 38).

Compile-time axes:

* `C1` `#if defined(_MSC_VER) || defined(__MINGW32__) || defined(__MSVCRT__)` (lines 27-36) selects `separator = '\\'` **and** the extra `extractFilename(filenameStart, '/')` pass. The C library here is built by `c_src/CMakeLists.txt` for the host (Linux/gcc) ⇒ the `#else` branch (`separator = '/'`, single pass). `src/lib.rs` mirrors this with `#[cfg(windows)] / #[cfg(not(windows))]` + `if cfg!(windows)`, so the compiled configuration matches. The Windows branch is not buildable on this host and is out of scope for differential testing.
* `C2` Cargo features: `Cargo.toml` has **no `[features]` table** and no optional dependencies, so the complete set of valid feature combinations is the single empty combination. `--no-default-features`, `--all-features` and the plain default build are therefore identical; the automation script runs all three.

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is driven with many randomized inputs (fixed seed `0x5EED_1234_ABCD_0001`,
xorshift64* PRNG implemented in `tests/common/mod.rs`) unless it is inherently
exhaustive.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `extractFilename` | `separator='/'`, path with **no** occurrence (random ASCII, len 0..64) | `cfg_01_extract_sep_absent` | [x] |
| 2  | `extractFilename` | `separator='/'`, path with exactly **one** occurrence at a random position | `cfg_02_extract_sep_once` | [x] |
| 3  | `extractFilename` | `separator='/'`, path with **many** occurrences (random count 2..8) | `cfg_03_extract_sep_many` | [x] |
| 4  | `extractFilename` | `separator='/'`, path **ends** with the separator (empty trailing component) | `cfg_04_extract_sep_trailing` | [x] |
| 5  | `extractFilename` | `separator='/'`, path **starts** with the separator / consists only of separators | `cfg_05_extract_sep_leading_and_only` | [x] |
| 6  | `extractFilename` | `path=""` (empty) × separator ∈ {`'/'`, `'a'`, `'\0'`, `0xFF`} | `cfg_06_extract_empty_path` | [x] |
| 7  | `extractFilename` | `separator='\0'` × random paths (NUL is "found" ⇒ one-past-end pointer) | `cfg_07_extract_nul_separator` | [x] |
| 8  | `extractFilename` | separator ≥ 0x80 (negative `c_char`) × random paths built from high-bit bytes | `cfg_08_extract_high_bit_separator` | [x] |
| 9  | `extractFilename` | **exhaustive**: all 256 separator values × random 0..48-byte paths over a byte alphabet that hits every value | `cfg_09_extract_all_separators_exhaustive` | [x] |
| 10 | `extractFilename` | long random paths (256..1024 bytes) × random separators, 400 iterations | `cfg_10_extract_long_random` | [x] |
| 11 | `FIO_createFilename_fromOutDir` | outDir **without** trailing `'/'` × path **without** separator × `suffixLen=0` | `cfg_11_fio_nosep_dir_plain_file` | [x] |
| 12 | `FIO_createFilename_fromOutDir` | outDir **without** trailing `'/'` × path **with** separators × `suffixLen=0` | `cfg_12_fio_nosep_dir_nested_path` | [x] |
| 13 | `FIO_createFilename_fromOutDir` | outDir **with** trailing `'/'` × path without separator | `cfg_13_fio_trailing_sep_dir_plain_file` | [x] |
| 14 | `FIO_createFilename_fromOutDir` | outDir **with** trailing `'/'` × path with separators | `cfg_14_fio_trailing_sep_dir_nested_path` | [x] |
| 15 | `FIO_createFilename_fromOutDir` | outDir == `"/"`, `"//"`, `"///"` (separator-only dirs) | `cfg_15_fio_separator_only_outdir` | [x] |
| 16 | `FIO_createFilename_fromOutDir` | outDir == `""` with the preceding byte pinned to `'/'` (OOB read selects the trailing-separator branch) | `cfg_16_fio_empty_outdir_prev_sep` | [x] |
| 17 | `FIO_createFilename_fromOutDir` | outDir == `""` with the preceding byte pinned to a random non-`'/'` byte (OOB read selects the else branch) | `cfg_17_fio_empty_outdir_prev_nonsep` | [x] |
| 18 | `FIO_createFilename_fromOutDir` | `path == ""` (empty filename component) × both outDir branches | `cfg_18_fio_empty_path` | [x] |
| 19 | `FIO_createFilename_fromOutDir` | path ends with `'/'` ⇒ empty `filenameStart` × both outDir branches | `cfg_19_fio_path_trailing_sep` | [x] |
| 20 | `FIO_createFilename_fromOutDir` | `suffixLen` random 1..32 (zero-padded tail must match byte-for-byte) × both outDir branches | `cfg_20_fio_small_suffixlen` | [x] |
| 21 | `FIO_createFilename_fromOutDir` | `suffixLen` large (1024, 4096, 65536) × both outDir branches | `cfg_21_fio_large_suffixlen` | [x] |
| 22 | `FIO_createFilename_fromOutDir` | non-UTF-8 / high-bit bytes (0x80..0xFF) in **both** `path` and `outDirName`, including a high-bit last byte of outDir (sign-extension of the `char` comparison on line 45) | `cfg_22_fio_high_bit_bytes` | [x] |
| 23 | `FIO_createFilename_fromOutDir` | long inputs: outDir 1..512 bytes × path 1..512 bytes × random `suffixLen` | `cfg_23_fio_long_inputs` | [x] |
| 24 | `FIO_createFilename_fromOutDir` | full randomized cross-product property test: 2000 iterations over random outDir/path/suffixLen drawn from all shapes above (separator density, emptiness, high-bit bytes, trailing separator) | `cfg_24_fio_random_property` | [x] |
| 25 | `extractFilename` + `FIO_createFilename_fromOutDir` | composed pipeline: the offset returned by the low-level `extractFilename` must be consistent with the tail of the wrapper's output for the same inputs (both libraries cross-checked, C↔Rust and Rust↔C) | `cfg_25_composed_pipeline_consistency` | [x] |
