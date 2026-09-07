# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Mechanically derived from `c_src/src/lib.c`.  Every `return 0` / error-string
assignment / `assert()` / range check in the file gets one row.

The library has exactly **two** kinds of rejection:

* **soft failure** — `pinflate()` returns `0` and the global `cp_error_reason`
  is set to one of six string literals.  There are exactly 6 such sites
  (grep: `cp_error_reason =`), reached through `goto cp_err`.
* **hard failure** — a live `assert()` calls `__assert_fail()`, printing
  `<prog>: <file>:<line>: <func>: Assertion \`<expr>' failed.` on stderr and
  raising `SIGABRT` (exit by signal 6).  There are exactly 10 `assert()`s
  (grep: `assert(`).  `NDEBUG` is **not** defined for the reference build —
  the `.so` imports `__assert_fail`.

`pinflate()` returns `1` on success.  There is no other return value and no
error enum.  A `NULL` `out` / `in` is *not* checked anywhere.

Test file: `tests/errors.rs` (plus `tests/fuzz_sweep.rs` for the broad sweeps).
Hard failures are exercised in a re-executed child process, which runs each case
in a `fork()`ed grandchild, so the abort — its signal *and* its stderr text — can
be observed and compared (`tests/common/mod.rs`, helper test `zz_child_runner`,
made available by `define_child_runner!()`).

**Status: all 16 rows plus all 10 generic-boundary rows have a passing
differential test.**  Verified with `cargo test` (73 tests) and, per Phase D,
under every build configuration via `./check_features.sh`.

## Soft failures — `pinflate` returns 0, `cp_error_reason` set

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 1 | `cp_stored` (lib.c:176) | stored block (`BTYPE==0`) whose `LEN != (uint16_t)~NLEN` | returns `0`; `cp_error_reason == "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `err01_stored_len_nlen_mismatch` | [x] |
| 2 | `cp_stored` (lib.c:185) | stored block where `s->bits_left / 8 > (int)LEN`, i.e. more input bits remain after the LEN/NLEN header than `LEN` says (e.g. `LEN` smaller than the trailing input) | returns `0`; `cp_error_reason == "Stored block extends beyond end of input stream."` | `err02_stored_block_beyond_input` | [x] |
| 3 | `cp_block` (lib.c:260) | literal symbol decoded while `s->out + 1 > s->out_end` (output buffer full / `out_bytes == 0`) | returns `0`; `cp_error_reason == "Attempted to overwrite out buffer while outputting a symbol."` | `err03_out_full_on_literal` | [x] |
| 4 | `cp_block` (lib.c:279) | length/distance pair whose `s->out - backwards_distance < s->begin` (back-reference before the start of the output buffer) | returns `0`; `cp_error_reason == "Attempted to write before out buffer (invalid backwards distance)."` | `err04_backwards_distance_before_begin` | [x] |
| 5 | `cp_block` (lib.c:288) | length/distance pair whose `s->out + length > s->out_end` (match longer than remaining output space) | returns `0`; `cp_error_reason == "Attempted to overwrite out buffer while outputting a string."` | `err05_match_overruns_out` | [x] |
| 6 | `pinflate` (lib.c:362) | block header with `BTYPE == 3` (`0b11`, reserved) | returns `0`; `cp_error_reason == "Detected unknown block type within input stream."` | `err06_btype_3_reserved` | [x] |

Ordering note (row 4 before row 5): `cp_block` checks the *backwards distance*
first and the *output overrun* second, so an input that violates **both** must
report row 4's message.  Covered by `err04b_both_violated_reports_distance`.

## Hard failures — `assert()` → `__assert_fail` → `SIGABRT`

All of these are compared in a forked grandchild (see
`tests/common/mod.rs::child_runner_main`), which records the signal *and* the
redirected stderr, so the check is that C and Rust produce the **same signal and
byte-identical assertion text** — not merely that both died.

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|------------------------------------------|-------------------|------|-----|
| 7 | `cp_ptr` (lib.c:95) | `assert(!(s->bits_left & 7))`. `count` and `bits_left` are congruent mod 8 everywhere *except* after `cp_peak_bits`'s final-word fold, which does `count += bits_left` instead of `count += 8 * last_bytes`. Reaching the assert needs a **non-final fixed block** that drives that fold with `bits_left ≢ 0 (mod 8)`, followed by a **stored block** whose misaligned `LEN`/`NLEN` still read as complements. Derived stream: 15 bytes, `last_bytes == 3`, 9 literals (78 bits) + EOB so the fold happens at `count == 15, bits_left == 39`, then `BFINAL=1, BTYPE=0`, 4 pad bits, `LEN = 0xFFFF`, `NLEN` low 9 bits `0`. | `SIGABRT`, stderr `… lib.c:95: cp_ptr: Assertion \`!(s->bits_left & 7)' failed.` | `abort07_cp_ptr_misaligned` (hits it with `row7/derived/p0/lffff`) | [x] |
| 8 | `cp_peak_bits` (lib.c:104) | `assert(s->word_index <= s->word_count)` — the increment is guarded by `word_index < word_count`, so the post-condition cannot fail. **Structurally unreachable.** | *n/a* | `unreachable_asserts_are_never_hit` asserts line 104 never fires across ~33 000 cases | [x] |
| 9 | `cp_consume_bits` (lib.c:115) | `assert(s->count >= num_bits_to_read)` — a `cp_read_bits`/`cp_decode` that passes the `would_overflow` pre-check but still has fewer bits *buffered* than requested. Deterministic case: a stored-block header truncated inside `LEN`/`NLEN` (`BFINAL=1, BTYPE=0`, aligned, then a single `0x00` byte). | `SIGABRT`, `… lib.c:115: cp_consume_bits: Assertion \`s->count >= num_bits_to_read' failed.` | `abort09_consume_bits_underflow` (deterministic case **and** a searched case) | [x] |
| 10 | `cp_read_bits` (lib.c:123) | `assert(num_bits_to_read <= 32)`. The internal call sites pass at most 16, so this is only reachable through the **exported, writable** tables: setting `cp_dist_extra_bits[0]` or `cp_len_extra_bits[0]` above 32 and then decoding a match. | `SIGABRT`, `… lib.c:123: cp_read_bits: Assertion \`num_bits_to_read <= 32' failed.` | `abort10_read_bits_gt_32` (values 33/40/64/255 on both tables) | [x] |
| 11 | `cp_read_bits` (lib.c:124) | `assert(num_bits_to_read >= 0)` — the argument is always either a `uint8_t` table entry (0..255) or `s->count & 7` (0..7), both non-negative after integer promotion. **Structurally unreachable.** | *n/a* | `unreachable_asserts_are_never_hit` asserts line 124 never fires | [x] |
| 12 | `cp_read_bits` (lib.c:125) | `assert(s->bits_left > 0)` — **`in_bytes == 0`** (or negative), and also any stream that consumes every input bit and keeps decoding. | `SIGABRT`, `… lib.c:125: cp_read_bits: Assertion \`s->bits_left > 0' failed.` | `abort12_zero_length_input`, `abort_g04_negative_in_bytes`, `abort12_bits_exhausted` | [x] |
| 13 | `cp_read_bits` (lib.c:126) | `assert(s->count <= 64)` — `count` only grows by 32 per word load, and a load only happens when `count < num_bits_to_read ≤ 16`, so `count ≤ 47`. **Structurally unreachable.** | *n/a* | `unreachable_asserts_are_never_hit` asserts line 126 never fires | [x] |
| 14 | `cp_read_bits` (lib.c:127) | `assert(!cp_would_overflow(s, num_bits_to_read))` — `(bits_left + count) - num_bits < 0`, i.e. asking for more bits than the whole remaining stream can supply (truncated stream). | `SIGABRT`, `… lib.c:127: cp_read_bits: Assertion \`!cp_would_overflow(s, num_bits_to_read)' failed.` | `abort14_would_overflow` (found by the shared scan) | [x] |
| 15 | `cp_build` (lib.c:154) | `assert(len < 16)` — a code length ≥ 16 in the symbol-length array. A dynamic block cannot produce one (16/17/18 are repeat codes), but `cp_fixed` reads the **exported, writable** `cp_fixed_table`, so poisoning any of its 320 entries with a value ≥ 16 reaches it — in both the literal half (built with `s != NULL`) and the distance half (built with `s == NULL`). | `SIGABRT`, `… lib.c:154: cp_build: Assertion \`len < 16' failed.` | `abort15_build_len_ge_16` (indices 0/5/100/287 and 288/300/319) | [x] |
| 16 | `cp_decode` (lib.c:217) | `assert((search >> len) == (key >> len))`, `len = 32 - (key & 0xF)` — the binary search landed on a key that does not prefix-match the stream. Reached when `lo` ends at `0` (so `tree[-1]`, an aliased neighbouring struct member, is used as the key) or when the tree is incomplete/over-subscribed. **Note:** `key & 0xF == 0` gives `len == 32`, and `uint32_t >> 32` is UB in C; gcc/x86 emits `shr`, which masks the count to 5 bits (i.e. `>> 0`). The translation uses `wrapping_shr`, matching gcc. This is by far the most frequently reached assert (≈ 8 400 of the sweep cases). | `SIGABRT`, `… lib.c:217: cp_decode: Assertion \`(search >> len) == (key >> len)' failed.` | `abort16_decode_prefix_mismatch` (searched case **and** a hand-built empty-literal-tree dynamic block) | [x] |

**Assertion lines actually reached by the C library across the whole suite:**
`95, 115, 123, 125, 127, 154, 217`.  Lines `104`, `124` and `126` are never
reached, exactly as rows 8, 11 and 13 claim.

## Generic FFI boundaries (not among the C source's own checks)

| # | condition | expected C result | test | [x] |
|---|-----------|-------------------|------|-----|
| G1 | `in_bytes == 0` with non-NULL `in` | row 12 — `SIGABRT` at lib.c:125 | `abort12_zero_length_input` (all 4 alignments) | [x] |
| G2 | `out_bytes == 0` with a stream that emits a literal | row 3 — returns `0`, `cp_error_reason` set | `err03_out_full_on_literal` | [x] |
| G3 | `out == NULL`, `out_bytes == 0`, a stream that emits nothing (empty fixed block, empty stored block) | returns `1`; `out` is never dereferenced | `g03_null_out_empty_block` | [x] |
| G4 | `in_bytes < 0` (`-1, -3, -4, -1000`) → `bits_left < 0` | row 12 — `SIGABRT` at lib.c:125 | `abort_g04_negative_in_bytes` | [x] |
| G5 | `out_bytes < 0` (`-1, -2, -1000, i32::MIN/2`) → `out_end < out` | first literal fails row 3 → returns `0`; an *empty* block still returns `1` | `g05_negative_out_bytes` | [x] |
| G6 | `in` misaligned by 0/1/2/3 bytes (the `first_bytes` prologue) crossed with `in_bytes % 4` ∈ {0,1,2,3} (the `final_word` tail) | identical decode for every one of the 16 combinations | every `CONFIGS.md` row via `diff_all_alignments` / `diff_matrix` | [x] |
| G7 | `BTYPE`: 2 bits, so 0/1/2/3 is the **whole** value space — there is no out-of-range value. `cp_dynamic`'s `switch (sym)` has no `default` guard either: every code-length symbol other than 16/17/18 is treated as a literal length, including values > 18 from a corrupt code-length tree. | see rows 6 / 15 / 16 | `g07_btype_all_four` (all 4 × BFINAL × 4 alignments × random payloads), `cfg38_dynamic_mixed_codelength_sweep` | [x] |
| G8 | Out-of-range *symbol* from `cp_decode`: it returns `(key >> 4) & 0xFFF`, i.e. 0..4095, while `cp_len_extra_bits`/`cp_len_base` hold 31 entries — `symbol - 257 > 30` reads past the end of the table. | reads whatever the linker placed after the array | see the note below | [x] |
| G9 | `out == NULL` with a stream that **does** emit a literal | both libraries dereference NULL → `SIGSEGV` | `cfg36c_argument_sweep` (release cdylib); excluded from the *debug* cdylib configuration, see below | [x] |
| G10 | Stored block whose `LEN` exceeds `out_bytes`: `cp_stored`'s `memcpy` has **no** `out_end` check, so up to 65535 bytes are written past the caller's buffer | writes the source bytes past the end; returns `1` | `cfg39_stored_overrun_sweep` (2 000 cases; the harness gives the buffer `OUT_SLACK` = 64 KiB + 64 of room so the overrun is *compared*) | [x] |

### The unchecked `memcpy` in `cp_stored` (row G10)

This is worth calling out because it shaped the harness.  `cp_stored` validates
`LEN` only against the *input* (`bits_left / 8 <= LEN`), never against
`out_bytes`, and then does `memcpy(s->out, p, LEN)`.  A stored block can
therefore write up to 65535 bytes beyond the caller's output buffer.  Letting
that land on the test process's heap produced allocator failures
(`munmap_chunk(): invalid pointer` in one run, `realloc(): invalid old size` in
the other) at *different* moments in the two libraries — a false "divergence"
that said nothing about the translation.  `common::OUT_SLACK` gives the output
buffer 64 KiB + 64 bytes of slack so the overrun stays inside the harness's own
allocation and every byte it writes is part of the compared digest.

### Note on out-of-bounds table reads (row G8)

`cp_block` indexes `cp_len_extra_bits[symbol - 257]` with no bound check, and
`cp_decode` can return a symbol up to 4095 when it falls back on `tree[-1]`.
Both libraries perform the same out-of-bounds *read*; the bytes found there are
whatever the **linker** placed after the array, and the two linkers order the
globals differently (see `SYMBOLS.md`).  This is unconstrained UB in C and is not
part of the library's ABI, so the tests do not require agreement past the end of
a table.  In practice `cp_decode`'s assert (row 16) fires first for essentially
every such input, which the sweeps confirm: ≈ 33 000 random, corrupted,
truncated and table-poisoned cases produce **zero** divergences.

### Note on infinite loops

`cp_dynamic` writes `lens[n]` with no bound check while `n` can advance by up to
138 per code-length symbol, so for some corrupt headers `n` runs past
`lens[288 + 32]` and clobbers `cp_dynamic`'s own locals — including `n` itself,
which makes the loop never terminate.  This is real, reachable behaviour of the
C library (1 041 of the 8 000 `cfg37`/`cfg38` cases hang), and the Rust
translation reproduces gcc -O0's frame layout so that it hangs on exactly the
same inputs.  Each case therefore runs under a small `setitimer` watchdog and a
hang is recorded as a comparable outcome (`HANG`) rather than being fatal.

### Note on the `assert()` message text

glibc's `assert` prints `__FILE__`.  CMake compiles `lib.c` with an **absolute**
path, so the C library's message contains `/…/c_src/src/lib.c`.  `build.rs`
reproduces that exact path for the Rust translation (it canonicalises
`../c_src/src/lib.c` relative to `CARGO_MANIFEST_DIR`), so the two libraries'
stderr is byte-identical, not merely "both aborted".

### Note on the debug-profile configuration

`check_features.sh` additionally runs the whole suite against a **debug**-built
cdylib (integer-overflow checks and `debug_assert!`s on).  All 73 tests pass
there too, which is what rules out the translation being accidentally correct
only because release mode wraps silently.  The single exception is row G9: a
debug Rust build traps a null dereference with its own check and aborts, where
C and the release build take `SIGSEGV`.  That is a property of the Rust debug
profile, not of the translation, so `check_features.sh` sets
`PINFLATE_SKIP_NULL_OUT=1` for that configuration (the non-dereferencing
null-`out` case, G3, still runs everywhere).
