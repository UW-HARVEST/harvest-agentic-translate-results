# ERRORS.md — Phase C error-surface table

Mechanically derived by grepping `c_src/src/lib.c` for **every** rejection /
fault path.  There are no `assert`s, no `RETURN_ERROR`-style macros, no error
enums and no NULL checks anywhere in the C source; the complete inventory is:

```
$ grep -n 'return\|assert\|NULL\|for (;;)' c_src/src/lib.c
 78:    const struct caf_audio_description *desc = NULL;
 79:    const struct caf_packet_table *pakt = NULL;
 80:    const struct ima_block *blocks = NULL;
 94:    for (;;) {
 91:        return -1;
 93:        return -2;
122:        return -3;
130:    return 0;
```

Three explicit error returns (`-1`, `-2`, `-3`), one success (`0`), plus the
implicit fault paths created by the *absence* of NULL/bounds checking (rows 4-7,
10) and by the unbounded `for (;;)` chunk scan (rows 8-9).

Rows 4-10 are real inputs that the C library "handles" by faulting or by never
terminating, so the Rust must fault / not terminate **identically**.  They are
verified by re-exec'ing the test binary as a child process (`crash_worker`),
once against the C `.so` and once against the Rust `.so`, and comparing how the
two children terminated (`WTERMSIG`, or "still running after the timeout").
Rows 8 and 10 use an `mmap`'d buffer followed by a `PROT_NONE` guard page so the
fault address is exact and deterministic rather than dependent on heap layout.

Tests live in `tests/phase_c_errors.rs`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `ima_parse` | `ima_btoh32(header->type) != 'ffac'` — the first 4 bytes of `data` are not ASCII `"caff"`.  Checked before anything else, so the rest of the file is irrelevant. | `return -1`, `*info` untouched | `err01_bad_magic` (12 curated), `cfg01_bad_magic_randomized` (20 000) | [x] |
| 2  | `ima_parse` | magic OK **and** `ima_btoh16(header->version) != 1` — bytes 4..6 are not big-endian `0x0001` | `return -2`, `*info` untouched | `err02_bad_version` (9 curated), `cfg02_bad_version_randomized` (20 000) | [x] |
| 3  | `ima_parse` | magic + version OK, a `desc` chunk was seen before the `data` chunk, but `ima_btoh32(desc->format_id) != '4ami'` — `desc+8..12` is not ASCII `"ima4"`.  Returns *before* `pakt` is dereferenced. | `return -3`, `*info` untouched | `err03_bad_format_id` (10 curated), `err03_bad_format_id_no_pakt` (2 000), `cfg17_bad_format_id_randomized` (20 000), `cfg24_bad_format_id_without_pakt` (3 000) | [x] |
| 4  | `ima_parse` | magic + version OK and the `data` chunk is reached with **no preceding `desc` chunk** ⇒ `desc == NULL` ⇒ `desc->format_id` reads address `0x8` | SIGSEGV (NULL-relative read) | `err04_null_desc_segv` | [x] |
| 5  | `ima_parse` | magic + version + `format_id` OK and the `data` chunk is reached with **no preceding `pakt` chunk** ⇒ `pakt == NULL` ⇒ `pakt->frame_count` reads address `0x8` | SIGSEGV (NULL-relative read) | `err05_null_pakt_segv` | [x] |
| 6  | `ima_parse` | `data == NULL` (the buffer pointer is never checked) ⇒ `header->type` reads address `0x0` | SIGSEGV (NULL read) | `err06_null_data_segv` | [x] |
| 7  | `ima_parse` | `info == NULL` with an otherwise fully valid buffer (the out-param is never checked) ⇒ `info->blocks = ...` **writes** address `0x0` | SIGSEGV (NULL write) | `err07_null_info_segv` | [x] |
| 8  | `ima_parse` | magic + version OK but the buffer contains **no `data` chunk** — the `for (;;)` scan is unbounded and walks off the end of the mapping (all-zero tail ⇒ type 0, size 0 ⇒ a 16-byte stride, so it faults exactly at the guard page) | SIGSEGV (out-of-bounds read) | `err08_no_data_chunk_segv` | [x] |
| 9  | `ima_parse` | a non-`data` chunk whose `size == -16` ⇒ `chunk = &chunk[1] + (-16) == chunk` ⇒ the scan never advances | **infinite loop** — never returns | `err09_self_referential_chunk_hangs` (both children must still be running after 3 s) | [x] |
| 10 | `ima_parse` | truncated buffer: (a) `data` points at the first unreadable byte; (b) only the 4 magic bytes are readable and `header->version` is not; (c) the 8-byte header is readable but the first chunk header is not; (d) `data` is non-NULL but wholly unmapped (`0x1`) | SIGSEGV (out-of-bounds read) | `err10_truncated_header_segv` — cases `trunc_type`, `trunc_version`, `trunc_chunk`, `unmapped` | [x] |

## Generic FFI-boundary cases (covered even though they are not distinct C branches)

| #  | case | why it is exercised | expected | test | [x] |
|----|------|---------------------|----------|------|-----|
| 11 | `version` one step past the valid range (`0`, `1`, `2`) plus an **exhaustive** sweep of all 65 536 values | `== 1` is the only accepted value | `-2` for every value but `1` | `err11_version_boundaries`, `cfg03_version_exhaustive` | [x] |
| 12 | `magic`: fully random 32-bit values (no filtering, so the valid value is occasionally hit) at all 8 alignments | any 4 bytes are a legal input | `0` iff exactly `"caff"`, else `-1` | `err12_magic_randomized` (20 000) | [x] |
| 13 | `format_id`: fully random 32-bit values (unfiltered) at all 8 alignments | any 4 bytes are a legal input | `0` iff exactly `"ima4"`, else `-3` | `err13_format_id_randomized` (20 000) | [x] |
| 14 | **out-of-range "enum" values across the FFI boundary**: `chunk->type` is an unconstrained `ima_u32_t` fed into an `if`/`else if` chain with exactly 3 recognised values; 1..4 chunks per file get completely arbitrary 32-bit types (C enums/`switch`es accept any `int`, so a value with no valid variant is a real input).  `desc`/`pakt` are emitted first so the unfiltered types stay non-faulting. | an unrecognised type must take the fall-through *skip* branch, not be mis-dispatched | identical skip behaviour; final `0`, with `info->size/frame_count/channel_count` unaffected | `err14_unknown_chunk_type_enum_fuzz` (20 000, ≈50 000 fuzzed types) | [x] |
| 15 | oversized / negative lengths: `chunk->size` = `0`, `±1`, `i64::MIN`, `i64::MAX`, `-16`, `-32`, `u64::MAX`, `u64::MAX>>1`, `±2^62`, × all 8 alignments, plus randomized | the `data` chunk's length is copied verbatim into `info->size` as a `u64` (signed→unsigned reinterpretation) | identical `info->size` bits | `err15_chunk_size_extremes`, `cfg13_data_chunk_size_extremes` | [x] |
| 16 | misaligned `data` pointer, offsets 0..7 | the C casts the buffer to `struct caf_*` with no alignment guarantee, so the Rust must use unaligned loads | identical results, no fault | `err16_misaligned_pointer`, `cfg16_misaligned_buffer` | [x] |

## Divergence found and fixed during Phase C

`err06` / `err07` initially **failed in the `dev` profile only**: the C child died
with `SIGSEGV` (11) while the Rust child died with `SIGABRT` (6).

Cause: rustc's optional Undefined-Behaviour instrumentation (the MIR null-check
pass, plus the `assert_unsafe_precondition!` guards inside
`core::ptr::read_unaligned`) is compiled in whenever `debug-assertions` is
enabled, and it converts precisely the NULL loads/stores of rows 6 and 7 into a
non-unwinding panic — `SIGABRT` instead of the C library's `SIGSEGV`.  The C
library is built by CMake with no such instrumentation.

Fix (in `translation/Cargo.toml`, with a matching comment in `src/lib.rs`):

```toml
[profile.dev]
debug-assertions = false
overflow-checks = false
```

This is load-bearing and is itself under test: re-enabling `debug-assertions`
makes `err06_null_data_segv` and `err07_null_info_segv` fail again (verified).

## Result

```
$ cargo test --release --test phase_c_errors
test result: ok. 17 passed; 0 failed; 1 ignored   (the ignored one is the child-side `crash_worker`)
```

Every row above is checked off, under both the `dev` and `release` profiles and
under all three Cargo feature configurations (see `verify.sh`).
