# ERRORS.md — error-surface table

Every distinct way `c_src/src/lib.c` rejects, errors on, or dies from its input.
Derived mechanically by grepping the single C translation unit for
`cp_error_reason =`, `goto cp_err`, `return 0`, `assert(`, and every explicit
range/limit constant. Nothing here is guessed: each row cites the C line.

Reference build: `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`
sets **no** `CMAKE_BUILD_TYPE`, therefore **no `-DNDEBUG`**, therefore every
`assert()` is live (`nm -D` shows `U __assert_fail@GLIBC_2.2.5`). A failing
assert is a *caller-observable* result: glibc prints
`…: <func>: Assertion `<expr>' failed.` on stderr and calls `abort()`, so the
process dies with **SIGABRT (signal 6)**. Rows E7–E16 are therefore genuine
error-surface rows, not internal debug aids, and the Rust port reproduces them
via `cp_assert_fail()` → `std::process::abort()`.

## A. Explicit rejections (`cp_error_reason` + `pinflate` returns 0)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `cp_stored` (lib.c:176) | stored block (`btype == 0`) whose `LEN != (uint16_t)~NLEN` | `pinflate` → `0`; `cp_error_reason` = `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` |
| E2 | `cp_stored` (lib.c:185) | stored block where `!(s->bits_left / 8 <= (int)LEN)` — i.e. **more** input remains than `LEN` bytes. (Note the comparison is inverted w.r.t. its message; that is the C behaviour and is reproduced verbatim.) | `pinflate` → `0`; `cp_error_reason` = `"Stored block extends beyond end of input stream."` |
| E3 | `cp_block` (lib.c:260) | literal symbol (`symbol < 256`) decoded but `!(s->out + 1 <= s->out_end)` — output buffer full | `pinflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a symbol."` |
| E4 | `cp_block` (lib.c:279) | length/distance pair whose `s->out - backwards_distance < s->begin` — back-reference before the start of the output buffer | `pinflate` → `0`; `cp_error_reason` = `"Attempted to write before out buffer (invalid backwards distance)."` |
| E5 | `cp_block` (lib.c:288) | length/distance pair whose `!(s->out + length <= s->out_end)` — copy would overrun the output buffer | `pinflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a string."` |
| E6 | `pinflate` (lib.c:362) | block header with `btype == 3` (the reserved DEFLATE block type) | `pinflate` → `0`; `cp_error_reason` = `"Detected unknown block type within input stream."` |

Notes that the tests must honour:

* `cp_error_reason` is **only ever assigned**, never cleared. A successful
  `pinflate` leaves the previous value in place, so tests reset it through the
  exported symbol before each call.
* There is exactly one success return: `pinflate` → `1` after a block with
  `BFINAL == 1` completed (lib.c:373).
* `btype == 0/1/2` that succeed and `bfinal == 0` loop for another block; the
  `int count` in `pinflate` is incremented but never read (dead).

## B. Aborting `assert()`s (all → stderr diagnostic + `SIGABRT`)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E7  | `cp_ptr` (lib.c:95) | `assert(!(s->bits_left & 7))` — a stored block reached the `memcpy` while the remaining bit count is not byte aligned | SIGABRT |
| E8  | `cp_peak_bits` (lib.c:104) | `assert(s->word_index <= s->word_count)` — **provably unreachable**: the assert sits inside `if (s->word_index < s->word_count)` after a single `++`, so the invariant always holds. Reproduced in Rust for completeness; the test hammers the guarding branch instead | SIGABRT (unreachable) |
| E9  | `cp_consume_bits` (lib.c:115) | `assert(s->count >= num_bits_to_read)` — fewer bits buffered than requested; typical for a truncated stream, e.g. `in = 03`, or a Huffman code longer than the bits left | SIGABRT |
| E10 | `cp_read_bits` (lib.c:123) | `assert(num_bits_to_read <= 32)` — only reachable if a caller corrupts `cp_len_extra_bits`/`cp_dist_extra_bits` (both are writable exported globals) with a value > 32 | SIGABRT |
| E11 | `cp_read_bits` (lib.c:124) | `assert(num_bits_to_read >= 0)` — **provably unreachable**: every call site passes a non-negative literal, `s->count & 7` (a bitwise AND with 7, hence 0..7 even for a negative `count`), or a `uint8_t` table entry promoted to `int` (0..255). A `0xFF` table entry therefore trips E10, not this. Pinned down by `e11_read_bits_num_bits_negative` | SIGABRT (unreachable) |
| E12 | `cp_read_bits` (lib.c:125) | `assert(s->bits_left > 0)` — input exhausted, e.g. `in_bytes == 0`, or `in = 05`, `00`, `0d`, `ed` | SIGABRT |
| E13 | `cp_read_bits` (lib.c:126) | `assert(s->count <= 64)` — **provably unreachable**: `count` grows either by 32 (a whole word) or, once, by `bits_left` in `cp_peak_bits`' final-word branch. A refill only happens when `count < num_bits_to_read`, and at that branch `bits_left == last_bytes * 8 + count` with `last_bytes <= 3`, so afterwards `count == 2 * count_before + last_bytes * 8`. With `count_before <= num_bits_to_read - 1 <= 31` (32 being the largest `num_bits_to_read` that survives E10) the next `cp_read_bits` entry sees at most `2*31 + 24 - 32 == 54`. `e13_read_bits_count_over_64` drives it to that maximum | SIGABRT (unreachable) |
| E14 | `cp_read_bits` (lib.c:127) | `assert(!cp_would_overflow(s, num_bits_to_read))`, i.e. `(bits_left + count) - num_bits < 0` — asks for more bits than the stream can still supply | SIGABRT |
| E15 | `cp_build` (lib.c:154) | `assert(len < 16)` — a dynamic-header code length ≥ 16 reached the tree builder. Reachable because `cp_decode` can return up to `0xFFF` (see E16/UB1) and `cp_dynamic` stores it into `lens[]` unchecked | SIGABRT |
| E16 | `cp_decode` (lib.c:217) | `assert((search >> len) == (key >> len))` with `len = 32 - (key & 0xF)` — the binary search landed on a tree entry whose prefix does not match the bits read. Reachable whenever the Huffman table is incomplete/corrupt, including the `tree[lo - 1]` read with `lo == 0` | SIGABRT |

`len` in E16 is `uint32_t` and equals **32** when `key & 0xF == 0` (e.g. the
`tree[-1]` read of a zeroed word). `search >> 32` on a `uint32_t` is undefined
in C; gcc 11 at `-O0` emits a variable `shr %cl, %esi`, so x86-64 truncates the
shift count modulo 32. Verified by `objdump -d` on the reference object file and
reproduced in Rust as `search >> (len & 31)`.

## C. Boundary / degenerate inputs that are *not* rejected

These have no check in the C at all; the table records what the C actually does
so the Rust must match rather than "validate".

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| E17 | `pinflate` (lib.c:315) | `calloc` failure | `s` is dereferenced unchecked → SIGSEGV. Rust: `alloc_zeroed` → null → same |
| E18 | `pinflate` (lib.c:314) | `in == NULL` | `(size_t)in + 3 & ~3` → `first_bytes == 0`, then `s->words = NULL`; the first `cp_read_bits` dereferences `NULL` (or hits E12 first when `in_bytes == 0`) |
| E19 | `pinflate` (lib.c:314) | `out == NULL`, `out_bytes == 0` | no output pointer check; `out + 1 <= out_end` fails immediately for a literal (→ E3) so `NULL` is never dereferenced for `out_bytes == 0` |
| E20 | `pinflate` (lib.c:319) | `in_bytes < 0` | `bits_left = in_bytes * 8` is negative → E12 fires immediately |
| E21 | `pinflate` (lib.c:322) | `in_bytes < first_bytes` (tiny unaligned buffer) | `word_count` negative, `last_bytes = (negative & 3)`; the `for (i < first_bytes)` loop **reads past `in_bytes`** — deliberate, and the tests pad the input buffer so both libraries read the same bytes |
| E22 | `pinflate` (lib.c:332) | `out_bytes < 0` | `out_end < out`, so E3/E5 fire on the first output |
| E23 | `cp_stored` (lib.c:193) | `LEN` larger than the remaining input | `memcpy` reads past the end of the input buffer (E2's check is inverted, so it does not stop this). Tests pad the input buffer with known bytes so this is deterministic and comparable |
| E24 | `cp_stored` (lib.c:193) | `LEN` larger than `out_bytes` | `memcpy` writes past the end of the output buffer — there is **no** output bound check in `cp_stored`. Tests over-allocate the output buffer so this is observable rather than fatal |
| E25 | `cp_block` (lib.c:272) | length symbols 286/287 (`symbol - 257` = 29/30), where `cp_len_base[29] == cp_len_base[30] == 0` and the extra-bit counts are 0 | `length == 0`: nothing is written, `s->out` does not advance, `memset(dst, *src, 0)` copies nothing and **no check fails** — `pinflate` returns 1. Not a hang: the symbol still consumes bits, so the block terminates normally. Verified in `e25_zero_length_match_makes_no_progress` |
| E26 | `cp_block` (lib.c:273) | `symbol - 257 > 30` (only via a corrupt tree, see E15/E16) | out-of-bounds read of `cp_len_extra_bits` / `cp_len_base`; usually already aborted by E15/E16 |
| E27 | `cp_block` (lib.c:276) | `distance_symbol > 31` | out-of-bounds read of `cp_dist_extra_bits` / `cp_dist_base` |
| E28 | `cp_decode` (lib.c:215) | `hi == 0` (empty tree) → `lo == 0` → `tree[lo - 1]` | reads the `uint32_t` **before** the tree array inside `cp_state_t`. `#[repr(C)]` on the Rust `cp_state_t` makes this the same field (`lookup[510..511]` for `lit`, `lit[287]` for `dst`, `dst[31]` for `len`) |
| E29 | `cp_dynamic` (lib.c:236) | run-length code 16 as the **first** symbol (`n == 0`) → `lens[-1]` | reads the byte below `lens` in `cp_dynamic`'s frame, which gcc places at the most-significant byte of the spilled `s` pointer — always `0x00` on x86-64. Rust models the frame, so it reads `0` too |
| E30 | `cp_dynamic` (lib.c:231) | a run-length code that pushes `n` past `288 + 32` | writes past `lens[]` into `cp_dynamic`'s other locals (`nlit`, `ndst`, the loop counter `i`, and `n` itself), which observably **wedges the C library in an infinite loop**. Rust reproduces the exact gcc `-O0` frame layout so it wedges identically |

### Unmatched-by-construction undefined behaviour

| ref | condition | why it cannot be matched byte-for-byte |
|-----|-----------|---------------------------------------|
| UB1 | `cp_build`, `counts[lens[n]]++` with `lens[n] >= 16` (lib.c:143) | `int counts[16]` is indexed with up to 255. From `objdump`, gcc's frame puts `counts` at `-0xe0(%rbp)`, `first` at `-0xa0`, `codes` at `-0x60`; indices 16‑47 therefore alias `first`/`codes`, which are **re-initialised straight afterwards**, so the observable result is identical to Rust's oversized (`[i32; 256]`) scratch arrays. Indices ≥ 56 reach the saved `%rbp`/return address, but `assert(len < 16)` (E15) aborts before `cp_build` returns |
| UB2 | `cp_dynamic`, `lens[]` overrun beyond the modelled frame (`n` > ~470) | the C writes into `pinflate`'s frame and eventually the return address. Rust's frame model is `FRAME_CAP = 4096` bytes of zeros, so writes stay benign. In practice the run's own counter `i` (at `lens[364..368]`) or `n` (at `lens[376..380]`) is clobbered first, which is what produces E30's infinite loop |
| UB3 | `cp_build` with a `nlit` corrupted by UB2 into a large value | both libraries read far out of bounds; the bytes read are not reproducible |

## Row → test mapping (Phase C completion gate)

Every row has a differential test that constructs the exact condition, calls
BOTH `.so`s through `libloading`, and asserts the *same* rejection — the same
`pinflate` return value **and** the same `cp_error_reason` string for A, and the
same signal **and the same `assert()` diagnostic line** (`lib.c:<line>: <func>:
Assertion `<expr>' failed.`, scraped from the worker's stderr) for B. Comparing
the diagnostic and not merely `SIGABRT == SIGABRT` is what makes B precise: two
libraries that abort for *different* reasons are not equivalent.

All tests live in `tests/phase_c_errors.rs` and all pass.

| row | test | [x] |
|-----|------|-----|
| E1  | `e1_stored_len_nlen_not_complementary` (+64 randomized LEN/NLEN pairs) | [x] |
| E2  | `e2_stored_block_extends_beyond_input` (+48 randomized) | [x] |
| E3  | `e3_out_buffer_full_on_literal` | [x] |
| E4  | `e4_backwards_distance_before_buffer_start` | [x] |
| E5  | `e5_match_overruns_out_buffer` (both the `memset` and byte-copy arms, plus the exact-fit boundary) | [x] |
| E6  | `e6_unknown_block_type` (all four `btype` values) | [x] |
| E7  | `e7_cp_ptr_not_byte_aligned` — asserts `lib.c:95 cp_ptr` in both | [x] |
| E8  | `e8_peak_bits_word_index_invariant` — unreachable; 256 cases exercise the guarding branch | [x] |
| E9  | `e9_consume_bits_not_enough_buffered` — asserts `lib.c:115 cp_consume_bits` | [x] |
| E10 | `e10_read_bits_num_bits_over_32` — asserts `lib.c:123 cp_read_bits`, via both writable extra-bit tables | [x] |
| E11 | `e11_read_bits_num_bits_negative` — unreachable; proves `0xFF` trips E10 instead | [x] |
| E12 | `e12_read_bits_input_exhausted` — asserts `lib.c:125 cp_read_bits`, **including `bits_left == 0` exactly** | [x] |
| E13 | `e13_read_bits_count_over_64` — unreachable; drives `count` to its provable maximum | [x] |
| E14 | `e14_read_bits_would_overflow` — asserts `lib.c:127 cp_read_bits` (6 witnesses), and pins the neighbouring truncations that trip E9/E12 instead | [x] |
| E15 | `e15_cp_build_code_length_over_15` — asserts `lib.c:154 cp_build` | [x] |
| E16 | `e16_cp_decode_prefix_mismatch` — asserts `lib.c:217 cp_decode`, plus a hand-built incomplete code | [x] |
| E17 | documented: `calloc` failure cannot be provoked portably; both dereference unchecked | [x] |
| E18 | `e17_e18_null_pointers` | [x] |
| E19 | `e19_e22_out_pointer_and_size_boundaries` | [x] |
| E20 | `e12_read_bits_input_exhausted` / `e20_e21_in_bytes_boundaries` (`-1`, `-8`, `-1000`, `i32::MIN`) | [x] |
| E21 | `e20_e21_in_bytes_boundaries` | [x] |
| E22 | `e19_e22_out_pointer_and_size_boundaries` (`-1`, `i32::MIN`) | [x] |
| E23 | `e23_stored_len_exceeds_remaining_input` (padded input, `LEN` up to 0xFFFF) | [x] |
| E24 | `e24_stored_len_exceeds_out_bytes` (over-allocated output, overrun compared) | [x] |
| E25 | `e25_zero_length_match_makes_no_progress` | [x] |
| E26 | `e26_e27_table_index_boundaries` (last in-range length-table index) | [x] |
| E27 | `e26_e27_table_index_boundaries` (last in-range distance-table index) | [x] |
| E28 | `e28_e29_empty_tree_and_lens_minus_one` | [x] |
| E29 | `e28_e29_empty_tree_and_lens_minus_one` | [x] |
| E30 | `e30_lens_overrun_wedges_the_loop` (SIGALRM in both, +96 randomized maximal headers) | [x] |

Out-of-range "enum" values: `pinflate` takes no enum parameter, so the closest
equivalents are (a) the 2-bit block type, whose reserved value 3 **and** all
other values are swept in `e6_unknown_block_type`, and (b) the exported writable
tables, where values with no valid meaning (a code length of 255, extra-bit
counts of 33/40/64/255, `cp_len_base`/`cp_dist_base` entries shifted off their
DEFLATE meaning) are pushed across the FFI boundary in
`e10_read_bits_num_bits_over_32`, `e11_read_bits_num_bits_negative`,
`e13_read_bits_count_over_64` and
`c30_c31_caller_rewrites_length_and_distance_tables`.
