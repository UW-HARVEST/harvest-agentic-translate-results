# CONFIGS.md — Configuration surface table (Phase A, gate for Phase B)

The mirror of `ERRORS.md`: every **valid** input axis the C actually branches on
or is sensitive to. Derived from the source, not guessed.

## Axis extraction (mechanical)

Public entry points, from the complete `c_src/include/lib.h`:

| entry point | signature | level |
|-------------|-----------|-------|
| `custom_strdup` | `char *custom_strdup(const char *str)` | **lowest-level == only level.** There is no convenience wrapper and no one-shot helper to hide behind; every test below drives the real entry point directly. |

Runtime options / modes / flags the public API can set:

| axis | values | source evidence |
|------|--------|-----------------|
| *(none)* | — | `lib.h` declares one function taking one `const char *`. `grep` finds **0** `#ifdef`/`#if`, **0** `switch`, and no global/static state in `lib.c`; the only `if`s are the two null checks in `ERRORS.md`. There is no byte-order, element-type, or format option to vary. |

Because there are no option flags, the configuration surface is exactly the
space of input **shapes** that the code is sensitive to. The code's behaviour
depends on the input through three operations only — `strlen(str)`, `malloc(len)`,
`memcpy(dst, str, len)` — so the shapes that matter are:

| axis | distinct values the code can distinguish |
|------|------------------------------------------|
| **length** (drives `strlen`, `malloc` size, `memcpy` count) | 0 (empty) · 1 · small 2..64 · medium 65..4096 · page-crossing ~4096 · large 1 MiB..8 MiB |
| **byte content** (drives `memcpy` fidelity) | ASCII · all 255 non-NUL values incl. `0x80..0xFF` (negative as signed `char`) · embedded high-entropy random bytes · repeated patterns |
| **source alignment** (drives `strlen`/`memcpy` SIMD prologue/epilogue) | offsets 0..16 inside the backing buffer, and unaligned tails |
| **placement** (drives `strlen` over-read behaviour) | ordinary heap buffer · NUL as the last readable byte before an unmapped guard page |
| **call multiplicity** (statelessness) | one call · many interleaved calls · alternating with `free` |

Rows below are the pruned cross-product: one row per combination the code
actually treats differently.

## Configuration-surface table

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_9876`, xorshift64\* PRNG, so runs are reproducible) unless the
row is a single degenerate shape. Both `.so`s are called through `libloading`
and the results compared byte-for-byte, including the terminating NUL, and the
returned pointer is checked to be a distinct, `free()`-able allocation.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `custom_strdup` | no options (none exist) + **empty string** `""` → `len == 1`, 1-byte `malloc`, 1-byte `memcpy` | `c1_empty_string` | [x] |
| C2 | `custom_strdup` | **length exactly 1** (single payload byte), swept over **all 255** non-NUL byte values | `c2_single_byte_all_values` | [x] |
| C3 | `custom_strdup` | **small lengths 2..=64**, randomized printable-ASCII content, many samples per length | `c3_small_ascii_random` | [x] |
| C4 | `custom_strdup` | **medium lengths 65..=4096**, randomized content, many samples | `c4_medium_random` | [x] |
| C5 | `custom_strdup` | **page-crossing lengths** (4095, 4096, 4097 and ±1 around 512/1024/2048/8192) — exercises `strlen`/`memcpy` page-boundary paths | `c5_page_boundary_lengths` | [x] |
| C6 | `custom_strdup` | **full byte alphabet**: a string containing every value `0x01..=0xFF` exactly once (255 bytes), plus randomized permutations of it | `c6_all_byte_values` | [x] |
| C7 | `custom_strdup` | **high-bit / non-UTF-8 content only** (`0x80..=0xFF`), randomized lengths — bytes that are *negative* in a signed `char` and invalid UTF-8, so a translation that round-tripped through `str`/`CStr::to_str` would diverge here | `c7_high_bytes_non_utf8` | [x] |
| C8 | `custom_strdup` | **large allocations**: 64 KiB, 1 MiB, 8 MiB randomized content (glibc `mmap` allocation path rather than the arena path) | `c8_large_strings` | [x] |
| C9 | `custom_strdup` | **NUL as the final readable byte before an unmapped guard page** (`mmap` two pages, `mprotect(PROT_NONE)` the second, put the string flush against the boundary) — proves neither implementation over-reads past the terminator | `c9_string_flush_against_guard_page` | [x] |
| C10 | `custom_strdup` | **unaligned source pointers**: same payload started at offsets 0..=16 within an over-allocated buffer, randomized lengths — exercises the SIMD prologue of `strlen`/`memcpy` | `c10_unaligned_source_offsets` | [x] |
| C11 | `custom_strdup` | **repeated / interleaved calls** (stateless­ness): 2000 randomized calls in one process, results freed in a shuffled order, C and Rust calls interleaved so any shared-state or allocator-interference bug shows up | `c11_many_interleaved_calls` | [x] |
| C12 | `custom_strdup` | **result independence**: returned buffer must not alias the input; mutating the source *after* the call must not change the copy, and mutating the copy must not change the source | `c12_result_is_independent_copy` | [x] |
| C13 | `custom_strdup` | **allocator compatibility**: the returned pointer is released with libc `free` (not a Rust deallocator) for both `.so`s, over many sizes — a translation using the Rust global allocator would corrupt the heap here | `c13_returned_pointer_is_free_able` | [x] |
| C14 | `custom_strdup` | **embedded-NUL truncation semantics**: buffer holds `"abc\0def"`; the C copies only up to the *first* NUL, so the result must be `"abc"` and 4 bytes long, not 8 | `c14_embedded_nul_truncates` | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the complete set of build
configurations is {default, `--no-default-features`} — the two are identical
builds. `run_all.sh` runs the whole Phase B + Phase C suite under both.

## Phase B gate

All 14 rows pass across their randomized inputs under every feature combination.
