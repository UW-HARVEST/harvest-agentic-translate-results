# ERRORS.md — error-surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping every `return`,
`assert`, `NULL`, comparison and loop construct:

```sh
grep -n 'return\|assert\|NULL\|!=\|==\|break\|for (\|while' c_src/src/lib.c c_src/include/lib.h
```

The C library has exactly **one** public entry point (`ima_parse`) and exactly
**three** rejection branches plus one success branch. There are **no** `assert`s,
**no** min/max constants, **no** explicit length/size arguments and **no**
null-pointer checks anywhere in the C source — the absence of those checks is
itself part of the observable surface and is recorded below as UB rows.

`ima_bswap16/32/64` and `ima_btoh16/32/64` are total functions over their
argument type: they have no rejection branches at all.

## Rejection rows (each has a differential test in `tests/errors.rs`)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `ima_parse` | `ima_btoh32(header->type) != 'caff'` — i.e. the first 4 bytes of `data` are not the ASCII sequence `c`,`a`,`f`,`f` (checked **before** anything else) | returns `-1`, `*info` left untouched |
| E2 | `ima_parse` | header magic is `caff` but `ima_btoh16(header->version) != 1` — bytes 4..6 big-endian are not `0x0001` | returns `-2`, `*info` left untouched |
| E3 | `ima_parse` | magic and version valid, chunk walk reached a `data` chunk, but `ima_btoh32(desc->format_id) != 'ima4'` — bytes 8..12 of the `desc` chunk payload are not the ASCII sequence `i`,`m`,`a`,`4` | returns `-3`, `*info` left untouched |
| E4 | `ima_parse` | all three checks pass | returns `0` and writes all five `ima_info` fields |

### Sub-rows of E1 exercised explicitly (each is a distinct byte pattern that
must produce `-1`)

| # | trigger | expected |
|---|---------|----------|
| E1a | all-zero 4-byte magic | `-1` |
| E1b | magic `ffac` (the little-endian byte order of the literal `'f'\|'f'<<8\|'a'<<16\|'c'<<24`) — proves the code is big-endian, not native | `-1` |
| E1c | magic `caf\0`, `Caff`, `cafF`, `caff`+1 in any single byte | `-1` |
| E1d | randomized 4-byte magics ≠ `caff` (fixed-seed sweep) | `-1` |
| E1e | 4-byte magic correct but nothing else present in the buffer (short buffer, over-allocated so the read is in-bounds) → falls through to E2 | `-2` for zero version |

### Sub-rows of E2 exercised explicitly

| # | trigger | expected |
|---|---------|----------|
| E2a | version `0x0000` | `-2` |
| E2b | version `0x0002` (one past the only valid value) | `-2` |
| E2c | version `0xFFFF` | `-2` |
| E2d | version `0x0100` (i.e. native-endian `1` — proves big-endian read) | `-2` |
| E2e | randomized versions ≠ 1 (fixed-seed sweep) | `-2` |
| E2f | version `0x0001` and `flags` set to any of `0x0000/0xFFFF/random` — `flags` is never read, so it must **not** change the result | proceeds past E2 |

### Sub-rows of E3 exercised explicitly

| # | trigger | expected |
|---|---------|----------|
| E3a | `desc.format_id` all zero | `-3` |
| E3b | `desc.format_id` = `4ami` (little-endian spelling of the literal) | `-3` |
| E3c | `desc.format_id` = `ima4` with one byte perturbed (all 4 positions × several deltas) | `-3` |
| E3d | `desc.format_id` = `alac`, `lpcm`, `ima5`, `IMA4` | `-3` |
| E3e | randomized `format_id` ≠ `ima4` (fixed-seed sweep) | `-3` |
| E3f | `desc` chunk appears **after** the `pakt` chunk and carries a bad `format_id` | `-3` |
| E3g | two `desc` chunks, the **last** one bad (last write wins, `desc` is overwritten each match) | `-3` |
| E3h | two `desc` chunks, the **last** one good, first bad | `0` |

## Generic FFI boundary rows

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| G1 | `ima_parse` | `info == NULL` but the parse fails at E1/E2/E3 — `info` is only written on the success path, so a NULL `info` is harmless for every error return | same error code, no write, no fault |
| G2 | `ima_parse` | `data` buffer whose start address is **unaligned** (offset 1..15 into an allocation) — the C compiles the struct field reads to plain x86-64 loads which tolerate misalignment | identical results to the aligned case |
| G3 | `ima_parse` | zero-length / oversized logical content: `data` chunk `size` field = `0`, `i64::MIN`, `i64::MAX`, `-1` | `info->size` = that value reinterpreted as `u64`; return `0` |
| G4 | `ima_parse` | out-of-range "enum" values across FFI: `chunk->type` is a plain `ima_u32_t` and the C compares it against exactly 3 FourCCs with no `default` rejection. Every other 32-bit value (including `0`, `0xFFFFFFFF`, `'desc'`±1, and randomized values) is a valid input meaning "skip this chunk". Must be skipped identically. | chunk skipped by `size` bytes, walk continues |
| G5 | `ima_parse` | negative `chunk->size` on a skipped chunk — the C advances `chunk = (u8*)&chunk[1] + chunk_size`, so the walk moves **backwards**. Tested with a layout that lands back on a valid chunk. | walk continues at the backwards address, identical result |
| G6 | `ima_parse` | `channels_per_frame` = `0`, `1`, `0xFFFFFFFF` (no range check in C) | copied through verbatim |
| G7 | `ima_parse` | `pakt->frame_count` = `0`, `-1`, `i64::MIN`, `i64::MAX` (no range check in C) | copied through verbatim as `u64` |
| G8 | `ima_parse` | `desc->sample_rate` = NaN / ±Inf / negative / `≥ 2^63` / subnormal / `0.0` / `-0.0`. The C does an **arithmetic** `double → unsigned long long` conversion (`conv64.u = desc->sample_rate;`), byte-swaps the integer, then reads it back as a `double`. All out-of-range cases are C-UB but have a definite x86-64 codegen. | identical 64-bit pattern in `info->sample_rate` |

## Documented-UB rows (NOT executed in-process — they fault or hang in **both**
implementations, which is itself the matching behaviour)

| # | trigger | C behaviour | Rust behaviour |
|---|---------|-------------|----------------|
| U1 | `data == NULL` | reads `((caf_header*)0)->type` → SIGSEGV | same read → SIGSEGV |
| U2 | valid header, chunk walk never finds a `data` chunk | walks off the buffer until it faults or loops forever | identical pointer walk |
| U3 | `data` chunk reached but no `desc` chunk was seen (`desc == NULL`) | `desc->format_id` dereferences NULL → SIGSEGV | identical NULL deref → SIGSEGV |
| U4 | `data` chunk reached, `desc` valid and `ima4`, but no `pakt` chunk (`pakt == NULL`) | `pakt->frame_count` dereferences NULL → SIGSEGV. Note `info->blocks`/`info->size` are written **first**. | identical |

These are asserted structurally (both `.so`s crash the same way) by
`tests/errors.rs::ub_rows_are_documented_not_executed`, which only records that
they are deliberately excluded; running them in-process would abort the test
binary. U1/U3 are additionally verified out-of-process in
`tests/errors.rs::ub_null_deref_faults_in_both` via forked child processes that
compare the termination signal.

## Divergence found and fixed during Phase C

The UB rows initially **failed**: with the C `.so` raising `SIGSEGV` on the
NULL dereferences, the dev-profile Rust `.so` raised `SIGABRT` instead. Cause:
Rust's optional UB checks (`-C debug-assertions`, on by default in the `dev`
profile) turn the `load::<T>()` NULL dereference into a panic, and a panic
escaping an `extern "C"` function aborts.

Fix (in `Cargo.toml`, not in the translation logic):

```toml
[profile.dev]
debug-assertions = false
overflow-checks = false
```

`ima_parse` deliberately reproduces the C's unchecked dereferences of `data`,
`desc` and `pakt`, so those checks have to be off for the artifact to behave
like the C one. The `release` profile already had them off, which is why only
the dev build diverged. `[profile.test]` re-enables them so the test harness
keeps its own assertions. After the change all three UB rows report the same
signal (11) from both libraries.

## Row status

| row(s) | test | status |
|---|---|---|
| E1, E1a–E1e | `e1a_zero_magic`, `e1b_little_endian_spelling_of_the_literal`, `e1c_near_miss_magics` (all 1020 single-byte perturbations), `e1d_randomized_magics` (2048), `e1e_magic_is_checked_before_everything_else` | [x] |
| E2, E2a–E2f | `e2a`…`e2d`, `e2e_all_versions_rejected` (**exhaustive over all 65 535 invalid u16 versions**), `e2f_flags_do_not_affect_the_version_check` | [x] |
| E3, E3a–E3h | `e3a`…`e3e` (1024 randomized `format_id`s), `e3f_desc_after_pakt_with_bad_format`, `e3g_duplicate_desc_last_is_bad`, `e3h_duplicate_desc_last_is_good` | [x] |
| E4 | every valid-path row in `CONFIGS.md` | [x] |
| G1 | `g1_null_info_on_all_error_paths` | [x] |
| G2 | `g2_error_paths_with_unaligned_buffers` (offsets 0..15 × all three codes) | [x] |
| G3 | `g3_minimal_and_truncated_buffers`, plus `c12`/`c13`/`c14` for the size extremes | [x] |
| G4 | `g4_out_of_range_chunk_types_before_a_failing_format_check`, `g4b_out_of_range_versions_and_formats_combined` (check precedence: magic > version > format), `c34`, `c35` | [x] |
| G5 | `g5_negative_skip_then_error`, `c08_negative_chunk_size_walks_backwards` | [x] |
| G6 | `c25_channel_count_axis` | [x] |
| G7 | `c26_frame_count_axis` | [x] |
| G8 | `c15`–`c24b` | [x] |
| U1, U3, U4 | `u1_u3_u4_null_derefs_fault_identically_in_both` (forked child processes, compares the termination signal) | [x] |
| U2 | `u2_unterminated_chunk_walk_is_documented_only` (fixture-only; not reproducibly observable) | [x] documented |
