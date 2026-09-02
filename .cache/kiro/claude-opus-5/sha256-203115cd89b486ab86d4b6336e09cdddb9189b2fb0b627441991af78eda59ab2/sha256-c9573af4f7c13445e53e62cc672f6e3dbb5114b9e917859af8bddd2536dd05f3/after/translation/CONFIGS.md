# CONFIGS.md — Phase B configuration-surface table

## Axes, derived mechanically from the C source

`c_src/include/lib.h` declares exactly one public entry point, and it is also
the lowest-level one — there is no convenience wrapper layer to hide behind:

```c
const char** UTIL_createLinePointers(char* buffer, size_t numLines, size_t bufferSize);
```

There are no runtime option/mode/flag setters, no global state, no `#ifdef`
blocks, and no feature gates:

```
$ grep -c '#if\|#ifdef\|#ifndef\|static ' c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:0
c_src/include/lib.h:0
$ grep -n 'if\|while' c_src/src/lib.c        # every branch in the library
10:    if (bufferPtrs == NULL) return NULL;
12:    while (lineIndex < numLines && pos < bufferSize) {
18:        while ((pos + len < bufferSize) && buffer[pos + len] != '\0') {
24:        if (pos < bufferSize) pos++;
29:    if (lineIndex != numLines) {
```

The configuration surface is therefore entirely the **input shape**. The branch
sites above give these distinguishing axes:

* **A1 — `numLines`**: `0` / `1` / many. Controls the outer-loop guard
  `lineIndex < numLines`, the `malloc` size, and the final
  `lineIndex != numLines` verdict.
* **A2 — `bufferSize`**: `0` / `1` / small / exactly the record bytes / larger
  than the record bytes. Controls the outer guard `pos < bufferSize`, the inner
  guard `pos + len < bufferSize`, and the `if (pos < bufferSize) pos++`
  NUL-skip.
* **A3 — record termination**: last record NUL-terminated (`pos < bufferSize`
  after `pos += len`, so the NUL is skipped) vs. unterminated / truncated by
  `bufferSize` (inner loop exits on `pos + len < bufferSize`, and the NUL-skip
  `if` is **not** taken).
* **A4 — empty records**: `buffer[pos] == '\0'`, so the inner `while` body never
  runs and `len == 0`. Includes a leading NUL, runs of consecutive NULs, and an
  all-NUL buffer.
* **A5 — `numLines` vs. records available**: fewer than available (loop exits on
  `lineIndex == numLines` with `pos < bufferSize`, trailing bytes ignored),
  exactly equal, more than available (→ error, see `ERRORS.md`).
* **A6 — byte values in the records**: ASCII, high bytes `0x80..0xFF` (relevant
  because `char` is signed on x86-64 Linux and the C compares
  `buffer[i] != '\0'`), and `0x01` sentinels.

Rows below are the cross-product of A1–A6 pruned to the combinations the C
actually treats differently. Every row is exercised through **both** `.so`
exports with many randomized inputs (fixed seed, see
`tests/differential.rs::SEED`), and the full returned pointer array is compared
element-by-element.

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|--------------------------------------------|-----|
| C1  | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize = 0`, `buffer` = valid non-empty alloc — zero-element result, `malloc(0)` | [x] |
| C2  | `UTIL_createLinePointers` | `numLines = 0`, `bufferSize` random > 0, buffer with random NULs — loop guard A1 short-circuits, nothing read | [x] |
| C3  | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = 1`, buffer = `"\0"` (single empty record, terminated) — `len == 0`, NUL-skip **not** taken (`pos == 1 == bufferSize`) | [x] |
| C4  | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize = 1`, buffer = one non-NUL byte (unterminated record, truncated by `bufferSize`) | [x] |
| C5  | `UTIL_createLinePointers` | `numLines = 1`, one NUL-terminated record, `bufferSize` = exactly `len+1` — NUL-skip taken then loop ends on `pos == bufferSize` | [x] |
| C6  | `UTIL_createLinePointers` | `numLines = 1`, one record, `bufferSize` = exactly `len` (NUL excluded) — record runs to buffer end, NUL-skip not taken | [x] |
| C7  | `UTIL_createLinePointers` | `numLines = N` (2..16), exactly `N` NUL-terminated records, `bufferSize` = exact total incl. final NUL | [x] |
| C8  | `UTIL_createLinePointers` | `numLines = N`, exactly `N` records but the **final** one unterminated, `bufferSize` = exact total excl. final NUL | [x] |
| C9  | `UTIL_createLinePointers` | `numLines = N`, more than `N` records present, `bufferSize` covers all — loop exits on `lineIndex == numLines`, trailing bytes ignored (A5 "fewer than available") | [x] |
| C10 | `UTIL_createLinePointers` | `numLines = N`, all-empty records (`bufferSize` NULs) — every iteration has `len == 0` | [x] |
| C11 | `UTIL_createLinePointers` | `numLines = N`, leading NUL then random records (first record empty, rest non-empty) | [x] |
| C12 | `UTIL_createLinePointers` | `numLines = N`, random runs of consecutive NULs interleaved with random records (mixed empty/non-empty) | [x] |
| C13 | `UTIL_createLinePointers` | `numLines = N`, record bytes drawn from the full `0x01..0xFF` range incl. high/negative `char` values (A6 sign-extension check) | [x] |
| C14 | `UTIL_createLinePointers` | `numLines = N`, records terminated but `bufferSize` truncates **mid-record** at a random offset (A3 × A2 interaction; may succeed or return NULL depending on the cut — both asserted equal) | [x] |
| C15 | `UTIL_createLinePointers` | fully randomized fuzz: random `bufferSize` 0..64, random NUL density 0..100 %, random `numLines` 0..20 — the unpruned A1×A2×A3×A4×A6 cross-product | [x] |
| C16 | `UTIL_createLinePointers` | `numLines = 1`, `bufferSize` large (4096) single long unterminated record — inner loop iterates to the buffer end | [x] |
| C17 | `UTIL_createLinePointers` | large `numLines` (256/1024) with exactly that many single-byte-plus-NUL records — many-iteration allocation and fill | [x] |
| C18 | `UTIL_createLinePointers` | `numLines = N` with `bufferSize` **greater** than the bytes needed, extra trailing garbage after the `N`-th record (loop exits before reading it) | [x] |

## Test mapping and results

| row | test | randomized inputs |
|-----|------|-------------------|
| C1  | `cfg_c1_zero_lines_zero_size` | deterministic corner |
| C2  | `cfg_c2_zero_lines_random_buffer` | 512 |
| C3  | `cfg_c3_single_empty_record` | deterministic corner |
| C4  | `cfg_c4_single_unterminated_byte` | 255 |
| C5  | `cfg_c5_one_record_exact_with_nul` | 512 |
| C6  | `cfg_c6_one_record_nul_excluded` | 512 |
| C7  | `cfg_c7_n_records_exact` | 512 |
| C8  | `cfg_c8_n_records_last_unterminated` | 512 |
| C9  | `cfg_c9_fewer_lines_than_records` | 512 |
| C10 | `cfg_c10_all_empty_records` | 256 × 2 |
| C11 | `cfg_c11_leading_nul` | 512 |
| C12 | `cfg_c12_mixed_empty_and_nonempty` | 1024 |
| C13 | `cfg_c13_high_byte_values` | 255 exhaustive byte sweep + 512 random |
| C14 | `cfg_c14_truncated_mid_record` | 2048 (asserts both verdicts occur) |
| C15 | `cfg_c15_full_fuzz` | 20 000 |
| C16 | `cfg_c16_long_unterminated` | 3 deterministic corners |
| C17 | `cfg_c17_many_records` | numLines 256 and 1024 |
| C18 | `cfg_c18_trailing_garbage_not_read` | 512 |

All 18 rows pass. Every row calls both `.so` exports with the **same** buffer
address, so the stored `buffer + pos` values must be bit-identical, not merely
equivalent offsets; the whole returned array is compared element by element.
`SEED` in `tests/differential.rs` fixes the PRNG (SplitMix64) and each row uses
its own stream, so runs are reproducible.

## Beyond the table: exhaustive enumeration

Sampling can miss a specific byte pattern, so the small-input space is
enumerated in full rather than sampled:

* `exhaustive_binary_alphabet` — every buffer over `{NUL, 'a'}` of length 0..=12
  (so every possible NUL placement), crossed with every `bufferSize` in `0..=len`
  and every `numLines` in `0..=len+1`: **1 294 334** input triples.
* `exhaustive_ternary_alphabet_with_high_byte` — same over `{0x00, 0x01, 0xFF}`
  for length 0..=8, adding the negative-as-signed-`char` value: **802 082**
  triples.

Total **4 192 832 FFI calls** (2 096 416 C/Rust pairs), 127 ms in release.
Reported by `exhaustive_call_count_report`.

## Mutation testing (harness adequacy)

Passing tests only prove the harness is adequate if it can also *fail*.
`translation/mutation_sweep.sh` injects 22 bugs into a copy of
`translation/src/lib.rs` (never the original), builds each as a standalone
`.so`, and runs the suite against it via `RUST_DRIVER_SO`:

```
killed=21  known-equivalent=1  UNEXPECTED-SURVIVORS=0
```

The one survivor is `nul-skip: unconditional pos += 1`, which is provably
unobservable: the `if pos < bufferSize` guard is false exactly when
`pos >= bufferSize`, in which case the outer loop guard also fails and `pos` is
never read again — so incrementing it or not cannot change any output. The 4.19M
exhaustive pairs agree. The justification is recorded in
`is_expected_equivalent()` in the sweep script, and any *other* survivor makes
the script exit non-zero.
