# ERRORS.md — Phase A/C: error-surface table

Mechanically derived from `c_src/src/lib.c`. The complete set of non-happy-path
exits in the C source is found by:

```sh
grep -n 'return\|assert\|NULL\|if (' c_src/src/lib.c
```

`c_src` contains **no** `assert`, **no** error enum, **no** `RETURN_ERROR`-style
macro, **no** length/size parameter, **no** min/max constant, and **no** null
check. The entire *explicit* rejection surface is three `return` statements
(`-1`, `-2`, `-3`); everything else is an *absent* check whose behaviour is
still observable and must match.

`ima_parse` is the only exported symbol, so every row is on it.

Legend for "expected C result":
* `ret N` — returns `N`, `*info` untouched.
* `SIGSEGV` — no check exists; the C dereferences a null/wild pointer. The Rust
  must fault the same way; verified by re-exec'ing the test binary as a child
  process and comparing the termination signal.
* `hang` — no check exists; the C loops forever. Verified by both children
  hitting the same timeout.

All rows verified against the default cmake C build, in all 5 build
configurations of `CONFIGS.md`, **and** against C rebuilt at `-O0/-O1/-O2/-O3/-Os`.

## Explicit rejections

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `ima_parse` | `ima_btoh32(header->type) != 'caff'`, i.e. bytes `[0..4)` are not ASCII `"caff"`. Checked first, before anything else. | `ret -1` | `row01_bad_magic_returns_minus_1` (all 4x256 single-byte mutations of the magic + 3000 random codes) | [x] |
| 2 | `ima_parse` | `ima_btoh16(header->version) != 1`, i.e. big-endian `u16` at bytes `[4..6)` is not `1`. Only reached when row 1 passes. | `ret -2` | `row02_bad_version_returns_minus_2` (**exhaustive**: all 65536 values) | [x] |
| 3 | `ima_parse` | `ima_btoh32(desc->format_id) != 'ima4'`, i.e. bytes `[desc+8 .. desc+12)` are not ASCII `"ima4"`. Only reached after a `data` chunk broke the loop. | `ret -3` | `row03_bad_format_id_returns_minus_3` (3000+ random codes) | [x] |

## Absent checks (the C does *not* reject; behaviour still observable)

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 4 | `ima_parse` | `data == NULL` (no null check on `data`) | `SIGSEGV` reading `*(u32*)0` | `row04_null_data_faults_in_both` | [x] |
| 5 | `ima_parse` | `info == NULL`, input otherwise fully valid (no null check on `info`) | `SIGSEGV` writing `*(void**)0` | `row05_null_info_faults_in_both` | [x] |
| 6 | `ima_parse` | `data == NULL` **and** `info == NULL` | `SIGSEGV` (the read of `data` faults first) | `row06_null_data_and_info_faults_in_both` | [x] |
| 7 | `ima_parse` | a `data` chunk is reached with **no preceding `desc` chunk** → `desc` is still `NULL` and `desc->format_id` is dereferenced | `SIGSEGV` reading `*(u32*)0x8` | `row07_null_desc_faults_in_both` | [x] |
| 8 | `ima_parse` | a `data` chunk is reached with a valid `ima4` `desc` but **no preceding `pakt` chunk** → `pakt` is `NULL` and `pakt->frame_count` is dereferenced. `info->blocks`/`info->size` are stored *before* the fault. | `SIGSEGV` reading `*(u64*)0x8` | `row08_null_pakt_faults_in_both` | [x] |
| 9 | `ima_parse` | **no `pakt` chunk and `desc->format_id != 'ima4'`** — the `-3` check happens *before* `pakt` is touched, so this returns cleanly instead of faulting. Ordering-sensitive: a Rust version that validated `pakt` early would diverge. | `ret -3` (no fault) | `row09_bad_format_id_beats_null_pakt` (2000 random) | [x] |
| 10 | `ima_parse` | valid header, but the buffer contains **no `data` chunk** → `for(;;)` never breaks and walks past the end of the buffer | fault on a wild address | `row10_no_data_chunk_walks_off_the_end_in_both` + in-process `row10_long_unknown_chunk_walk_is_identical` (walks of 1…65535 unknown chunks compared byte-exactly) | [x] |
| 11 | `ima_parse` | `chunk_size == -16` on a non-`desc`/`pakt`/`data` chunk → `chunk += 16 + (-16)` leaves `chunk` unchanged → tight infinite loop, no forward-progress check | `hang` (forever) | `row11_chunk_size_minus_16_loops_forever_in_both` (both children time out) | [x] |
| 12 | `ima_parse` | `chunk_size < 0` → the chunk pointer walks *backwards*; no lower-bound check | fault on a wild address | `row12_hugely_negative_chunk_size_faults_in_both`; the *benign* backwards walk (landing back on a valid `data` chunk) is `row13_backward_walk_onto_data_chunk` in Phase B | [x] |
| 13 | `ima_parse` | `chunk_size == INT64_MAX` / `INT64_MIN` → pointer arithmetic overflows; no range check | fault on a wild address | `row13_chunk_size_i64_extremes_fault_in_both` | [x] |
| 14 | `ima_parse` | `chunk_size` at and past every "reasonable" bound (`0`, `±1`, `2^32`, `±2^47`, `i64::MIN`, `i64::MAX`) on a `data` chunk. `info->size` is set from the raw value with **no validation**, as the `s64`→`u64` bit-preserving reinterpretation. | `ret 0`, `info->size == (u64)chunk_size` | `row14_data_size_unvalidated`, `row10_data_size_random_full_range`, `row11_data_size_boundaries` | [x] |
| 15 | `ima_parse` | `header->flags` set to any of `0..=0xffff` — the field is **never read**; no rejection may depend on it | `ret 0` (flags inert) | `row15_header_flags_inert` (byte-identical output across 20k+ flag values) | [x] |
| 16 | `ima_parse` | `chunk->type` = any of the 2^32 values that is **not** `'desc'`/`'pakt'`/`'data'`, including one-byte-off near-misses of each. There is no `enum` anywhere in this ABI, so this open 32-bit discriminant is the closest analogue of an out-of-range enum value crossing FFI. All fall through to the advance path with no rejection. | no rejection; `chunk += 16 + size` | `row16_unrecognised_chunk_types_are_skipped_identically` (all 5×4×256 single-byte mutations of every fourcc + 2000 random), `row32_chunk_type_values_and_mutations` | [x] |
| 17 | `ima_parse` | `desc->sample_rate` bytes forming a `double` that is negative, `NaN`, `±Inf`, `>= 2^63`, or `>= 2^64` when read as a native (little-endian) `double`. The C then does a `double`→`unsigned long long` *value conversion*, which C leaves **undefined** for all of these. No check exists. | no rejection; `ret 0` with the x86-64 `cvttsd2si` result (`0x8000000000000000` "integer indefinite" where out of range), then byte-swapped and bit-cast | `fuzz_sample_rate_pipeline` (10^6 inputs; measured branch coverage 371611 / 191541 / 311709 / 125139 over in-range / `subsd` / negative / NaN), plus `row17`–`row23` | [x] |
| 18 | `ima_parse` | `desc->channels_per_frame == 0`, `1`, `0xffffffff` — no range check on channel count | `ret 0`, `info->channel_count == bswap32(raw)` | `row18_channel_count_unvalidated`, `row24_channel_count_values` | [x] |
| 19 | `ima_parse` | `pakt->frame_count` negative / `INT64_MIN` / `INT64_MAX` — no range check | `ret 0`, `info->frame_count == bswap64(raw)` | `row19_frame_count_unvalidated`, `row25_frame_count_values` | [x] |
| 20 | `ima_parse` | Truncated buffer: valid `caff` + version but fewer than 16 bytes of chunk present, so the first `chunk->type`/`chunk->size` load reads past the logical end of the stream. There is no length parameter to check against. | no rejection; reads whatever follows | `row20_truncated_stream_reads_past_logical_end` (truncation lengths 8…40, with the "past the end" bytes made real and known so the comparison is deterministic) | [x] |

## Ordering constraints that must be preserved

The C performs its checks in exactly this order; any reordering in Rust changes
which error a multiply-invalid input produces. Asserted directly by
`return_codes_are_only_0_minus1_minus2_minus3`, which drives 20000 inputs that
are independently valid/invalid in each of the three dimensions, checks the
resulting code against the precedence model, and asserts all four exit codes
(`0`, `-1`, `-2`, `-3`) are actually reached:

1. `type != 'caff'` → `-1` — wins over a bad version *and* a bad `format_id`.
2. `version != 1` → `-2` — wins over a bad `format_id` and over a malformed
   chunk list (the chunk walk is never entered).
3. chunk walk runs to the first `data` chunk — a bad `format_id` in a `desc`
   chunk does **not** stop the walk; only `data` breaks it.
4. `format_id != 'ima4'` → `-3` — wins over a `NULL` `pakt` (row 9).
5. Stores happen in source order `blocks`, `size`, `frame_count`,
   `channel_count`, `sample_rate`, so a fault in row 8 leaves the first two
   fields already written.

## Notes on the fault rows

### Bug found and fixed: wrong signal on null/wild pointers

Rows 4–8 originally **failed**. The Rust read through `ptr::read_unaligned`,
which is wrong for reproducing a C raw load in two independent ways:

* `read_unaligned` forwards to `copy_nonoverlapping`, whose
  `debug_assertions`-only "is not null" precondition fires `panic_nounwind` —
  so the Rust died with **SIGABRT** where the C died with **SIGSEGV**;
* with optimisation and `debug_assertions` off, a plain load from a
  provably-invalid pointer is UB that LLVM deletes outright, so the release
  build did **not fault at all** (verified: exit code 0).

The same applied to the `(*info).field = …` stores. Both are now done with
`read_volatile`/`write_volatile` through an `align == 1` `#[repr(C, packed)]`
wrapper (`src/parse.rs`): alignment is trivially satisfied so there is no
misaligned-deref abort, `read_volatile` has no null precondition, and a volatile
access can never be elided. Rows 4–8 now report SIGSEGV from both `.so`s in
every build configuration.

### Why rows 10, 12 and 13 compare "both faulted" rather than "same signal"

For rows 4–8 the faulting address is exact and canonical (`0` or `0x8`), so
SIGSEGV is deterministic and the signals are compared for equality.

Rows 10, 12 and 13 fault on a *wild* address produced by unchecked pointer
arithmetic. On x86-64, an address whose high bits are not a sign extension of
bit 47 is **non-canonical**, and touching it raises a general-protection fault
that Linux reports as **SIGBUS**, whereas a canonical-but-unmapped address
raises a page fault reported as **SIGSEGV**. Which side of that line
`chunk + 16 + size` lands on depends on how the compiler folds the arithmetic —
measured: the default `-O0` build and `-O2`/`-O3`/`-Os` give SIGSEGV, while
`-O1` gives SIGBUS for `size == i64::MAX`. That is a property of the address, not
of the library, so these rows require both sides to die by one of those two
signals. The loop *logic* is still pinned down exactly, in-process and
byte-for-byte, by `row10_long_unknown_chunk_walk_is_identical`.
