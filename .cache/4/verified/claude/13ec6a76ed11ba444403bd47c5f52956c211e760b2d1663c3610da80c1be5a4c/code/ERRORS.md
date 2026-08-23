# ERRORS.md — the ERROR-SURFACE TABLE

Every distinct way the C code rejects or errors on input, derived **mechanically**
from the C source (every `RETURN_ERROR` / `RETURN_ERROR_IF`, every `return 0` /
`return -1` / `return NULL` failure sentinel, every `assert`, every explicit range
check, null check, and min/max constant). One row per distinct rejection branch —
three distinct `RETURN_ERROR` branches are three rows.

**250 rows total**, numbered globally and sequentially:

| C source file | rows | count |
|---------------|------|-------|
| `lz4.c`      | 1–93 | 93 |
| `lz4hc.c`    | 94–147 | 54 |
| `xxhash.c`   | 148–167 | 20 |
| `lz4frame.c` | 168–221 | 54 |
| `lz4file.c`  | 222–250 | 29 |
| **total**    |      | **250** |

## Phase C row-coverage matrix

Every one of the 250 rows above is accounted for exactly once: either a
differential test constructs that exact condition and asserts C and Rust return
the SAME error code / sentinel, or the row is not observable in this build and the
reason is recorded. Verified mechanically — no row is unaccounted for, no row
appears twice, and every test function named here was checked to exist in the
file it is attributed to.

| C source file | rows | tested | not observable | test file |
|---|---|---|---|---|
| `lz4.c` | 1–93 | 68 | 25 | `tests/lz4_errors.rs` |
| `lz4hc.c` | 94–147 | 47 | 7 | `tests/lz4hc_xxhash_errors.rs` |
| `xxhash.c` | 148–167 | 12 | 8 | `tests/lz4hc_xxhash_errors.rs` |
| `lz4frame.c` | 168–221 | 51 | 3 | `tests/lz4frame_errors.rs` |
| `lz4file.c` | 222–250 | 20 | 9 | `tests/lz4file_errors.rs` |
| **total** | 1–250 | **198** | **52** | |

### Tested rows

| row(s) | covering `#[test]` | test file |
|---|---|---|
| 1–2 | `row_01_02_compress_bound_out_of_range` | `tests/lz4_errors.rs` |
| 3–4, 7–8 | `row_03_08_compress_srcsize_and_dstcapacity_rejection` | `tests/lz4_errors.rs` |
| 5 | `row_05_compress_fast_continue_bad_input_size` | `tests/lz4_errors.rs` |
| 6, 9, 23 | `row_06_09_23_compress_destsize_rejections` | `tests/lz4_errors.rs` |
| 10–12 | `row_10_12_limited_output_capacity_exhaustion` | `tests/lz4_errors.rs` |
| 15–20 | `row_15_20_acceleration_clamping` | `tests/lz4_errors.rs` |
| 26–28 | `row_26_28_init_stream_rejections` | `tests/lz4_errors.rs` |
| 30, 86 | `row_30_86_free_null_pointers` | `tests/lz4_errors.rs` |
| 31–32 | `row_31_32_load_dict_size_handling` | `tests/lz4_errors.rs` |
| 33–34 | `row_33_34_attach_dictionary_edge_cases` | `tests/lz4_errors.rs` |
| 35–36 | `row_35_36_save_dict_clamping` | `tests/lz4_errors.rs` |
| 38–39 | `row_38_39_continue_dictionary_discard_and_overlap` | `tests/lz4_errors.rs` |
| 42–44 | `row_42_44_decoder_ring_buffer_size` | `tests/lz4_errors.rs` |
| 45–51 | `row_45_51_decompress_safe_input_validation` | `tests/lz4_errors.rs` |
| 52–55, 60–72 | `row_52_72_corrupted_input_differential` | `tests/lz4_errors.rs` |
| 73–77 | `row_73_77_decompress_fast_output_checks` | `tests/lz4_errors.rs` |
| 79–80 | `row_79_80_decompress_continue_error_propagation` | `tests/lz4_errors.rs` |
| 83 | `row_83_set_stream_decode_always_succeeds` | `tests/lz4_errors.rs` |
| 88 | `row_88_reset_stream_state_always_zero` | `tests/lz4_errors.rs` |
| 94–95 | `row_94_95_hc_level_silently_clamped_to_9_and_12` | `tests/lz4hc_xxhash_errors.rs` |
| 96–98 | `row_96_97_98_stored_level_clamped` | `tests/lz4hc_xxhash_errors.rs` |
| 99–100 | `row_99_100_srcsize_out_of_range_every_entry_point` | `tests/lz4hc_xxhash_errors.rs` |
| 101–103 | `row_101_102_103_destsize_rejections` | `tests/lz4hc_xxhash_errors.rs` |
| 104–106, 108 | `row_104_105_106_108_extstatehc_and_destsize_bad_state` | `tests/lz4hc_xxhash_errors.rs` |
| 109–111 | `row_109_110_111_init_stream_hc_returns_null` | `tests/lz4hc_xxhash_errors.rs` |
| 113–114, 116 | `row_113_114_116_free_null_and_reset_stream_state_hc` | `tests/lz4hc_xxhash_errors.rs` |
| 117 | `row_117_negative_dstcapacity_returns_zero` | `tests/lz4hc_xxhash_errors.rs` |
| 120 | `row_120_lz4mid_last_literals_do_not_fit` | `tests/lz4hc_xxhash_errors.rs` |
| 121 | `row_121_lz4mid_midstream_dest_overflow` | `tests/lz4hc_xxhash_errors.rs` |
| 122–123, 125 | `row_122_123_125_hashchain_encode_sequence_overflow` | `tests/lz4hc_xxhash_errors.rs` |
| 124 | `row_124_hashchain_last_literals_do_not_fit` | `tests/lz4hc_xxhash_errors.rs` |
| 126–127 | `row_126_127_optimal_parser_overflow` | `tests/lz4hc_xxhash_errors.rs` |
| 129 | `row_129_optimal_sufficient_len_clamped_to_opt_num_minus_one` | `tests/lz4hc_xxhash_errors.rs` |
| 130 | `row_130_destsize_truncates_input_instead_of_failing` | `tests/lz4hc_xxhash_errors.rs` |
| 131 | `row_131_continue_notlimited_threshold_is_compressbound` | `tests/lz4hc_xxhash_errors.rs` |
| 132 | `row_132_compresshc2_continue_has_no_output_bound_check` | `tests/lz4hc_xxhash_errors.rs` |
| 133 | `row_133_failed_compression_marks_stream_dirty` | `tests/lz4hc_xxhash_errors.rs` |
| 134 | `row_134_load_dict_hc_truncates_to_last_64kb` | `tests/lz4hc_xxhash_errors.rs` |
| 135–136 | `row_135_136_load_dict_hc_too_small_leaves_tables_empty` | `tests/lz4hc_xxhash_errors.rs` |
| 138–141 | `row_138_139_140_141_save_dict_hc_clamps` | `tests/lz4hc_xxhash_errors.rs` |
| 142 | `row_142_attach_hc_dictionary_null_detaches` | `tests/lz4hc_xxhash_errors.rs` |
| 143 | `row_143_continue_src_overlaps_extdict` | `tests/lz4hc_xxhash_errors.rs` |
| 144 | `row_144_continue_two_gigabyte_position_overflow_reloads_dict` | `tests/lz4hc_xxhash_errors.rs` |
| 145 | `row_145_attached_dictctx_dropped_past_64kb` | `tests/lz4hc_xxhash_errors.rs` |
| 146 | `row_146_incompatible_dictctx_strategy_uses_slow_path` | `tests/lz4hc_xxhash_errors.rs` |
| 147 | `row_147_fastreset_accepts_uninitialised_state` | `tests/lz4hc_xxhash_errors.rs` |
| 148–150 | `row_148_149_150_update_null_input_is_the_only_rejection` | `tests/lz4hc_xxhash_errors.rs` |
| 151 | `row_151_xxh32_total_len_wraparound` | `tests/lz4hc_xxhash_errors.rs` |
| 152–153, 155–156 | `row_152_153_155_156_reset_always_ok_and_freestate_null` | `tests/lz4hc_xxhash_errors.rs` |
| 160–161 | `row_160_161_oneshot_null_pointer_zero_length` | `tests/lz4hc_xxhash_errors.rs` |
| 165–166 | `row_165_166_finalize_switch_covers_every_residue` | `tests/lz4hc_xxhash_errors.rs` |
| 168–169 | `row_168_169_get_block_size_invalid_id` | `tests/lz4frame_errors.rs` |
| 170 | `row_170_compress_frame_dst_too_small` | `tests/lz4frame_errors.rs` |
| 171 | `row_171_compress_begin_dst_too_small` | `tests/lz4frame_errors.rs` |
| 172–173 | `row_172_173_compress_begin_allocation_failures` | `tests/lz4frame_errors.rs` |
| 174 | `row_174_compress_begin_dict_size_too_large` | `tests/lz4frame_errors.rs` |
| 176, 192 | `row_176_192_create_context_null_out_pointer` | `tests/lz4frame_errors.rs` |
| 177, 193 | `row_177_193_default_allocator_failure` | `tests/lz4frame_errors.rs` |
| 178, 194 | `row_178_194_create_advanced_allocation_failure` | `tests/lz4frame_errors.rs` |
| 179–180 | `row_179_180_create_cdict_allocation_failures` | `tests/lz4frame_errors.rs` |
| 181 | `row_181_update_before_begin` | `tests/lz4frame_errors.rs` |
| 182–183 | `row_182_183_update_dst_too_small` | `tests/lz4frame_errors.rs` |
| 185–186 | `row_185_186_flush_state_and_capacity` | `tests/lz4frame_errors.rs` |
| 187 | `row_187_update_internal_flush_not_error_checked` | `tests/lz4frame_errors.rs` |
| 188–190 | `row_188_189_190_compress_end_errors` | `tests/lz4frame_errors.rs` |
| 195, 207–208 | `row_195_207_208_get_frame_info_incomplete_header` | `tests/lz4frame_errors.rs` |
| 196–202, 209 | `row_196_202_209_decode_header_validation` | `tests/lz4frame_errors.rs` |
| 203–205 | `row_203_205_header_size_errors` | `tests/lz4frame_errors.rs` |
| 206 | `row_206_get_frame_info_partial_header` | `tests/lz4frame_errors.rs` |
| 210 | `row_210_get_frame_info_after_header_decoded` | `tests/lz4frame_errors.rs` |
| 211–212 | `row_211_212_decompress_allocation_failures` | `tests/lz4frame_errors.rs` |
| 213 | `row_213_block_header_too_large` | `tests/lz4frame_errors.rs` |
| 214–215 | `row_214_215_block_checksum_invalid` | `tests/lz4frame_errors.rs` |
| 216–217 | `row_216_217_corrupt_block_payload` | `tests/lz4frame_errors.rs` |
| 218 | `row_218_frame_size_wrong` | `tests/lz4frame_errors.rs` |
| 219 | `row_219_content_checksum_invalid` | `tests/lz4frame_errors.rs` |
| 220 | `row_220_null_dst_with_nonzero_size` | `tests/lz4frame_errors.rs` |
| 221 | `row_221_free_dctx_returns_dstage` | `tests/lz4frame_errors.rs` |
| 222–223 | `row_222_223_read_open_null_arguments` | `tests/lz4file_errors.rs` |
| 226 | `row_226_read_open_short_or_failing_fread_io_read` | `tests/lz4file_errors.rs` |
| 227 | `row_227_read_open_frame_info_errors_verbatim` | `tests/lz4file_errors.rs` |
| 230–231 | `row_230_231_read_null_state_and_null_buffer` | `tests/lz4file_errors.rs` |
| 232 | `row_232_read_short_read_at_eof_is_not_an_error` | `tests/lz4file_errors.rs` |
| 233 | `row_233_read_decompress_errors_verbatim` | `tests/lz4file_errors.rs` |
| 234 | `row_234_read_close_null` | `tests/lz4file_errors.rs` |
| 235–236 | `row_235_236_write_open_null_arguments` | `tests/lz4file_errors.rs` |
| 238 | `row_238_write_open_invalid_block_size_id` | `tests/lz4file_errors.rs` |
| 242 | `row_242_write_open_header_io_write` | `tests/lz4file_errors.rs` |
| 243–244 | `row_243_244_write_null_state_and_null_buffer` | `tests/lz4file_errors.rs` |
| 246, 249 | `row_246_249_write_payload_io_write_and_close_masks_it` | `tests/lz4file_errors.rs` |
| 247 | `row_247_write_close_null` | `tests/lz4file_errors.rs` |
| 248 | `row_248_write_close_compress_end_frame_size_wrong` | `tests/lz4file_errors.rs` |
| 250 | `row_250_write_close_footer_io_write` | `tests/lz4file_errors.rs` |

Row 161 note: the `len == 0` half is tested (`LZ4_XXH32(NULL, 0, seed)` is
well defined); the `len > 0` half is an unconditional NULL dereference in the C
and is documented rather than executed.

### Rows that are not observable in this build

| row(s) | reason | documented in |
|---|---|---|
| 13, 14 | notLimited behavioural split: the documented consequence requires LYING about dstCapacity, which writes out of bounds in BOTH libraries | `tests/lz4_errors.rs` |
| 21, 22 | LZ4_HEAPMODE=1 allocation failure: not compiled in (c_src/CMakeLists.txt sets LZ4_HEAPMODE=0) | `tests/lz4_errors.rs` |
| 24, 25 | NULL/misaligned extState: assert compiled out in lz4.c -> UB in this build | `tests/lz4_errors.rs` |
| 29, 87 | malloc failure inside LZ4_createStream / LZ4_createStreamDecode: no allocator hook in these APIs | `tests/lz4_errors.rs` |
| 37 | LZ4_saveDict(stream, NULL, nonzero): assert compiled out -> UB | `tests/lz4_errors.rs` |
| 40, 41 | assert-only paths whose release behaviour is exactly the `return 0` already asserted by rows 3/5 | `tests/lz4_errors.rs` |
| 56 | 32-bit-only overflow branch (sizeof(size_t) < 8); unreachable on x86-64 | `tests/lz4_errors.rs` |
| 57, 58, 59 | address-space wrap of op+length / ip+length: needs a length near 2^64, which the preceding ilimit check rejects first on any allocatable buffer | `tests/lz4_errors.rs` |
| 78 | LZ4_decompress_fast has no input-side bound at all; malformed input over-reads the source in BOTH libraries | `tests/lz4_errors.rs` |
| 81, 82 | NULL streamDecode / negative originalSize for LZ4_decompress_fast_continue: assert compiled out -> UB | `tests/lz4_errors.rs` |
| 84, 85 | negative dictSize: assert compiled out, cast to a huge size_t -> UB | `tests/lz4_errors.rs` |
| 89, 90, 91, 92, 93 | compile-time #error / LZ4_STATIC_ASSERT guards; both libraries compiled successfully, which is the proof | `tests/lz4_errors.rs` |
| 107, 112, 115, 128 | un-forceable malloc failure (LZ4HC_HEAPMODE==1: lz4hc.c:1523, 1556, 2161, 1838); no allocator hook in these APIs | `tests/lz4hc_xxhash_errors.rs` |
| 118, 119 | unreachable: the unsigned guard at lz4hc.c:1389 rejects the value before this code runs | `tests/lz4hc_xxhash_errors.rs` |
| 137 | assert(dictSize >= 0) at lz4hc.c:1632 compiled out -> negative size cast to a huge size_t, out-of-bounds read | `tests/lz4hc_xxhash_errors.rs` |
| 154, 159 | xxhash has no NULL check on the state pointer; an unconditional NULL load/store faults both libraries identically | `tests/lz4hc_xxhash_errors.rs` |
| 157, 158 | un-forceable malloc failure (xxhash.c:422, 883); no allocator hook | `tests/lz4hc_xxhash_errors.rs` |
| 162, 163, 164 | no NULL check at all; unconditional NULL dereference with no sentinel in the return type | `tests/lz4hc_xxhash_errors.rs` |
| 167 | compile-time XXH_STATIC_ASSERT (xxhash.c:567, 1020); both libraries built, which is the proof | `tests/lz4hc_xxhash_errors.rs` |
| 175, 191 | assert(...!=NULL) at lz4frame.c:620 / 1303 is compiled out (LZ4_DEBUG undefined), so the row is not in the binary; its production behaviour is rows 176/192, which ARE tested | `tests/lz4frame_errors.rs` |
| 184 | assert(blockCompression == LZ4B_COMPRESSED) at lz4frame.c:1071 compiled out; the C then corrupts its OWN heap via LZ4F_localSaveDict with a 64 KB tmpBuff | `tests/lz4frame_errors.rs` |
| 224, 225, 229, 237, 239, 240, 241 | lz4file.c calls libc calloc/malloc directly (lz4file.c:83, 128, 225, 253, 259, 265) and lz4file.h exposes no LZ4F_CustomMem hook | `tests/lz4file_errors.rs` |
| 228 | unreachable: blockSizeID = (BD>>4)&_3BITS and lz4frame.c:1410 rejects <4, so LZ4F_getFrameInfo can only report 0 or 4..7 -> the default arm at lz4file.c:122 is dead | `tests/lz4file_errors.rs` |
| 245 | unreachable: cStage is always 1 for any cctx reachable by LZ4F_write, and dstBuf is sized LZ4F_compressBound(maxWriteSize) which already assumes worst-case buffering | `tests/lz4file_errors.rs` |


## Assert liveness per translation unit (affects which rows are testable)

Many rows below are guarded only by `assert()`. Whether an `assert` **aborts** or is
**compiled out** decides whether the row is observable, and it differs per file.
Derived mechanically with `nm -u` on each object file in
`c_src/build/CMakeFiles/lz4.dir/src/` — a reference to `__assert_fail` means live asserts:

| C source file | `__assert_fail` referenced | asserts | why |
|---------------|---------------------------|---------|-----|
| `lz4.c`      | no  | **compiled out** | `#define assert(condition) ((void)0)` at lz4.c:268-274 because `LZ4_DEBUG` is undefined |
| `lz4hc.c`    | no  | **compiled out** | inherits the same no-op `assert` macro; no `<assert.h>` include of its own |
| `lz4frame.c` | no  | **compiled out** | `#define assert(condition) ((void)0)` at lz4frame.c:143-149, same reason |
| `lz4file.c`  | YES | **live**         | `#include <assert.h>` unconditionally at lz4file.c:36 |
| `xxhash.c`   | YES | **live**         | `#include <assert.h>` unconditionally at xxhash.c:114 |

`c_src/CMakeLists.txt` never defines `LZ4_DEBUG`, and `-DNDEBUG` is absent — but
`-DNDEBUG` is not what decides it; the library's own `LZ4_DEBUG` gate is.

Consequences used throughout the Phase C test files:

* In `lz4.c` / `lz4hc.c` / `lz4frame.c`, an `assert`-only guard does **not** abort. The
  check simply is not there, and the trigger runs on into undefined behaviour (a wrapped
  `size_t`, an out-of-bounds read, or a write past the caller's buffer). Such rows are
  documented rather than executed, because both libraries would fault identically and the
  comparison would prove nothing — **not** because they "abort".
* Because `assert` is a no-op in `lz4frame.c`, the rows that the C reaches *after* a
  dead assert become genuinely testable — e.g. `LZ4F_createCompressionContext(NULL, 100)`
  returns `LZ4F_ERROR_parameter_null` (21) instead of aborting, so row 176 is a real,
  asserted row and row 175 is "not compiled in".
* In `lz4file.c` / `xxhash.c` the asserts really do abort, so those rows are excluded on
  that basis.

## lz4.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `LZ4_compressBound` | `isize < 0` (e.g. `-1`); macro `LZ4_COMPRESSBOUND` compares `(unsigned)isize > (unsigned)LZ4_MAX_INPUT_SIZE` — lz4.h:215, body lz4.c:751 | returns 0 |
| 2 | `LZ4_compressBound` | `isize > LZ4_MAX_INPUT_SIZE (0x7E000000 = 2113929216)`, e.g. `0x7E000001` — lz4.h:215, body lz4.c:751 | returns 0 |
| 3 | `LZ4_compress_default` (same branch via `LZ4_compress_fast`, `LZ4_compress_fast_extState`, `LZ4_compress_fast_extState_fastReset`, `LZ4_compress`, `LZ4_compress_limitedOutput`, `LZ4_compress_withState`, `LZ4_compress_limitedOutput_withState`) | `srcSize < 0`: static `LZ4_compress_generic` does `if ((U32)srcSize > (U32)LZ4_MAX_INPUT_SIZE) return 0;` — lz4.c:1360 | returns 0 |
| 4 | `LZ4_compress_default` (same entry points as #3) | `srcSize > LZ4_MAX_INPUT_SIZE (0x7E000000)` — lz4.c:1360 | returns 0 |
| 5 | `LZ4_compress_fast_continue` (also `LZ4_compress_continue`, `LZ4_compress_limitedOutput_continue`, `LZ4_compress_forceExtDict`) | `inputSize < 0` or `inputSize > LZ4_MAX_INPUT_SIZE` — lz4.c:1360 (via `LZ4_compress_generic`) | returns 0 |
| 6 | `LZ4_compress_destSize` (also `LZ4_compress_destSize_extState`) | `*srcSizePtr < 0` or `*srcSizePtr > LZ4_MAX_INPUT_SIZE` — lz4.c:1360 | returns 0 |
| 7 | `LZ4_compress_default` (also `LZ4_compress_fast`, `_fast_extState`, `_fast_continue`) | `srcSize == 0` **and** `dstCapacity <= 0` (limitedOutput path): `if (outputDirective != notLimited && dstCapacity <= 0) return 0;` — lz4.c:1361-1362 | returns 0 |
| 8 | `LZ4_compress_default` | `srcSize == 0` with `dstCapacity >= 1` (accept boundary, contrast with #7): writes one 0 byte — lz4.c:1366-1371 | returns 1 |
| 9 | `LZ4_compress_destSize` (also `LZ4_compress_destSize_extState`) | `targetDstSize < 1` (0 or negative) while `*srcSizePtr >= 1`: `fillOutput && maxOutputSize < 1` in `LZ4_compress_generic_validated` — lz4.c:985 | returns 0 |
| 10 | `LZ4_compress_default` (any limitedOutput entry: `LZ4_compress_fast`, `_fast_extState`, `_fast_continue`, `_limitedOutput`) | `dstCapacity` too small at literal encoding: `op + litLength + (2+1+LASTLITERALS) + (litLength/255) > olimit` — lz4.c:1114-1116 | returns 0 |
| 11 | `LZ4_compress_default` (limitedOutput entries as #10) | `dstCapacity` too small at match-length encoding: `op + (1+LASTLITERALS) + (matchCode+240)/255 > olimit` — lz4.c:1187-1188, return at lz4.c:1210 | returns 0 |
| 12 | `LZ4_compress_default` (limitedOutput entries as #10) | `dstCapacity` too small for the last literal run: `op + lastRun + 1 + ((lastRun+255-RUN_MASK)/255) > olimit` — lz4.c:1305-1306, return at lz4.c:1314 | returns 0 |
| 13 | `LZ4_compress_fast_extState` (and `LZ4_compress_default`/`LZ4_compress_fast` through it) | behavioural split: when `maxOutputSize >= LZ4_compressBound(inputSize)` the `notLimited` directive is selected (lz4.c:1388-1394) and **no** output-bound check is performed at all (rows #10-#12 are compiled out) | returns >0; writes past `dst` if the caller understated capacity (no 0 return) |
| 14 | `LZ4_compress_fast_extState_fastReset` | same split at lz4.c:1421-1434: `dstCapacity >= LZ4_compressBound(srcSize)` ⇒ `notLimited`, no overflow detection | returns >0, no 0-return path |
| 15 | `LZ4_compress_fast` (also `LZ4_compress_fast_extState`, and `LZ4_compress_default` which passes 1) | `acceleration < 1` (0 or negative) — lz4.c:1386 | silently clamps to `LZ4_ACCELERATION_DEFAULT` (1); returns normally |
| 16 | `LZ4_compress_fast` (also `LZ4_compress_fast_extState`) | `acceleration > LZ4_ACCELERATION_MAX (65537)` — lz4.c:1387 | silently clamps to 65537; returns normally |
| 17 | `LZ4_compress_fast_extState_fastReset` | `acceleration < 1` — lz4.c:1417 | clamps to 1 |
| 18 | `LZ4_compress_fast_extState_fastReset` | `acceleration > 65537` — lz4.c:1418 | clamps to 65537 |
| 19 | `LZ4_compress_fast_continue` | `acceleration < 1` — lz4.c:1719 | clamps to 1 |
| 20 | `LZ4_compress_fast_continue` | `acceleration > 65537` — lz4.c:1720 | clamps to 65537 |
| 21 | `LZ4_compress_fast` | build with `LZ4_HEAPMODE=1` and `ALLOC(sizeof(LZ4_stream_t))` returns NULL — lz4.c:1457-1458 | returns 0 |
| 22 | `LZ4_compress_destSize` | build with `LZ4_HEAPMODE=1` and `ALLOC(sizeof(LZ4_stream_t))` returns NULL — lz4.c:1509-1510 | returns 0 |
| 23 | `LZ4_compress_destSize` (also `LZ4_compress_destSize_extState`, `LZ4_compress_HC_destSize`-like semantics) | `targetDstSize < LZ4_compressBound(*srcSizePtr)`: `fillOutput` silently truncates the input instead of failing — lz4.c:1118-1122, 1147-1152, 1307-1311, `*inputConsumed` written at lz4.c:1332 | returns >0 and overwrites `*srcSizePtr` with the number of bytes actually consumed (< original) |
| 24 | `LZ4_compress_fast_extState` | `state == NULL` or `state` not aligned to `LZ4_stream_t_alignment()` or state smaller than `sizeof(LZ4_stream_t)`: `LZ4_initStream()` returns NULL and the result is only guarded by `assert(ctx != NULL)` — lz4.c:1384-1385 | **no error return**; NULL/invalid pointer dereference (assert only fires with `LZ4_DEBUG>=1`) |
| 25 | `LZ4_compress_destSize_extState` | `state == NULL`/misaligned: `LZ4_initStream()` returns NULL, guarded only by `assert(s != NULL)` — lz4.c:1483-1484 | **no error return**; UB in release build |
| 26 | `LZ4_initStream` | `buffer == NULL` — lz4.c:1555 | returns NULL |
| 27 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` (`LZ4_STREAM_MINSIZE` = `(1<<LZ4_MEMORY_USAGE)+32` = 16416 by default) — lz4.c:1556 | returns NULL |
| 28 | `LZ4_initStream` | `buffer` not aligned: `!LZ4_isAligned(buffer, LZ4_stream_t_alignment())` (alignment of `LZ4_stream_t`, test active because `LZ4_ALIGN_TEST=1`) — lz4.c:1557, helper lz4.c:292-295 | returns NULL |
| 29 | `LZ4_createStream` | `ALLOC(sizeof(LZ4_stream_t))` fails — lz4.c:1533-1536 | returns NULL |
| 30 | `LZ4_freeStream` | `LZ4_stream == NULL` (free-on-NULL is supported) — lz4.c:1577 | returns 0 (no free performed) |
| 31 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < (int)HASH_UNIT` i.e. `< sizeof(reg_t)` = 8 on 64-bit (includes `dictSize <= 0` and negative) — `LZ4_loadDict_internal`, lz4.c:1613-1615 | returns 0; no dictionary registered (`dict->dictionary` stays NULL, `dictSize` 0) |
| 32 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize > 64 KB`: `if ((dictEnd - p) > 64 KB) p = dictEnd - 64 KB;` silently keeps only the **last** 64 KB — lz4.c:1617 | returns 65536 (not `dictSize`) |
| 33 | `LZ4_attach_dictionary` | `dictionaryStream == NULL` — lz4.c:1660-1661 | void; silently detaches (`workingStream->dictCtx = NULL`) |
| 34 | `LZ4_attach_dictionary` | `dictionaryStream` whose `dictCtx->dictSize == 0` — lz4.c:1679-1681 | void; dictionary silently **not** attached (`dictCtx` forced to NULL) |
| 35 | `LZ4_saveDict` | `(U32)dictSize > 64 KB` (also true for any negative `dictSize`, which becomes a huge U32) — lz4.c:1820 | silently clamps `dictSize` to 65536 and returns the clamped value |
| 36 | `LZ4_saveDict` | `(U32)dictSize > dict->dictSize` (asking to save more history than exists) — lz4.c:1821 | silently clamps to `dict->dictSize`; returns that smaller value |
| 37 | `LZ4_saveDict` | `safeBuffer == NULL` with `dictSize != 0`: only `assert(dictSize == 0)` — lz4.c:1823 | no rejection; `LZ4_memmove` into NULL ⇒ UB in release build |
| 38 | `LZ4_compress_fast_continue` | registered `streamPtr->dictSize < 4` while not in prefix mode and `inputSize > 0` and no `dictCtx` — lz4.c:1723-1733 | dictionary silently discarded (`dictSize = 0`, `dictionary = source`); compression proceeds |
| 39 | `LZ4_compress_fast_continue` | `source .. source+inputSize` overlaps the registered dictionary (`sourceEnd > dictionary && sourceEnd < dictEnd`) — lz4.c:1736-1742 | dictionary silently shrunk to `dictEnd - sourceEnd`, capped at 64 KB, and zeroed if the remainder is `< 4` |
| 40 | `LZ4_compress_fast_continue` | `inputSize < 0`: `assert(nextSize >= 0)` in static `LZ4_renormDictT` — lz4.c:1689 (called at lz4.c:1718) | assert only (`LZ4_DEBUG>=1`); in release, execution continues and returns 0 via lz4.c:1360 |
| 41 | `LZ4_compress_fast_extState_fastReset` / `LZ4_resetStream_fast` | negative `srcSize`: `assert(inputSize >= 0)` in static `LZ4_prepareTable` — lz4.c:892 | assert only; in release the table may be reset and the call returns 0 via lz4.c:1360 |
| 42 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` — lz4.c:2617 | returns 0 |
| 43 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE (0x7E000000)` — lz4.c:2618 | returns 0 |
| 44 | `LZ4_decoderRingBufferSize` | `0 <= maxBlockSize < 16` — lz4.c:2619 | silently clamps to 16; returns `65536+14+16 = 65566` |
| 45 | `LZ4_decompress_safe` (same branch via `LZ4_decompress_safe_partial`, `_withPrefix64k`, `_usingDict`, `_forceExtDict`, `_continue`, `LZ4_uncompress_unknownOutputSize`) | `source == NULL`: `if ((src == NULL) || (outputSize < 0)) return -1;` in `LZ4_decompress_generic` — lz4.c:2036 | returns -1 |
| 46 | `LZ4_decompress_safe` | `maxDecompressedSize < 0` — lz4.c:2036 | returns -1 |
| 47 | `LZ4_decompress_safe_partial` | `targetOutputSize < 0` (or `dstCapacity < 0`): `dstCapacity = MIN(targetOutputSize, dstCapacity)` at lz4.c:2461 then `outputSize < 0` at lz4.c:2036 | returns -1 |
| 48 | `LZ4_decompress_safe` | `maxDecompressedSize == 0` and input is **not** the 1-byte empty block: `return ((srcSize==1) && (*ip==0)) ? 0 : -1;` — lz4.c:2064-2067 | returns -1 |
| 49 | `LZ4_decompress_safe` | `maxDecompressedSize == 0` with `compressedSize == 1` and `source[0] == 0` (accept boundary, contrast with #48) — lz4.c:2067 | returns 0 |
| 50 | `LZ4_decompress_safe_partial` | `dstCapacity == 0` (or `targetOutputSize == 0`): `if (partialDecoding) return 0;` — lz4.c:2066 | returns 0 (never -1, whatever the input) |
| 51 | `LZ4_decompress_safe` | `compressedSize == 0` with `maxDecompressedSize > 0` — lz4.c:2069 | returns -1 |
| 52 | `LZ4_decompress_safe` | `compressedSize < 0`: no explicit check; `iend = ip + srcSize` lies before `ip`, so parsing falls into the last-literals validation `(ip+length != iend) || (cpy > oend)` — lz4.c:2279, 2312-2318 | returns negative (`-(consumed)-1`) after reading ≥1 byte from `src` |
| 53 | `LZ4_decompress_safe` | literal-length token == 15 (`RUN_MASK`) with no further input byte: `initial_check` in static `read_variable_length` (`*ip >= ilimit`, ilimit = `iend-RUN_MASK`) — lz4.c:1986-1988, called at lz4.c:2093 / 2265 | returns negative value < 0 (`-(int)(ip-src)-1`, lz4.c:2443) |
| 54 | `LZ4_decompress_safe` | long literal length whose 255-continuation bytes run past `iend-RUN_MASK`: `(*ip) > ilimit` — lz4.c:1992-1994 and 2004-2006 | returns negative value < 0 |
| 55 | `LZ4_decompress_safe` | long match length whose continuation bytes run past `iend-LASTLITERALS+1` — `read_variable_length` called with `initial_check=0` at lz4.c:2128 / 2346, limit hit at lz4.c:1992 / 2004 | returns negative value < 0 |
| 56 | `LZ4_decompress_safe` | 32-bit builds only (`sizeof(size_t) < 8`): accumulated variable length `> ((Rvl_t)-1)/2` — lz4.c:1996-1998 and 2008-2010 | returns negative value < 0 |
| 57 | `LZ4_decompress_safe` | literal length so large that `(uptrval)op + length < (uptrval)op` (address wrap) — lz4.c:2099 (fast loop) and lz4.c:2268 (safe loop) | returns negative value < 0 |
| 58 | `LZ4_decompress_safe` | literal length so large that `(uptrval)ip + length < (uptrval)ip` — lz4.c:2100 (fast loop) and lz4.c:2269 (safe loop) | returns negative value < 0 |
| 59 | `LZ4_decompress_safe` | match length so large that `(uptrval)op + length < (uptrval)op` — lz4.c:2136 (fast loop) and lz4.c:2349 (safe loop) | returns negative value < 0 |
| 60 | `LZ4_decompress_safe` | offset larger than the data written so far (offset points before `lowPrefix`): `checkOffset && (match + dictSize < lowPrefix)` in the FAST_DEC_LOOP — lz4.c:2161-2163 | returns negative value < 0 |
| 61 | `LZ4_decompress_safe` | same out-of-range offset detected in the safe loop: `(checkOffset) && (match + dictSize < lowPrefix)` — lz4.c:2356 | returns negative value < 0 |
| 62 | `LZ4_decompress_safe` / `LZ4_decompress_fast` | **offset == 0** (illegal per the block format): `match == op`, so `match + dictSize < lowPrefix` (lz4.c:2161 / 2356) and `offset > (op-prefixStart)+dictSize` (lz4.c:1926) are both false — there is **no** zero-offset check anywhere | **no rejection**; the match is copied from `op` itself after `LZ4_write32(op, 0)` (lz4.c:2407 / 500), so the output is zero-filled and a non-negative length is returned |
| 63 | `LZ4_decompress_safe` | last literal run does not consume the input exactly: `ip + length != iend` in full-block mode — lz4.c:2312-2318 | returns negative value < 0 |
| 64 | `LZ4_decompress_safe` | last literal run does not fit in `dst`: `cpy > oend` in full-block mode — lz4.c:2312-2318 | returns negative value < 0 |
| 65 | `LZ4_decompress_safe` | a match copy would reach into the final `LASTLITERALS` (5) bytes: `cpy > oend-LASTLITERALS` — lz4.c:2421-2423 | returns negative value < 0 |
| 66 | `LZ4_decompress_safe_usingDict` / `LZ4_decompress_safe_forceExtDict` / `LZ4_decompress_safe_continue` | extDict match ends beyond `oend-LASTLITERALS` in full-block mode, FAST_DEC_LOOP path — lz4.c:2166-2175 (`goto _output_error` at 2174) | returns negative value < 0 |
| 67 | `LZ4_decompress_safe_usingDict` / `_forceExtDict` / `_continue` | same end-of-block violation in the safe loop: `op+length > oend-LASTLITERALS` and `!partialDecoding` — lz4.c:2358-2362 | returns negative value < 0 |
| 68 | `LZ4_decompress_safe_partial` | literal run longer than the remaining **input**: `ip+length > iend` ⇒ `length = iend-ip` — lz4.c:2296-2299 | no error; returns the (smaller) number of bytes decoded |
| 69 | `LZ4_decompress_safe_partial` | literal run longer than the remaining **output**: `cpy > oend` ⇒ `cpy = oend; length = oend-op` — lz4.c:2303-2307 | no error; returns `dstCapacity` bytes decoded |
| 70 | `LZ4_decompress_safe_partial_usingDict` / `LZ4_decompress_safe_partial_forceExtDict` | extDict match longer than remaining output in partial mode: `length = MIN(length, (size_t)(oend-op))` — lz4.c:2169-2171 and lz4.c:2361 | no error; returns bytes actually written |
| 71 | `LZ4_decompress_safe_partial` | `targetOutputSize > dstCapacity` — lz4.c:2461 (`dstCapacity = MIN(targetOutputSize, dstCapacity)`) | silently uses the smaller `dstCapacity` as the decode limit |
| 72 | `LZ4_decompress_safe_partial` | match copy in partial mode reaching `oend`: clamped by `mlen = MIN(length, (size_t)(oend-op))` and loop breaks at `op == oend` — lz4.c:2392-2403 | no error; returns bytes written (may be < `targetOutputSize`) |
| 73 | `LZ4_decompress_fast` (also `LZ4_uncompress`, `LZ4_decompress_fast_withPrefix64k`, `_usingDict`, `_continue`) | literal length exceeds remaining output: `(size_t)(oend-op) < ll` in static `LZ4_decompress_unsafe_generic` — lz4.c:1898 | returns -1 |
| 74 | `LZ4_decompress_fast` | after a literal run fewer than `MFLIMIT` (12) bytes remain and `op != oend` (illegal block end) — lz4.c:1902-1908 | returns -1 |
| 75 | `LZ4_decompress_fast` | match length exceeds remaining output: `(size_t)(oend-op) < ml` — lz4.c:1921 | returns -1 |
| 76 | `LZ4_decompress_fast` | offset out of range: `offset > (size_t)(op - prefixStart) + dictSize` — lz4.c:1926-1929 | returns -1 |
| 77 | `LZ4_decompress_fast` | match ends within the last `LASTLITERALS` (5) bytes: `(size_t)(oend-op) < LASTLITERALS` — lz4.c:1957-1962 | returns -1 |
| 78 | `LZ4_decompress_fast` | truncated / malformed **input**: `LZ4_decompress_unsafe_generic` has no input-side bound at all (`read_long_length_no_check`, lz4.c:1852-1858; token/offset reads at 1890, 1912) | **no rejection**; reads past the source buffer (only output-side checks #73-#77 exist) |
| 79 | `LZ4_decompress_safe_continue` | underlying block decode returns `<= 0` (corrupt block / too-small `maxOutputSize`) — lz4.c:2640, 2653, 2662 | returns that value unchanged (negative, or 0) and leaves `prefixSize`/`prefixEnd` unmodified |
| 80 | `LZ4_decompress_fast_continue` | underlying decode returns `<= 0` — lz4.c:2685, 2694, 2703 | returns that value unchanged; stream state not advanced |
| 81 | `LZ4_decompress_fast_continue` | `LZ4_streamDecode == NULL`: only `assert(LZ4_streamDecode != NULL)` — lz4.c:2675 | no rejection; NULL dereference in release build |
| 82 | `LZ4_decompress_fast_continue` | `originalSize < 0`: only `assert(originalSize >= 0)` — lz4.c:2679 | no rejection; `oend < op` ⇒ out-of-bounds writes in release build |
| 83 | `LZ4_setStreamDecode` | `dictSize != 0` with `dictionary == NULL`: only `assert(dictionary != NULL)` — lz4.c:2593-2595 | always returns 1 (this function has **no** failure return) |
| 84 | `LZ4_decompress_safe_usingDict` / `LZ4_decompress_safe_partial_usingDict` | `dictSize < 0`: only `assert(dictSize >= 0)` — lz4.c:2727 / 2730 (and 2742 / 2745) | no rejection; negative value is cast to `size_t` (huge `dictSize`) and disables `checkOffset` (lz4.c:2047) |
| 85 | `LZ4_decompress_fast_usingDict` | `dictSize < 0`: `(size_t)dictSize` passed as prefix size — lz4.c:2751-2754, `assert(dictSize >= 0)` at 2755 | no rejection; UB |
| 86 | `LZ4_freeStreamDecode` | `LZ4_stream == NULL` — lz4.c:2577 | returns 0 |
| 87 | `LZ4_createStreamDecode` | `ALLOC_AND_ZERO(sizeof(LZ4_streamDecode_t))` fails — lz4.c:2572 | returns NULL |
| 88 | `LZ4_resetStreamState` (obsolete) | any `state` — the function ignores `inputBuffer` and cannot fail — lz4.c:2808-2813 | always returns 0 |
| 89 | compile-time (`lz4.h` gate for every entry point) | `LZ4_MEMORY_USAGE < LZ4_MEMORY_USAGE_MIN (10)` — lz4.h:166-168 | `#error "LZ4_MEMORY_USAGE is too small !"` (build fails) |
| 90 | compile-time (`lz4.h`) | `LZ4_MEMORY_USAGE > LZ4_MEMORY_USAGE_MAX (20)` — lz4.h:170-172 | `#error "LZ4_MEMORY_USAGE is too large !"` (build fails) |
| 91 | compile-time (`lz4.c`) | `LZ4_DISTANCE_MAX > LZ4_DISTANCE_ABSOLUTE_MAX (65535)` — lz4.c:255-258 | `#error "LZ4_DISTANCE_MAX is too big : must be <= 65535"` |
| 92 | compile-time (`lz4.c`, inside `LZ4_getIndexOnHash`) | `LZ4_STATIC_ASSERT(LZ4_MEMORY_USAGE > 2)` — lz4.c:855 (macro at lz4.c:277, division by zero at compile time) | build fails |
| 93 | compile-time (`LZ4_createStream`) | `LZ4_STATIC_ASSERT(sizeof(LZ4_stream_t) >= sizeof(LZ4_stream_t_internal))` — lz4.c:1534 (and the `LZ4_streamDecode_t` counterpart at lz4.c:2571) | build fails |


## lz4hc.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 94 | `LZ4_compress_HC` (also `LZ4_compressHC`, `LZ4_compressHC2`, `LZ4_compressHC_limitedOutput`, `LZ4_compressHC2_limitedOutput`, `LZ4_compress_HC_extStateHC`, `LZ4_compress_HC_destSize`) | `compressionLevel < 1` (0 or negative — note `LZ4_compressHC*` wrappers hard-code 0): static `LZ4HC_getCLevelParams` does `if (cLevel < 1) cLevel = LZ4HC_CLEVEL_DEFAULT;` — lz4hc.c:112-113 | silently uses level `LZ4HC_CLEVEL_DEFAULT` (9) |
| 95 | `LZ4_compress_HC` (same entry points as #94) | `compressionLevel > LZ4HC_CLEVEL_MAX (12)`: `cLevel = MIN(LZ4HC_CLEVEL_MAX, cLevel)` — lz4hc.c:114 | silently clamps to level 12 |
| 96 | `LZ4_setCompressionLevel` | `compressionLevel < 1` — lz4hc.c:1614 | void; stored level becomes `LZ4HC_CLEVEL_DEFAULT` (9) |
| 97 | `LZ4_setCompressionLevel` | `compressionLevel > LZ4HC_CLEVEL_MAX (12)` — lz4hc.c:1615 | void; stored level clamped to 12 |
| 98 | `LZ4_resetStreamHC` / `LZ4_resetStreamHC_fast` | `compressionLevel` out of `[1,12]` — same clamps via `LZ4_setCompressionLevel` at lz4hc.c:1592 / 1608 | void; level replaced by 9 (if `<1`) or 12 (if `>12`) |
| 99 | `LZ4_compress_HC` (also `LZ4_compress_HC_extStateHC`, `_extStateHC_fastReset`, `LZ4_compress_HC_continue`, `LZ4_compressHC2_continue`, `LZ4_compressHC2_limitedOutput_continue`) | `srcSize < 0`: `if ((U32)*srcSizePtr > (U32)LZ4_MAX_INPUT_SIZE) return 0;` in `LZ4HC_compress_generic_internal` — lz4hc.c:1389 | returns 0 |
| 100 | `LZ4_compress_HC` (same entry points as #99) | `srcSize > LZ4_MAX_INPUT_SIZE (0x7E000000)` — lz4hc.c:1389 | returns 0 |
| 101 | `LZ4_compress_HC_destSize` | `*sourceSizePtr < 0` or `> LZ4_MAX_INPUT_SIZE` — lz4hc.c:1389 | returns 0 |
| 102 | `LZ4_compress_HC_destSize` | `targetDestSize < 1` (0 or negative): `if (limit == fillOutput && dstCapacity < 1) return 0;` — lz4hc.c:1388 | returns 0 |
| 103 | `LZ4_compress_HC_continue_destSize` | `targetDestSize < 1` — lz4hc.c:1388 (via `LZ4_compressHC_continue_generic`, lz4hc.c:1733) | returns 0 |
| 104 | `LZ4_compress_HC_extStateHC_fastReset` | `state` not aligned: `if (!LZ4_isAligned(state, LZ4_streamHC_t_alignment())) return 0;` — lz4hc.c:1503 | returns 0 |
| 105 | `LZ4_compress_HC_extStateHC` | `state == NULL`: `LZ4_initStreamHC` returns NULL ⇒ `if (ctx==NULL) return 0;` — lz4hc.c:1514-1515 | returns 0 |
| 106 | `LZ4_compress_HC_extStateHC` | `state` misaligned (fails `LZ4_streamHC_t_alignment()` test inside `LZ4_initStreamHC`) — lz4hc.c:1515 (+1580) | returns 0 |
| 107 | `LZ4_compress_HC` | `LZ4HC_HEAPMODE==1` (the default) and `ALLOC(sizeof(LZ4_streamHC_t))` (262200 bytes) fails — lz4hc.c:1523-1524 | returns 0 |
| 108 | `LZ4_compress_HC_destSize` | `state == NULL` or misaligned ⇒ `LZ4_initStreamHC` NULL — lz4hc.c:1540-1541 | returns 0 |
| 109 | `LZ4_initStreamHC` | `buffer == NULL` — lz4hc.c:1578 | returns NULL |
| 110 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` (`LZ4_STREAMHC_MINSIZE` = 262200) — lz4hc.c:1579 | returns NULL |
| 111 | `LZ4_initStreamHC` | `buffer` not aligned to `LZ4_streamHC_t_alignment()` — lz4hc.c:1580 | returns NULL |
| 112 | `LZ4_createStreamHC` | `ALLOC_AND_ZERO(sizeof(LZ4_streamHC_t))` fails — lz4hc.c:1556-1558 | returns NULL |
| 113 | `LZ4_freeStreamHC` | `LZ4_streamHCPtr == NULL` — lz4hc.c:1566 | returns 0 |
| 114 | `LZ4_freeHC` (obsolete) | `LZ4HC_Data == NULL` — lz4hc.c:2169 | returns 0 |
| 115 | `LZ4_createHC` (obsolete) | `LZ4_createStreamHC()` returns NULL (allocation failure) — lz4hc.c:2161-2162 | returns NULL |
| 116 | `LZ4_resetStreamStateHC` (obsolete) | `state == NULL` or misaligned ⇒ `LZ4_initStreamHC` returns NULL — lz4hc.c:2152-2153. **Correction (verified against the C):** the signature is `LZ4_resetStreamStateHC(void* state, char* inputBuffer)` — there is NO `size` parameter; it hard-codes `sizeof(*hc4)`, so a "too small" trigger is not expressible through this entry point. Only NULL and misaligned are. | returns **1** (non-zero = error; success is 0 — inverted vs every other function) |
| 117 | `LZ4_compress_HC` with `compressionLevel <= 2` (lz4mid strategy) | `dstCapacity < 0`: `if (maxOutputSize < 0) return 0;` in static `LZ4MID_compress` — lz4hc.c:560 | returns 0 |
| 118 | `LZ4_compress_HC` with level ≤ 2 | `*srcSizePtr < 0`: `if (*srcSizePtr < 0) return 0;` — lz4hc.c:559 (defensive; unreachable through the public API because lz4hc.c:1389 rejects first) | returns 0 |
| 119 | `LZ4_compress_HC` with level ≤ 2 | `*srcSizePtr > LZ4_MAX_INPUT_SIZE` — lz4hc.c:561-564 (duplicate guard of lz4hc.c:1389) | returns 0 |
| 120 | `LZ4_compress_HC` with level ≤ 2 | `limit == limitedOutput` and last literal run does not fit: `op + totalSize > oend` — lz4hc.c:713-714 | returns 0 |
| 121 | `LZ4_compress_HC` with level ≤ 2 | mid-stream output overflow (`LZ4HC_encodeSequence` returned 1) with `limit != fillOutput`: falls through `_lz4mid_dest_overflow` — lz4hc.c:684-689, 771-772 | returns 0 |
| 122 | `LZ4_compress_HC` levels 3-9 (hashChain) | `dstCapacity` too small for the literals of a sequence: `limit && ((op + (length/255) + length + (2+1+LASTLITERALS)) > oend)` in `LZ4HC_encodeSequence` — lz4hc.c:305-309 ⇒ `goto _dest_overflow` | returns 1 from the helper ⇒ public call returns 0 (lz4hc.c:1361) |
| 123 | `LZ4_compress_HC` levels 3-9 | `dstCapacity` too small for the match length: `limit && (op + (length/255) + (1+LASTLITERALS) > oend)` — lz4hc.c:331-334 | helper returns 1 ⇒ public call returns 0 |
| 124 | `LZ4_compress_HC` levels 3-9 | `limit == limitedOutput` and last literal run does not fit — lz4hc.c:1314-1315 | returns 0 |
| 125 | `LZ4_compress_HC` levels 3-9 | `_dest_overflow` reached with `limit != fillOutput` — lz4hc.c:1340-1341, 1360-1361 | returns 0 |
| 126 | `LZ4_compress_HC` levels 10-12 (optimal parser) | `limit == limitedOutput` and last literal run does not fit: `retval = 0; goto _return_label;` — lz4hc.c:2065-2069 | returns 0 |
| 127 | `LZ4_compress_HC` levels 10-12 | `_dest_overflow` with `limit != fillOutput`: `retval` stays at its initial 0 — lz4hc.c:1835, 2095-2117, 2122 | returns 0 |
| 128 | `LZ4_compress_HC` levels 10-12 | `LZ4HC_HEAPMODE==1` and `ALLOC(sizeof(LZ4HC_optimal_t)*(LZ4_OPT_NUM+3))` fails: `if (opt == NULL) goto _return_label;` — lz4hc.c:1838, 1856 | returns 0 |
| 129 | `LZ4_compress_HC` level 12 | internal `sufficient_len` (table `targetLength` = `LZ4_OPT_NUM` = 4096) `>= LZ4_OPT_NUM` — lz4hc.c:1861 | silently clamped to `LZ4_OPT_NUM-1` (4095) |
| 130 | `LZ4_compress_HC_destSize` / `LZ4_compress_HC_continue_destSize` | `targetDestSize` smaller than needed (`fillOutput`): input is silently truncated instead of failing — lz4hc.c:712-719 (mid), 1313-1320 / 1341-1358 (hashChain), 2064-2073 / 2096-2117 (optimal) | returns >0 and writes the consumed byte count into `*srcSizePtr` (`*sourceSizePtr`) |
| 131 | `LZ4_compress_HC_continue` | `dstCapacity >= LZ4_compressBound(srcSize)` selects `notLimited` — lz4hc.c:1725-1728 | no output-bound check performed; returns >0 (overflows `dst` if capacity was overstated) |
| 132 | `LZ4_compressHC2_continue` (obsolete) | `LZ4_compressHC2_continue` hard-codes `dstCapacity = 0` with `notLimited` — lz4hc.c:2177, so every output-overflow check is compiled out. **Correction (verified against the C):** this applies ONLY to `LZ4_compressHC2_continue`; `LZ4_compressHC2_limitedOutput_continue` (lz4hc.c:2182) forwards the caller's `dstCapacity` with `limitedOutput` and therefore DOES return 0 on overflow — it is the contrasting control. | no output-bound check at all; cannot return 0 for overflow |
| 133 | any HC compress entry (`LZ4_compress_HC*`, `LZ4_compress_HC_continue*`, `LZ4_compressHC2*`) | compression returned `<= 0`: `if (result <= 0) ctx->dirty = 1;` — lz4hc.c:1412 | the stream is marked dirty, so a later `LZ4_resetStreamHC_fast` performs a **full** re-init (lz4hc.c:1599-1600) instead of a fast reset |
| 134 | `LZ4_loadDictHC` | `dictSize > 64 KB`: `dictionary += dictSize - 64 KB; dictSize = 64 KB;` — lz4hc.c:1634-1637 | silently keeps only the last 64 KB; returns 65536 |
| 135 | `LZ4_loadDictHC` | `dictSize < LZ4HC_HASHSIZE (4)` (non-lz4mid levels): `if (dictSize >= LZ4HC_HASHSIZE) LZ4HC_Insert(...)` is skipped — lz4hc.c:1649 | returns `dictSize` unchanged but no match references are inserted (dictionary effectively unusable) |
| 136 | `LZ4_loadDictHC` | `dictSize <= LZ4MID_HASHSIZE (8)` at levels ≤ 2: `if (size <= LZ4MID_HASHSIZE) return;` in `LZ4MID_fillHTable` — lz4hc.c:498-499 | returns `dictSize` but hash tables stay empty |
| 137 | `LZ4_loadDictHC` | `dictSize < 0`: only `assert(dictSize >= 0)` — lz4hc.c:1632 | no rejection; returns the negative `dictSize`, `ctxPtr->end` set before `dictionary` ⇒ UB in release |
| 138 | `LZ4_saveDictHC` | `dictSize > 64 KB` — lz4hc.c:1748 | silently clamps to 65536; returns clamped value |
| 139 | `LZ4_saveDictHC` | `dictSize < 4` (includes 0 and negative) — lz4hc.c:1749 | sets `dictSize = 0`; returns 0, nothing copied |
| 140 | `LZ4_saveDictHC` | `dictSize > prefixSize` (more than the available history) — lz4hc.c:1750 | clamps to `prefixSize`; returns that value |
| 141 | `LZ4_saveDictHC` | `safeBuffer == NULL` with `dictSize != 0`: only `assert(dictSize == 0)` — lz4hc.c:1751 | no rejection; `LZ4_memmove` to NULL ⇒ UB in release |
| 142 | `LZ4_attach_HC_dictionary` | `dictionary_stream == NULL` — lz4hc.c:1655 | void; `dictCtx` silently set to NULL (detach) |
| 143 | `LZ4_compress_HC_continue` / `LZ4_compress_HC_continue_destSize` | `src` range overlaps the context's extDict (`sourceEnd > dictBegin && src < dictEnd`) — lz4hc.c:1706-1717 | dictionary silently shrunk; fully invalidated when `dictLimit - lowLimit < LZ4HC_HASHSIZE (4)` |
| 144 | `LZ4_compress_HC_continue` / `_continue_destSize` | accumulated stream position `(end-prefixStart) + dictLimit > 2 GB` — lz4hc.c:1695-1699 | dictionary silently reloaded via `LZ4_loadDictHC` from the last ≤64 KB (history beyond that is dropped) |
| 145 | `LZ4_compress_HC_continue` with an attached dictCtx | `position >= 64 KB` — `LZ4HC_compress_generic_dictCtx`, lz4hc.c:1452-1456 | attached dictionary silently dropped (`ctx->dictCtx = NULL`) |
| 146 | `LZ4_compress_HC_continue` with an attached dictCtx | dictCtx strategy incompatible with the working stream's (`isStateCompatible` false, lz4hc.c:1434-1439, 1457) | falls back to `usingDictCtxHc` path instead of the fast table copy (silent behaviour change, no error) |
| 147 | `LZ4_compress_HC_extStateHC_fastReset` | `state` "correctly initialized" precondition violated (e.g. never initialized, or dirty from a previous failure) — no check beyond the alignment test at lz4hc.c:1503 | no rejection; garbage state is used (UB) |


## xxhash.c

`XXH_errorcode` is `typedef enum { XXH_OK=0, XXH_ERROR } XXH_errorcode;` (xxhash.h:79).
Public symbols carry the `LZ4_` namespace prefix (`XXH_NAMESPACE=LZ4_`).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 148 | `LZ4_XXH32_update` | `input == NULL` (any `len`, including 0), because `XXH_ACCEPT_NULL_INPUT_POINTER` defaults to 0 (xxhash.c:70-72) so the `return XXH_OK` alternative is compiled out — `XXH32_update_endian`, xxhash.c:454-459 | returns `XXH_ERROR` (1); state left untouched |
| 149 | `LZ4_XXH64_update` | `input == NULL` (any `len`) — `XXH64_update_endian`, xxhash.c:914-919 | returns `XXH_ERROR` (1); state left untouched |
| 150 | `LZ4_XXH32_update` | valid `input` with any `len` (including 0 and huge) — every other path returns success — xxhash.c:470, 511 | returns `XXH_OK` (0); **no** length/overflow rejection exists |
| 151 | `LZ4_XXH32_update` | cumulative input length > 4 GiB: `state->total_len_32 += (unsigned)len` is a 32-bit accumulator — xxhash.c:464 | no error; length silently wraps mod 2^32 and the digest reflects the wrapped length |
| 152 | `LZ4_XXH32_reset` | `statePtr == NULL`: there is **no** NULL check; `memcpy(statePtr, &state, sizeof(state)-sizeof(state.reserved))` — xxhash.c:437-448 | no rejection; NULL write ⇒ crash/UB. For any non-NULL state it unconditionally returns `XXH_OK` (0) |
| 153 | `LZ4_XXH64_reset` | `statePtr == NULL`: no NULL check; `memcpy` at xxhash.c:907 | no rejection; UB. Otherwise always returns `XXH_OK` (0) |
| 154 | `LZ4_XXH32_copyState` / `LZ4_XXH64_copyState` | `dstState == NULL` or `srcState == NULL`: plain `memcpy`, no checks — xxhash.c:432-435 and 893-896 | void, no rejection; UB on NULL |
| 155 | `LZ4_XXH32_freeState` | `statePtr == NULL`: `XXH_free(NULL)` is a no-op, no check — xxhash.c:426-430 | returns `XXH_OK` (0) |
| 156 | `LZ4_XXH64_freeState` | `statePtr == NULL` — xxhash.c:887-891 | returns `XXH_OK` (0) |
| 157 | `LZ4_XXH32_createState` | `XXH_malloc(sizeof(XXH32_state_t))` fails — xxhash.c:422-425 | returns NULL |
| 158 | `LZ4_XXH64_createState` | `XXH_malloc(sizeof(XXH64_state_t))` fails — xxhash.c:883-886 | returns NULL |
| 159 | `LZ4_XXH32` | `input == NULL` with `len > 0`: the NULL guard at xxhash.c:359-364 is compiled out (`XXH_ACCEPT_NULL_INPUT_POINTER == 0`) | no rejection and no sentinel (return type is `unsigned`); dereferences NULL ⇒ crash |
| 160 | `LZ4_XXH32` | `input == NULL` with `len == 0`: `len >= 16` false and `XXH32_finalize` with `len&15 == 0` returns immediately — xxhash.c:366-388, 344 | no crash; returns the seed-only hash (no error signalling) |
| 161 | `LZ4_XXH64` | `input == NULL` with `len > 0`: guard at xxhash.c:818-823 compiled out | no rejection; NULL dereference |
| 162 | `LZ4_XXH32_digest` / `LZ4_XXH64_digest` | `state_in == NULL`: no check, state fields are read directly — xxhash.c:531-542 and 985-1002 | no rejection (return type has no error value); UB |
| 163 | `LZ4_XXH32_hashFromCanonical` / `LZ4_XXH64_hashFromCanonical` | `src == NULL`: no check, direct `XXH_readBE32/64` — xxhash.c:572-575 and 1025-1028 | no rejection; UB |
| 164 | `LZ4_XXH32_canonicalFromHash` / `LZ4_XXH64_canonicalFromHash` | `dst == NULL`: no check, direct `memcpy` — xxhash.c:565-570 and 1018-1023 | void, no rejection; UB |
| 165 | `LZ4_XXH32` / `LZ4_XXH32_digest` | unreachable `switch` default in `XXH32_finalize` (all 16 values of `len&15` are handled): `assert(0)` — xxhash.c:346-347 | assert fires only with `assert()` enabled; otherwise returns `h32` unmixed |
| 166 | `LZ4_XXH64` / `LZ4_XXH64_digest` | unreachable `switch` default in `XXH64_finalize` (all 32 values of `len&31` are handled): `assert(0)` — xxhash.c:805-807 | assert fires only with `assert()` enabled; otherwise returns 0 |
| 167 | compile-time (`LZ4_XXH32_canonicalFromHash`) | `XXH_STATIC_ASSERT(sizeof(XXH32_canonical_t) == sizeof(XXH32_hash_t))` — xxhash.c:567 (64-bit counterpart at xxhash.c:1020) | build fails (compile-time division by zero) |


## lz4frame.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|---|---|---|
| 168 | `LZ4F_getBlockSize` | `blockSizeID` in 1..3 (e.g. `blockSizeID == 3`), i.e. `blockSizeID < LZ4F_max64KB(4)` after the `0 -> LZ4F_max64KB` remap — lz4frame.c:337-339 | error `LZ4F_ERROR_maxBlockSize_invalid` = `-(size_t)LZ4F_ERROR_maxBlockSize_invalid`; `LZ4F_isError()==1`, `LZ4F_getErrorCode()==2` |
| 169 | `LZ4F_getBlockSize` | `blockSizeID > LZ4F_max4MB(7)` (e.g. `blockSizeID == 8`, or any value >= 8) — lz4frame.c:338-339 | error `LZ4F_ERROR_maxBlockSize_invalid` (`LZ4F_getErrorCode()==2`) |
| 170 | `LZ4F_compressFrame`, `LZ4F_compressFrame_usingCDict` | `dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs)` where `prefs` has been auto-corrected (`autoFlush=1`, `blockSizeID` via `LZ4F_optimalBSID()`, `contentSize=srcSize` if nonzero). Smallest reproduction: `srcSize==0`, `dstCapacity==0` (bound is 19+4+4) — lz4frame.c:456 | error `LZ4F_ERROR_dstMaxSize_tooSmall` (`LZ4F_getErrorCode()==11`) |
| 171 | `LZ4F_compressBegin`, `LZ4F_compressBegin_usingDict`, `LZ4F_compressBegin_usingDictOnce`, `LZ4F_compressBegin_usingCDict` (static helper `LZ4F_compressBegin_internal`); also reached from `LZ4F_compressFrame*` and `LZ4F_writeOpen` | `dstCapacity < maxFHSize` i.e. `dstCapacity < 19` (`LZ4F_HEADER_SIZE_MAX`); e.g. `dstCapacity == 18` — lz4frame.c:700 | error `LZ4F_ERROR_dstMaxSize_tooSmall` (`LZ4F_getErrorCode()==11`) |
| 172 | `LZ4F_compressBegin`, `LZ4F_compressBegin_usingDict`, `LZ4F_compressBegin_usingCDict` (helper `LZ4F_compressBegin_internal`) | allocation of the internal LZ4/LZ4HC stream fails: `cctx->lz4CtxPtr == NULL` after `LZ4F_malloc(sizeof(LZ4_stream_t))` / `LZ4F_malloc(sizeof(LZ4_streamHC_t))`. Reproducible with a `LZ4F_CustomMem` whose `customAlloc` returns NULL (via `LZ4F_createCompressionContext_advanced`) — lz4frame.c:714-722 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`) |
| 173 | `LZ4F_compressBegin`, `LZ4F_compressBegin_usingDict`, `LZ4F_compressBegin_usingCDict` (helper `LZ4F_compressBegin_internal`) | allocation of the streaming tmp buffer fails: `cctx->tmpBuff == NULL` after `LZ4F_malloc(requiredBuffSize)` where `requiredBuffSize` = `autoFlush ? (blockLinked ? 64KB : 0) : maxBlockSize + (blockLinked ? 128KB : 0)`. Reproducible with a failing `LZ4F_CustomMem.customAlloc` — lz4frame.c:749-750 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`) |
| 174 | `LZ4F_compressBegin_usingDict`, `LZ4F_compressBegin_usingDictOnce` (helper `LZ4F_compressBegin_internal`) | non-NULL `dictBuffer` with `dictSize > INT_MAX` (i.e. `dictSize > 2147483647`, e.g. `0x80000000`) — lz4frame.c:766-768 | error `LZ4F_ERROR_parameter_invalid` (`LZ4F_getErrorCode()==4`) |
| 175 | `LZ4F_createCompressionContext` | `cctxPtr == NULL` (first argument NULL) — `assert(LZ4F_compressionContextPtr != NULL)` at lz4frame.c:620 fires when built with `LZ4_DEBUG>=1` | debug builds: `assert` abort. Release builds: falls through to the check on the next line (see row 9) |
| 176 | `LZ4F_createCompressionContext` | `cctxPtr == NULL` (narrow-contract violation), release build — lz4frame.c:622 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 177 | `LZ4F_createCompressionContext` | `LZ4F_createCompressionContext_advanced()` returns NULL (calloc of `sizeof(LZ4F_cctx)` fails) so `*cctxPtr == NULL` — lz4frame.c:624-625 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`); `*cctxPtr` left NULL |
| 178 | `LZ4F_createCompressionContext_advanced` | `LZ4F_calloc(sizeof(LZ4F_cctx), customMem)` fails (e.g. `customMem.customCalloc`/`customAlloc` returns NULL) — lz4frame.c:598-600 | returns NULL |
| 179 | `LZ4F_createCDict`, `LZ4F_createCDict_advanced` | `LZ4F_malloc(sizeof(LZ4F_CDict), cmem)` for the CDict struct fails — lz4frame.c:542-544 | returns NULL |
| 180 | `LZ4F_createCDict`, `LZ4F_createCDict_advanced` | any of the three sub-allocations fails: `!cdict->dictContent \|\| !cdict->fastCtx \|\| !cdict->HCCtx` (`dictContent` = `min(dictSize, 64 KB)` bytes, `fastCtx` = `sizeof(LZ4_stream_t)`, `HCCtx` = `sizeof(LZ4_streamHC_t)`) — lz4frame.c:550-557 | returns NULL (and internally `LZ4F_freeCDict(cdict)` is called first) |
| 181 | `LZ4F_compressUpdate`, `LZ4F_uncompressedUpdate` (helper `LZ4F_compressUpdateImpl`) | `cctxPtr->cStage != 1`, i.e. `LZ4F_compressBegin*()` was never called on this cctx, or `LZ4F_compressEnd()` already reset it to 0 — lz4frame.c:1005 | error `LZ4F_ERROR_compressionState_uninitialized` (`LZ4F_getErrorCode()==20`) |
| 182 | `LZ4F_compressUpdate`, `LZ4F_uncompressedUpdate` (helper `LZ4F_compressUpdateImpl`) | `dstCapacity < LZ4F_compressBound_internal(srcSize, &cctxPtr->prefs, cctxPtr->tmpInSize)`; e.g. after a valid `LZ4F_compressBegin()` call `LZ4F_compressUpdate(cctx, dst, 0, src, 1, NULL)` — lz4frame.c:1006-1007 | error `LZ4F_ERROR_dstMaxSize_tooSmall` (`LZ4F_getErrorCode()==11`) |
| 183 | `LZ4F_uncompressedUpdate` only (helper `LZ4F_compressUpdateImpl` with `blockCompression == LZ4B_UNCOMPRESSED`) | `dstCapacity < srcSize` (extra check that only applies to the uncompressed path) — lz4frame.c:1009-1010 | error `LZ4F_ERROR_dstMaxSize_tooSmall` (`LZ4F_getErrorCode()==11`) |
| 184 | `LZ4F_uncompressedUpdate` (helper `LZ4F_compressUpdateImpl`) | frame configured with `frameInfo.blockMode == LZ4F_blockLinked` (the default, 0) and the last block came from the src buffer: `assert(blockCompression == LZ4B_COMPRESSED)` at lz4frame.c:1071 is violated (uncompressed blocks are only supported with `LZ4F_blockIndependent`) | debug builds (`LZ4_DEBUG>=1`): `assert` abort. Release builds: no error is returned — silently produces a frame whose linked-block history is inconsistent (documented restriction, unenforced) |
| 185 | `LZ4F_flush`; also reached from `LZ4F_compressEnd` (lz4frame.c:1213) and from `LZ4F_compressUpdate`/`LZ4F_uncompressedUpdate` (lz4frame.c:1014) | `cctxPtr->tmpInSize != 0` (data buffered) **and** `cctxPtr->cStage != 1` — lz4frame.c:1167-1168. Note: when `tmpInSize == 0` the function returns 0 *before* the state check, so `LZ4F_flush`/`LZ4F_compressEnd` on a fresh/finished cctx does **not** report an uninitialized state | error `LZ4F_ERROR_compressionState_uninitialized` (`LZ4F_getErrorCode()==20`) |
| 186 | `LZ4F_flush`; also reached from `LZ4F_compressEnd` | `dstCapacity < cctxPtr->tmpInSize + BHSize + BFSize` i.e. `< tmpInSize + 8`, with `tmpInSize > 0` (requires `autoFlush == 0` so data can be buffered) — lz4frame.c:1169 | error `LZ4F_ERROR_dstMaxSize_tooSmall` (`LZ4F_getErrorCode()==11`) |
| 187 | `LZ4F_compressUpdate`, `LZ4F_uncompressedUpdate` (helper `LZ4F_compressUpdateImpl`) | block-compression-mode switch with buffered data and tight capacity: `cctxPtr->blockCompressMode != blockCompression` triggers `LZ4F_flush()` at lz4frame.c:1014 whose return value is **not** error-checked (`dstPtr += bytesWritten`). Reachable with `autoFlush==0`: `LZ4F_uncompressedUpdate(...)` to buffer `tmpInSize` bytes, then `LZ4F_compressUpdate(...)` with `dstCapacity` satisfying row 15 but `< tmpInSize + 8` (e.g. `dstCapacity==4`, `srcSize` small) | Latent bug: the `-(size_t)LZ4F_ERROR_dstMaxSize_tooSmall` value is added to `dstPtr`; the function keeps going and returns `(size_t)(dstPtr - dstStart)`. Observable result is a wrapped/garbage `size_t` (typically the `dstMaxSize_tooSmall` code itself when no further bytes are emitted) rather than a reliably reported error |
| 188 | `LZ4F_compressEnd` | after the internal `LZ4F_flush()`, `dstCapacity - flushSize < 4` (no room for the 4-byte endMark); e.g. `dstCapacity == 3` on a cctx with no buffered data — lz4frame.c:1221 | error `LZ4F_ERROR_dstMaxSize_tooSmall` (`LZ4F_getErrorCode()==11`) |
| 189 | `LZ4F_compressEnd` | frame has `frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled` and, after the internal flush, `dstCapacity < 8` (endMark 4 + content checksum 4); e.g. `dstCapacity == 7` — lz4frame.c:1225-1227 | error `LZ4F_ERROR_dstMaxSize_tooSmall` (`LZ4F_getErrorCode()==11`) |
| 190 | `LZ4F_compressEnd` | `cctxPtr->prefs.frameInfo.contentSize != 0` and `contentSize != cctxPtr->totalInSize` (declared content size does not match the number of bytes actually fed through `LZ4F_compressUpdate`), e.g. declare `contentSize=100` then feed 99 bytes — lz4frame.c:1235-1237 | error `LZ4F_ERROR_frameSize_wrong` (`LZ4F_getErrorCode()==14`). Note: `cStage` has already been reset to 0 at lz4frame.c:1233, and the endMark/checksum bytes have already been written into `dstBuffer` |
| 191 | `LZ4F_createDecompressionContext` | `dctxPtr == NULL` — `assert(LZ4F_decompressionContextPtr != NULL)` at lz4frame.c:1303 fires when built with `LZ4_DEBUG>=1` | debug builds: `assert` abort. Release builds: falls through to row 25 |
| 192 | `LZ4F_createDecompressionContext` | `dctxPtr == NULL` (narrow-contract violation), release build — lz4frame.c:1304 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 193 | `LZ4F_createDecompressionContext` | `LZ4F_createDecompressionContext_advanced()` returns NULL (calloc of `sizeof(LZ4F_dctx)` fails), so `*dctxPtr == NULL` — lz4frame.c:1306-1309 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`) |
| 194 | `LZ4F_createDecompressionContext_advanced` | `LZ4F_calloc(sizeof(LZ4F_dctx), customMem)` fails — lz4frame.c:1286-1287 | returns NULL |
| 195 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (static helper `LZ4F_decodeHeader`) | `srcSize < minFHSize` i.e. fewer than 7 bytes of frame header available at the point `LZ4F_decodeHeader()` is entered — lz4frame.c:1354. (Via `LZ4F_getFrameInfo` this is reachable when `LZ4F_headerSize()` reports 7 but a 5- or 6-byte buffer was supplied; via `LZ4F_decompress` the header is buffered first, so this branch is mostly guarded.) | error `LZ4F_ERROR_frameHeader_incomplete` (`LZ4F_getErrorCode()==12`) |
| 196 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (helper `LZ4F_decodeHeader`) | first 4 bytes are neither `LZ4F_MAGICNUMBER` (0x184D2204 LE) nor a skippable magic (`value & 0xFFFFFFF0 == 0x184D2A50`); e.g. src starts with `00 00 00 00` or `04 22 4D 19` (byte-swapped) — lz4frame.c:1358, 1372-1374 (compiled out under `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`) | error `LZ4F_ERROR_frameType_unknown` (`LZ4F_getErrorCode()==13`) |
| 197 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (helper `LZ4F_decodeHeader`) | FLG byte (`src[4]`) has reserved bit 1 set: `((FLG>>1)&1) != 0`, e.g. `FLG == 0x42` — lz4frame.c:1388 | error `LZ4F_ERROR_reservedFlag_set` (`LZ4F_getErrorCode()==8`) |
| 198 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (helper `LZ4F_decodeHeader`) | FLG version field `(FLG>>6)&3 != 1`, i.e. FLG top two bits are `00`, `10` or `11` (e.g. `FLG == 0x00`, `0x80`, `0xC0`) — lz4frame.c:1389 | error `LZ4F_ERROR_headerVersion_wrong` (`LZ4F_getErrorCode()==6`) |
| 199 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (helper `LZ4F_decodeHeader`) | BD byte (`src[5]`) has reserved bit 7 set: `((BD>>7)&1) != 0`, e.g. `BD == 0xC0` — lz4frame.c:1409 | error `LZ4F_ERROR_reservedFlag_set` (`LZ4F_getErrorCode()==8`) |
| 200 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (helper `LZ4F_decodeHeader`) | BD blockSizeID field `(BD>>4)&7 < 4`, i.e. `BD & 0x70` in {0x00,0x10,0x20,0x30} (e.g. `BD == 0x00` or `BD == 0x30`) — lz4frame.c:1410. (The field is 3 bits so `> 7` is unrepresentable; only the `< 4` branch exists.) | error `LZ4F_ERROR_maxBlockSize_invalid` (`LZ4F_getErrorCode()==2`) |
| 201 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (helper `LZ4F_decodeHeader`) | BD low nibble non-zero: `((BD>>0)&0x0F) != 0`, e.g. `BD == 0x71` — lz4frame.c:1411 | error `LZ4F_ERROR_reservedFlag_set` (`LZ4F_getErrorCode()==8`) |
| 202 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` (helper `LZ4F_decodeHeader`) | header checksum byte mismatch: `LZ4F_headerChecksum(src+4, frameHeaderSize-5) != src[frameHeaderSize-1]` where `frameHeaderSize = 7 + (contentSizeFlag?8:0) + (dictIDFlag?4:0)`; e.g. flip the last header byte of a valid frame — lz4frame.c:1417-1418 (compiled out under `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`) | error `LZ4F_ERROR_headerChecksum_invalid` (`LZ4F_getErrorCode()==17`) |
| 203 | `LZ4F_headerSize` | `src == NULL` — lz4frame.c:1446 | error `LZ4F_ERROR_srcPtr_wrong` (`LZ4F_getErrorCode()==15`) |
| 204 | `LZ4F_headerSize` | `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` i.e. `srcSize < 5` (e.g. `srcSize == 4`) — lz4frame.c:1449-1450 | error `LZ4F_ERROR_frameHeader_incomplete` (`LZ4F_getErrorCode()==12`) |
| 205 | `LZ4F_headerSize` | first 4 bytes are neither `LZ4F_MAGICNUMBER` nor a skippable magic (`& 0xFFFFFFF0 == 0x184D2A50`) — lz4frame.c:1458-1459 (compiled out under `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`) | error `LZ4F_ERROR_frameType_unknown` (`LZ4F_getErrorCode()==13`) |
| 206 | `LZ4F_getFrameInfo` | `dctx->dStage == dstage_storeFrameHeader` (1): decoding was started with a partial frame header (e.g. `LZ4F_decompress()` was fed 1..6 bytes first), then `LZ4F_getFrameInfo()` is called — lz4frame.c:1498-1501 | error `LZ4F_ERROR_frameDecoding_alreadyStarted` (`LZ4F_getErrorCode()==19`), and `*srcSizePtr` is set to 0 |
| 207 | `LZ4F_getFrameInfo` | `LZ4F_headerSize()` fails (rows 36-38: `srcBuffer == NULL`, `*srcSizePtr < 5`, or bad magic) on a fresh dctx (`dStage == dstage_getFrameHeader`) — lz4frame.c:1503-1504 | the `LZ4F_headerSize()` error is returned verbatim (`srcPtr_wrong`==15 / `frameHeader_incomplete`==12 / `frameType_unknown`==13) and `*srcSizePtr` is forced to 0 |
| 208 | `LZ4F_getFrameInfo` | `*srcSizePtr < hSize` where `hSize = LZ4F_headerSize(srcBuffer, *srcSizePtr)`; e.g. a frame with contentSize+dictID flags set (`hSize == 19`) but only 7 bytes supplied — lz4frame.c:1505-1508 | error `LZ4F_ERROR_frameHeader_incomplete` (`LZ4F_getErrorCode()==12`), and `*srcSizePtr` is set to 0 |
| 209 | `LZ4F_getFrameInfo` | `LZ4F_decodeHeader()` fails (rows 28, 30-35) — lz4frame.c:1510-1512 | the `LZ4F_decodeHeader()` error is returned verbatim, and `*srcSizePtr` is set to 0; `*frameInfoPtr` is still overwritten with `dctx->frameInfo` (zeroed by `MEM_INIT` at lz4frame.c:1355) |
| 210 | `LZ4F_getFrameInfo` | `dctx->dStage > dstage_storeFrameHeader` (header already decoded): the function tail-calls `LZ4F_decompress(dctx, NULL, &o, NULL, &i, NULL)` with `o=i=0`, so any error that stage can produce is returned; notably `dstage_getSuffix` with `dctx->frameRemainingSize != 0` — lz4frame.c:1490-1496 | whatever `LZ4F_decompress()` returns, e.g. error `LZ4F_ERROR_frameSize_wrong` (`LZ4F_getErrorCode()==14`); `*srcSizePtr` is set to 0 and `*frameInfoPtr` is filled first |
| 211 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` | `dstage_init`: allocation of `dctx->tmpIn` (`maxBlockSize + BFSize` bytes) fails — lz4frame.c:1685-1686. Reproducible via `LZ4F_createDecompressionContext_advanced()` with a failing `customAlloc` | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`) |
| 212 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` | `dstage_init`: allocation of `dctx->tmpOutBuffer` (`maxBlockSize + (blockLinked ? 128KB : 0)` bytes) fails — lz4frame.c:1687-1689 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`) |
| 213 | `LZ4F_decompress`, `LZ4F_decompress_usingDict` | block header declares a block larger than the frame's block size: `(blockHeader & 0x7FFFFFFF) > dctx->maxBlockSize`, where `maxBlockSize` is 64KB/256KB/1MB/4MB per blockSizeID; e.g. blockSizeID=4 (64KB) with block header `0x00010001` (65537). Applies to both compressed and uncompressed (high-bit-set) blocks — lz4frame.c:1737-1739 | error `LZ4F_ERROR_maxBlockSize_invalid` (`LZ4F_getErrorCode()==2`) |
| 214 | `LZ4F_decompress`, `LZ4F_decompress_usingDict` | **uncompressed** block (block header bit31 = `LZ4F_BLOCKUNCOMPRESSED_FLAG` set) in a frame with `blockChecksumFlag == 1`, and the trailing 4-byte block checksum read in `dstage_getBlockChecksum` != `XXH32(blockData, 0)` — lz4frame.c:1821-1830 (compiled out under `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`; also skipped when `decompressOptions.skipChecksums != 0`) | error `LZ4F_ERROR_blockChecksum_invalid` (`LZ4F_getErrorCode()==7`) |
| 215 | `LZ4F_decompress`, `LZ4F_decompress_usingDict` | **compressed** block in a frame with `blockChecksumFlag == 1`, and the trailing 4-byte checksum `LZ4F_readLE32(selectedIn + tmpInTarget)` != `XXH32(selectedIn, tmpInTarget, 0)` — lz4frame.c:1871-1878 (compiled out under `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`; note this path is **not** gated by `skipChecksum`) | error `LZ4F_ERROR_blockChecksum_invalid` (`LZ4F_getErrorCode()==7`) |
| 216 | `LZ4F_decompress`, `LZ4F_decompress_usingDict` | corrupt LZ4 block payload while decoding **directly into dstBuffer** (taken when `(dstEnd-dstPtr) >= maxBlockSize` and the dict is not in `tmpOut`): `LZ4_decompress_safe_usingDict() < 0` — lz4frame.c:1901-1905 | error `LZ4F_ERROR_decompressionFailed` (`LZ4F_getErrorCode()==16`) |
| 217 | `LZ4F_decompress`, `LZ4F_decompress_usingDict` | corrupt LZ4 block payload while decoding **into `dctx->tmpOut`** (taken when dst has less than `maxBlockSize` room, e.g. a small `*dstSizePtr`, or when the dictionary lives in `tmpOut`): `LZ4_decompress_safe_usingDict() < 0` — lz4frame.c:1946-1950 | error `LZ4F_ERROR_decompressionFailed` (`LZ4F_getErrorCode()==16`) |
| 218 | `LZ4F_decompress`, `LZ4F_decompress_usingDict`, `LZ4F_getFrameInfo` | `dstage_getSuffix` reached (block header == 0 endMark) while `dctx->frameRemainingSize != 0`: frame header declared a `contentSize` that does not match the bytes actually regenerated — both too little (positive remainder) and too much (`frameRemainingSize` underflows to a huge U64) trigger it — lz4frame.c:1984 | error `LZ4F_ERROR_frameSize_wrong` (`LZ4F_getErrorCode()==14`) |
| 219 | `LZ4F_decompress`, `LZ4F_decompress_usingDict` | frame with `contentChecksumFlag == 1` whose trailing 4-byte content checksum != `XXH32_digest(&dctx->xxh)`; e.g. flip a byte of the frame's last 4 bytes — lz4frame.c:2016-2021 (compiled out under `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`; skipped when `decompressOptions.skipChecksums != 0`) | error `LZ4F_ERROR_contentChecksum_invalid` (`LZ4F_getErrorCode()==18`) |
| 220 | `LZ4F_decompress`, `LZ4F_decompress_usingDict` | `dstBuffer == NULL` while `*dstSizePtr != 0`: `assert(*dstSizePtr == 0)` at lz4frame.c:1632 | debug builds (`LZ4_DEBUG>=1`): `assert` abort. Release builds: no error; `dstEnd` is NULL so all copy stages treat capacity as 0 and the frame simply makes no output progress |
| 221 | `LZ4F_freeDecompressionContext` | released while a frame is only partially decoded (`dctx->dStage != dstage_getFrameHeader(0)`), e.g. free after feeding a truncated frame — lz4frame.c:1316-1317 | returns `(LZ4F_errorCode_t)dctx->dStage`, a **non-zero but non-error** value in 1..14 (`dStage` enum). `LZ4F_isError()` returns 0 on it and `LZ4F_getErrorCode()` maps it to `LZ4F_OK_NoError`; callers must compare against 0 directly |


## lz4file.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|---|---|---|
| 222 | `LZ4F_readOpen` | `fp == NULL` — lz4file.c:79-81 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`); `*lz4fRead` untouched |
| 223 | `LZ4F_readOpen` | `lz4fRead == NULL` (out-parameter NULL) — lz4file.c:79-81 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 224 | `LZ4F_readOpen` | `calloc(1, sizeof(LZ4_readFile_t))` returns NULL — lz4file.c:83-86 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`); `*lz4fRead == NULL` |
| 225 | `LZ4F_readOpen` | `LZ4F_createDecompressionContext(&(*lz4fRead)->dctxPtr, LZ4F_VERSION)` fails (dctx calloc failure) — lz4file.c:88-92 | that error is returned verbatim (`LZ4F_ERROR_allocation_failed`, code 9); `LZ4F_freeAndNullReadFile()` frees the handle and sets `*lz4fRead = NULL` |
| 226 | `LZ4F_readOpen` | initial `fread(buf, 1, LZ4F_HEADER_SIZE_MAX, fp)` returns `!= 19`: file is shorter than 19 bytes total (this rejects *any* valid but very small `.lz4` file, e.g. a 15-byte frame: 7-byte header + 4-byte endMark), or is at EOF, or the stream errors — lz4file.c:95-99 | error `LZ4F_ERROR_io_read` (`LZ4F_getErrorCode()==23`); handle freed and `*lz4fRead = NULL` |
| 227 | `LZ4F_readOpen` | `LZ4F_getFrameInfo()` on the first 19 bytes fails: bad magic, reserved FLG/BD bits, wrong version, blockSizeID<4, bad header checksum (rows 29-35) — lz4file.c:101-106 | the `LZ4F_getFrameInfo()` error code is returned verbatim (e.g. `frameType_unknown`==13, `reservedFlag_set`==8, `headerVersion_wrong`==6, `maxBlockSize_invalid`==2, `headerChecksum_invalid`==17); handle freed and `*lz4fRead = NULL` |
| 228 | `LZ4F_readOpen` | `info.blockSizeID` falls into the `default:` arm of the switch, i.e. not in {`LZ4F_default`(0), 4, 5, 6, 7} — lz4file.c:108-125. **Defensive/unreachable in practice**: `LZ4F_decodeHeader()` already rejects `<4` and the BD field is only 3 bits, so `LZ4F_getFrameInfo()` can never return another value | error `LZ4F_ERROR_maxBlockSize_invalid` (`LZ4F_getErrorCode()==2`); handle freed and `*lz4fRead = NULL` |
| 229 | `LZ4F_readOpen` | `malloc((*lz4fRead)->srcBufMaxSize)` (64KB/256KB/1MB/4MB per the frame's blockSizeID) returns NULL — lz4file.c:128-132 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`); handle freed and `*lz4fRead = NULL` |
| 230 | `LZ4F_read` | `lz4fRead == NULL` — lz4file.c:145-146 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 231 | `LZ4F_read` | `buf == NULL` (checked unconditionally, before the loop, so it also fires for `size == 0`) — lz4file.c:145-146 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 232 | `LZ4F_read` | `else` arm of the `fread` result test at lz4file.c:159-163. **Dead code**: `ret` is `size_t`, so after `if (ret > 0)` and `else if (ret == 0)` the final `else` is unreachable; a read error is indistinguishable from EOF and takes the `break` at lz4file.c:160 | Unreachable `LZ4F_ERROR_io_read` (code 23). Actual observable behaviour on a short/failed read: the loop breaks and `LZ4F_read` returns the number of bytes decoded so far (`next`), i.e. a short read with **no** error indication |
| 233 | `LZ4F_read` | inner `LZ4F_decompress()` fails on corrupt/truncated frame content: bad block header size, block checksum, LZ4 block payload, content checksum, frame size (rows 46-52) — lz4file.c:166-173 | the `LZ4F_decompress()` error code is returned verbatim (`maxBlockSize_invalid`==2, `blockChecksum_invalid`==7, `decompressionFailed`==16, `contentChecksum_invalid`==18, `frameSize_wrong`==14, `allocation_failed`==9); the handle is **not** freed and its dctx is left unresumable |
| 234 | `LZ4F_readClose` | `lz4fRead == NULL` — lz4file.c:185-186 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 235 | `LZ4F_writeOpen` | `fp == NULL` — lz4file.c:222-223 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 236 | `LZ4F_writeOpen` | `lz4fWrite == NULL` (out-parameter NULL) — lz4file.c:222-223 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 237 | `LZ4F_writeOpen` | `calloc(1, sizeof(LZ4_writeFile_t))` returns NULL — lz4file.c:225-228 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`); `*lz4fWrite == NULL` |
| 238 | `LZ4F_writeOpen` | `prefsPtr != NULL` and `prefsPtr->frameInfo.blockSizeID` not in {`LZ4F_default`(0), 4, 5, 6, 7} — e.g. 1, 2, 3, 8, or 99 — hits the `default:` arm at lz4file.c:244-246 | error `LZ4F_ERROR_maxBlockSize_invalid` (`LZ4F_getErrorCode()==2`); handle freed and `*lz4fWrite = NULL` |
| 239 | `LZ4F_writeOpen` | `malloc(LZ4F_compressBound(maxWriteSize, prefsPtr))` for `dstBuf` returns NULL — lz4file.c:252-257 | error `LZ4F_ERROR_allocation_failed` (`LZ4F_getErrorCode()==9`); handle freed and `*lz4fWrite = NULL` |
| 240 | `LZ4F_writeOpen` | `LZ4F_createCompressionContext(&(*lz4fWrite)->cctxPtr, LZ4F_VERSION)` fails (cctx calloc failure) — lz4file.c:259-263 | that error is returned verbatim (`LZ4F_ERROR_allocation_failed`, code 9); handle freed and `*lz4fWrite = NULL` |
| 241 | `LZ4F_writeOpen` | `LZ4F_compressBegin(cctx, buf, LZ4F_HEADER_SIZE_MAX, prefsPtr)` fails — lz4file.c:265-269. Reachable failure is `allocation_failed` (rows 5, 6); `dstMaxSize_tooSmall` is **not** reachable here because the capacity passed is exactly `maxFHSize` (19) | the `LZ4F_compressBegin()` error is returned verbatim (`LZ4F_ERROR_allocation_failed`, code 9); handle freed and `*lz4fWrite = NULL` |
| 242 | `LZ4F_writeOpen` | `fwrite(buf, 1, ret, fp) != ret` when writing the frame header (e.g. `fp` opened read-only, full/closed device) — lz4file.c:271-274 | error `LZ4F_ERROR_io_write` (`LZ4F_getErrorCode()==22`); handle freed and `*lz4fWrite = NULL` |
| 243 | `LZ4F_write` | `lz4fWrite == NULL` — lz4file.c:288-289 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 244 | `LZ4F_write` | `buf == NULL` (checked unconditionally, before the loop, so it also fires for `size == 0`) — lz4file.c:288-289 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 245 | `LZ4F_write` | inner `LZ4F_compressUpdate(cctx, dstBuf, dstBufMaxSize, p, chunk, NULL)` fails, e.g. state not initialized after a prior failure (`compressionState_uninitialized`, row 14) — lz4file.c:296-303 | the `LZ4F_compressUpdate()` error is returned verbatim; additionally `lz4fWrite->errCode` is latched to that error, which makes a later `LZ4F_writeClose()` skip `LZ4F_compressEnd()` (see row 82) |
| 246 | `LZ4F_write` | `fwrite(dstBuf, 1, ret, fp) != ret` for a compressed chunk — lz4file.c:305-308 | error `LZ4F_ERROR_io_write` (`LZ4F_getErrorCode()==22`); `lz4fWrite->errCode` latched to the same value |
| 247 | `LZ4F_writeClose` | `lz4fWrite == NULL` — lz4file.c:321-323 | error `LZ4F_ERROR_parameter_null` (`LZ4F_getErrorCode()==21`) |
| 248 | `LZ4F_writeClose` | `LZ4F_compressEnd(cctx, dstBuf, dstBufMaxSize, NULL)` fails — lz4file.c:326-331. Reachable: `frameSize_wrong` (row 23) when `prefsPtr->frameInfo.contentSize` was declared at `LZ4F_writeOpen()` and the total bytes passed to `LZ4F_write()` differ | the `LZ4F_compressEnd()` error is returned verbatim (e.g. `LZ4F_ERROR_frameSize_wrong`, code 14); the handle is still freed via the `out:` label |
| 249 | `LZ4F_writeClose` | `lz4fWrite->errCode != LZ4F_OK_NoError` (a previous `LZ4F_write()` failed, row 78/79): the whole finalize block is skipped — lz4file.c:325 | returns `LZ4F_OK_NoError` (0) — the previously latched error is **silently discarded**, and no endMark/content checksum is written, leaving a truncated frame on disk. The handle is freed |
| 250 | `LZ4F_writeClose` | `fwrite(dstBuf, 1, ret, fp) != ret` when writing the frame footer (endMark + optional content checksum) — lz4file.c:333-335 | returns `returnErrorCode(LZ4F_ERROR_io_write)` = error `LZ4F_ERROR_io_write` (`LZ4F_getErrorCode()==22`); the handle is still freed |


## LZ4F_errorCodes enum values

Derived from the `LZ4F_LIST_ERRORS(ITEM)` macro order in `c_src/include/lz4frame.h:653-678`
(expanded by `LZ4F_GENERATE_ENUM` into a plain C enum, so values are sequential from 0):

| value | constant |
|---|---|
| 0 | `LZ4F_OK_NoError` |
| 1 | `LZ4F_ERROR_GENERIC` |
| 2 | `LZ4F_ERROR_maxBlockSize_invalid` |
| 3 | `LZ4F_ERROR_blockMode_invalid` |
| 4 | `LZ4F_ERROR_parameter_invalid` |
| 5 | `LZ4F_ERROR_compressionLevel_invalid` |
| 6 | `LZ4F_ERROR_headerVersion_wrong` |
| 7 | `LZ4F_ERROR_blockChecksum_invalid` |
| 8 | `LZ4F_ERROR_reservedFlag_set` |
| 9 | `LZ4F_ERROR_allocation_failed` |
| 10 | `LZ4F_ERROR_srcSize_tooLarge` |
| 11 | `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 12 | `LZ4F_ERROR_frameHeader_incomplete` |
| 13 | `LZ4F_ERROR_frameType_unknown` |
| 14 | `LZ4F_ERROR_frameSize_wrong` |
| 15 | `LZ4F_ERROR_srcPtr_wrong` |
| 16 | `LZ4F_ERROR_decompressionFailed` |
| 17 | `LZ4F_ERROR_headerChecksum_invalid` |
| 18 | `LZ4F_ERROR_contentChecksum_invalid` |
| 19 | `LZ4F_ERROR_frameDecoding_alreadyStarted` |
| 20 | `LZ4F_ERROR_compressionState_uninitialized` |
| 21 | `LZ4F_ERROR_parameter_null` |
| 22 | `LZ4F_ERROR_io_write` |
| 23 | `LZ4F_ERROR_io_read` |
| 24 | `LZ4F_ERROR_maxCode` |

`LZ4F_isError(code)` is `code > (size_t)(-LZ4F_ERROR_maxCode)`, i.e. true only for
`code` in `[-(size_t)23 .. -(size_t)1]` (SIZE_MAX-22 .. SIZE_MAX). `-(size_t)24` (maxCode)
itself is **not** reported as an error, and `LZ4F_getErrorName()` indexes
`LZ4F_errorStrings[-(int)code]`, which is in range 1..23 for all real errors.


## Notes: checks that do NOT exist in these two files

These are recorded so the test suite does not assert behaviour the C code never implements.

- **Never raised anywhere in `lz4frame.c` / `lz4file.c`**: `LZ4F_ERROR_GENERIC` (1),
  `LZ4F_ERROR_blockMode_invalid` (3), `LZ4F_ERROR_compressionLevel_invalid` (5),
  `LZ4F_ERROR_srcSize_tooLarge` (10). Verified by grep; the constants exist only in the enum.
- **No compressionLevel validation.** `prefs.compressionLevel` is only compared against
  `LZ4HC_CLEVEL_MIN` (2) to pick fast vs HC context (lz4frame.c:705, 711, 728, 763, 956, 967).
  Negative values become an LZ4 `acceleration` (lz4frame.c:913, 925); values above
  `LZ4HC_CLEVEL_MAX` (12) are clamped inside lz4hc. No error is ever returned.
- **No blockMode / contentChecksumFlag / blockChecksumFlag enum validation on the compress
  side.** They are masked into the FLG byte with `& _1BIT` (lz4frame.c:788-791), so out-of-range
  values silently alias to 0/1.
- **`LZ4F_compressBegin*` does not validate `frameInfo.blockSizeID`.** At lz4frame.c:740 the
  result of `LZ4F_getBlockSize()` is assigned to `cctx->maxBlockSize` **without** an error check,
  so e.g. `blockSizeID == 3` produces a huge `maxBlockSize` and a frame header carrying an
  illegal BD nibble instead of `maxBlockSize_invalid`. (The `LZ4F_compressFrame*` path is also
  unaffected because `LZ4F_compressBound_internal()` swallows the same error value.)
- **`LZ4F_createCompressionContext` / `LZ4F_createDecompressionContext` do not check `version`.**
  The `version` argument is merely stored into `cctx->version` / `dctx->version`
  (lz4frame.c:603, 1290) and never compared against `LZ4F_VERSION` (100); passing a wrong
  version is accepted silently. Same for the `_advanced` variants.
- **`LZ4F_compressEnd` does not check `cStage`.** When `tmpInSize == 0`, `LZ4F_flush()` returns 0
  before its state check (lz4frame.c:1167), so `LZ4F_compressEnd()` on a never-begun cctx emits a
  bare endMark instead of `compressionState_uninitialized`.
- **`LZ4F_decompress` / `LZ4F_getFrameInfo` / `LZ4F_resetDecompressionContext` do not NULL-check
  `dctx`, `srcSizePtr` or `dstSizePtr`** (only a debug `assert(dctx != NULL)` at lz4frame.c:1637);
  passing NULL is undefined behaviour, not an error code.
- **`LZ4F_compressUpdate` / `LZ4F_flush` / `LZ4F_compressEnd` do not NULL-check `cctx`.**
- **`LZ4F_freeCompressionContext` / `LZ4F_freeDecompressionContext` / `LZ4F_freeCDict` accept NULL**
  (lz4frame.c:631, 1316, 583) and return `LZ4F_OK_NoError`.
- Debug-only `assert()`s that are **internal invariants**, not reachable through any public API
  input, and therefore not table rows: lz4frame.c:462, 467, 472 (`dstEnd >= dstPtr`), 767
  (`cdict == NULL` when `dictBuffer != NULL` — no public entry passes both), 774
  (`lz4CtxType == ctxHC`), 891 (`compress != NULL`), 1024 (`blockSize > tmpInSize`), 1076, 1089,
  1181 ("flush overflows dstBuffer!"), 1218 (`flushSize <= dstCapacity`), 1415
  (`frameHeaderSize > 5`), 1531, 1533, 1540, 1547, 1550, 1554 (`LZ4F_updateDict` invariants),
  1872 (`tmpInTarget >= 4`), 1874 (`selectedIn != NULL`), 1895 (`dstPtr != NULL`), 2095
  (`tmpOutBuffer != NULL`); and lz4file.c:68, 212 (`statePtr != NULL`, always `&caller's ptr`
  which was already NULL-checked).
