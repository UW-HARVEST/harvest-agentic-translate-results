# CONFIGS.md — configuration-surface table (Phase A / Phase B)

Derived mechanically from `c_src/include/lib.h` and `c_src/src/lib.c`.

## Axis extraction from the C source

**Runtime options / modes / flags.** `grep -n 'if\|switch\|#if\|#ifdef' c_src/src/lib.c`
returns only the two null checks already listed in `ERRORS.md`. There is no
global state, no init/teardown, no options struct, no `#ifdef` in either file.
**Number of runtime option axes: 0.**

**Public entry points.** `c_src/include/lib.h` is one line and declares exactly
one function, `custom_strdup`. It is simultaneously the highest- and the
lowest-level entry point — there is no convenience wrapper to hide behind, and
no internal helper with external linkage (`nm -D` confirms a single `T` symbol).

**Input shapes the code is sensitive to.** The function body is
`strlen` → `+1` → `malloc` → `memcpy`. Although the C has no explicit
size branches, the *observable* behaviour varies with:

* `len = strlen(str) + 1` — the terminator-inclusive length, which selects a
  different `malloc` size class (fastbin / smallbin / page-multiple / `mmap`
  threshold) and a different `memcpy` code path (byte loop vs SSE/AVX vs
  `rep movsb` vs page copy). These are the axes where a translation that got
  the `+1` or the copy length wrong shows up.
* byte content, including `0x80`–`0xFF` (the buffer is `char*`, not UTF-8; a
  Rust translation that routed through `str`/`CStr` UTF-8 validation would
  diverge here).
* alignment of the input pointer.
* placement relative to the end of a mapped page (detects reading past the NUL).
* call sequence / repetition (detects hidden shared state — there must be none).

## Configuration-surface table

One row per meaningful combination of `{entry point} × {length class} × {content
class} × {alignment} × {call pattern}`, pruned to the combinations the C
actually distinguishes. Every row is driven with many randomized inputs
(xorshift64\* PRNG, fixed seed `0x2024_0601_C0FFEE01`), and both the C `.so` and
the Rust `.so` are called through `libloading` and compared byte-for-byte
(including the NUL terminator).

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `custom_strdup` | no options (none exist); `len_with_nul == 1`, i.e. the empty string `""` | `cfg_c1_empty` | [x] |
| C2 | `custom_strdup` | `len_with_nul == 2`: **exhaustively all 255** legal single-byte contents `0x01..=0xFF` | `cfg_c2_all_single_bytes` | [x] |
| C3 | `custom_strdup` | `len_with_nul == 3`: **exhaustively all 255×255** two-byte contents | `cfg_c3_all_two_byte_pairs` | [x] |
| C4 | `custom_strdup` | small sizes 1..=64 bytes, randomized non-zero contents, many trials per size (fastbin / smallbin `malloc` classes, short-`memcpy` paths) | `cfg_c4_small_sizes_sweep` | [x] |
| C5 | `custom_strdup` | `memcpy`/`malloc` alignment boundaries: lengths `{7,8,9,15,16,17,23,24,25,31,32,33,63,64,65,127,128,129}`, randomized contents | `cfg_c5_alignment_boundaries` | [x] |
| C6 | `custom_strdup` | page boundaries: lengths `{4094,4095,4096,4097,4098,8191,8192,8193}`, randomized contents | `cfg_c6_page_boundaries` | [x] |
| C7 | `custom_strdup` | large allocation, 1 MiB of randomized bytes | `cfg_c7_one_mib` | [x] |
| C8 | `custom_strdup` | past `malloc`'s default `mmap` threshold: 16 MiB + 1 of randomized bytes (different allocator path entirely) | `cfg_c8_mmap_threshold` | [x] |
| C9 | `custom_strdup` | high-bit-only / non-UTF-8 contents (`0x80..=0xFF`, plus deliberately invalid UTF-8 sequences: lone continuation bytes, truncated multi-byte sequences, `0xFE`/`0xFF`), randomized lengths | `cfg_c9_non_utf8` | [x] |
| C10 | `custom_strdup` | misaligned input pointer: same logical string read at offsets 0..=15 inside an over-aligned backing buffer, randomized lengths | `cfg_c10_misaligned_input` | [x] |
| C11 | `custom_strdup` | result-ownership shape: returned pointer is non-null, **not** aliasing the input, and releasable via libc `free()`; two successive calls return distinct buffers | `cfg_c11_result_is_free_able` | [x] |
| C12 | `custom_strdup` | input whose NUL terminator is the **last readable byte** before an unmapped guard page (proves neither impl reads past the terminator), lengths 1..=64 | `cfg_c12_guard_page` | [x] |
| C13 | `custom_strdup` | call pattern: 2000 interleaved C/Rust calls with randomized inputs, results kept alive simultaneously (proves no shared/leaked state and no allocator interference) | `cfg_c13_interleaved_stateful` | [x] |
| C14 | `custom_strdup` | free-form property sweep: 5000 iterations, random length `0..=8192`, random byte contents, fixed seed | `cfg_c14_property_sweep` | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the cross-product of
cargo features is a single point. `tests/feature_matrix.sh` still runs the
default build and `--no-default-features` explicitly so the claim is verified
rather than assumed.
