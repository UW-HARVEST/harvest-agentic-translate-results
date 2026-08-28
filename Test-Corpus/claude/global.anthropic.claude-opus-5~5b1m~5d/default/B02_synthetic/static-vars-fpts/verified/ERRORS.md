# Differential verification of the C → Rust translation

The C program in `c_src/` is the ground truth. This document records how the
Rust port was verified against it, every mismatch that was found, and the C
quirks that had to be preserved deliberately.

## How to reproduce

```sh
# 1. build the reference
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .

# 2. build the translation
cd translation && cargo build --release

# 3. run the differential suite (spawns both executables as subprocesses)
cd translation && cargo test --release
```

Run commands used by the tests:

| program | command |
|---|---|
| C       | `c_src/build/driver` (stdin redirected from a file) |
| Rust    | `translation/target/release/driver` (stdin redirected from a file) |

`tests/harness/mod.rs` compares **stdout, stderr and the exit status** (both the
exit code and the terminating signal) for every case. If `c_src/build/driver`
does not exist the harness compiles the reference with `cc` into
`translation/target/c_reference/driver`; `c_src/` itself is only ever read.

`tests/differential.rs` contains 16 test groups covering ~900 distinct inputs.
No test is `#[ignore]`d, skipped or otherwise disabled.

---

## Mismatches found and fixed

### 1. `read_file`: `fseek(f, 0, SEEK_END)` / `ftell` does not map onto `Seek::seek`

*Symptom* — input `2\n/proc/self/status\n7\n` (menu option "Load text from
file"):

```
C     stdout: ... Enter filename: \n=== Analysis Results ===\nWords/Identifiers: 0\n ... exit 0
C     stderr: (empty)

rust  stdout: ... Enter filename: (no analysis at all)          exit 0
rust  stderr: Error: Memory allocation failed\n
```

*Cause* — the original translation computed the file size with

```rust
let size = match file.seek(SeekFrom::End(0)) { Ok(p) => p as i64, Err(_) => -1 };
```

glibc does **not** issue an `lseek(fd, 0, SEEK_END)` for a regular file.
`_IO_new_file_seekoff` calls `fstat` first and, when `S_ISREG(st.st_mode)`
holds, uses `st_size` and converts the request into a `SEEK_SET`. That
difference is observable on procfs: a file such as `/proc/self/status` *is* a
regular file with `st_size == 0`, but the kernel's `seq_lseek` rejects
`SEEK_END` with `EINVAL`. Measured directly:

```
/proc/self/status   stat: isreg=1 size=0     lseek(END) = -1 (EINVAL)
data/small.txt      stat: isreg=1 size=17    lseek(END) = 17
```

So the C program saw `size == 0`, read nothing, and analysed an empty string,
while Rust's `seek` failed and took a branch the C never takes.

*Fix* — `translation/src/main.rs`, new helper `c_seek_end_and_tell`: use
`fstat`'s `st_size` for regular files and fall back to `lseek(SEEK_END)` only
for anything else. If that fails too, report the unchanged stream position
(what `ftell` returns after a failed `fseek`) instead of `-1`.

### 2. `read_file`: `malloc(size + 1)` never fails for a negative size

*Symptom* — same code path as above; the Rust port printed
`Error: Memory allocation failed` whenever the computed size was negative.

*Cause* — `size` is a `long`. When `ftell` returns `-1`, the C code calls
`malloc(-1 + 1)` = `malloc(0)`, which **succeeds** and returns a non-null
pointer, so the `if (!content)` branch is not taken. It then calls
`fread(content, 1, (size_t)-1, file)`, i.e. "read until EOF".

*Fix* — treat `size < 0` as "read to end of file" rather than as an allocation
failure. (`Error: Memory allocation failed` is dead code in the C program:
`malloc` of at most 8193 bytes does not fail. It is kept in the Rust port only
for structural fidelity.)

---

## C behaviour deliberately preserved (verified, not "fixed")

These all looked like bugs and were confirmed against the C binary before being
replicated. Each has at least one dedicated test case.

* **A lone `/` is scanned as a comment.** `tokenizer_next_token` tests
  `if (c == '/' && (peek_char() == '/' || peek_char() == '*'))`, but `peek_char()`
  has not consumed anything yet, so it returns the very same `/`. The condition
  is therefore true for *every* `/`, `scan_comment` runs, and `/`, `a / b`,
  `a/` all yield a `COMMENT` token — the `TOKEN_OPERATOR` branch is unreachable
  for `/`. `gcov` confirms `peek_char() == '*'` is never even evaluated.
  Tests: `comment_slash_alone`, `comment_slash_between`, `comment_slash_at_eof`.
* **Negative token columns.** `create_token` computes
  `token.column = current_column - token.length` where `current_column` is `int`
  and `token.length` is `size_t`: the subtraction happens in `size_t`, wraps, and
  is then truncated back to `int`. A `NEWLINE` token always reports column `0`,
  and a block comment spanning newlines reports a negative column
  (`/*\nabc*/` → `L2:C-2`). Reproduced with
  `(current_column as u64).wrapping_sub(length as u64) as u32 as i32`.
  Tests: `tokenizer_column_arithmetic`, `pattern_negative_column`.
* **`analyze_text` overwrites `line_count`.** The `TOKEN_NEWLINE` case increments
  `result.line_count`, but `result.line_count = lines` afterwards discards it and
  reports the tokenizer's *cumulative* `total_lines_processed`.
* **The tokenizer's totals are never reset.** `tokenizer_reset` explicitly keeps
  `total_lines_processed` / `total_tokens_processed` / `total_chars_processed`, so
  `Lines:` and `Characters:` grow across menu commands, and option 5
  ("Find pattern") inflates them because it re-tokenizes the buffer.
  Tests: `session_char_count_accumulates`, `pattern_then_analyze`.
* **`print_token_distribution` mutates the word table.** The bubble sort reorders
  `common_words` / `common_word_counts` in place, so calling option 3 twice, or
  analysing more text afterwards, produces order-dependent output.
  Tests: `dist_sorted_twice`, `dist_analyze_after_sort`.
* **The word table saturates at 100 entries** and never grows; words past that
  are counted in `token_type_counts` but not tracked.
  Tests: `dist_99_words`, `dist_100_words`, `dist_101_words`.
* **`analyzer_init` clears the counts but not the strings.** Faithfully mirrored,
  though `analyzer_init` is only called once.
* **Two-byte truncation limits differ per scanner.** `scan_word` / `scan_number` /
  single-line `scan_comment` stop at `MAX_TOKEN_LENGTH - 1` (255) while
  `scan_string` and block comments stop at `MAX_TOKEN_LENGTH - 2` (254), and
  `create_token` clamps `token.length` to 255 and NUL-terminates there.
  A 300-character identifier therefore becomes two tokens.
  Tests: `tokenizer_token_length_limits` (13 lengths × 10 shapes).
* **`fgets` vs `scanf`.** The menu reads with `fgets(input, 256, stdin)` and then
  `sscanf(input, "%d", &choice)`, so a line longer than 255 bytes is split and its
  tail is parsed as the *next* menu choice; `%d` never reads across lines.
  Tests: `fgets_boundary_*`, `fgets_split_digits`, `long_garbage_line`.
* **`%d` overflow follows `strtol` saturation followed by a `long`→`int` cast.**
  `4294967303` becomes `7` (and exits), while `99999999999999999999` saturates to
  `LONG_MAX` and becomes `-1`. Tests: `menu_choice_parsing`.
* **`strncat` truncation at `MAX_INPUT_SIZE`.** `char text[4096]` is filled with
  `strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1)`, so the collected text
  is cut at 4095 bytes, possibly mid-token, and any NUL byte typed on a line ends
  that line's contribution. Tests: `analyze_input_buffer_limits`,
  `nul_inside_line`.
* **`break` inside a `switch` is not `continue`.** For options 2 and 5, EOF at the
  sub-prompt leaves the `switch`, so the menu is printed *once more* before the
  outer loop sees EOF and stops. Tests: `file_eof_at_prompt`,
  `pattern_eof_at_prompt`.
* **Two error messages for one 8192-byte file.** `read_file` accepts a file of
  exactly `MAX_BUFFER_SIZE` bytes (`size > MAX_BUFFER_SIZE` is false), but
  `tokenizer_load_text` then rejects it (`length >= MAX_BUFFER_SIZE`), so stderr
  gets both `Error: Input text too large` and `Error: Failed to load text` and a
  zeroed result is printed. 8193 bytes instead yields only
  `Error: File too large` and no result at all. Tests: `file_8191`, `file_8192`,
  `file_over_8192`.
* **`strstr` with an empty needle matches everything**, so an empty pattern on
  option 5 lists every token. Test: `pattern_empty`.
* **High-bit bytes.** `isalpha`/`isspace`/`isdigit` receive a *signed* `char`;
  in the C locale bytes `0x80`–`0xFF` are unclassified, so they become
  `TOKEN_ERROR`. Tests: `every_byte_value` (one case per byte value),
  `high_byte_*`.
* **`interactive_tokenizer` prints 101 tokens before truncating**, because
  `count` is incremented before `if (count > 100)`. Tests:
  `interactive_100_tokens` … `interactive_200_tokens`.

---

## Paths that cannot be reached from stdin

`gcov` on an instrumented copy of the C sources (compiled outside `c_src/`) over
the whole input corpus reports:

```
src/main.c        branches executed 100.00% of 42
src/analyzer.c    branches executed 100.00% of 42
src/tokenizer.c   branches executed  97.37% of 152
```

Everything still uncovered is provably dead:

* `interactive_tokenizer`: `printf("Failed to load text\n")` — the collected input
  is at most 4095 bytes, always below `MAX_BUFFER_SIZE`.
* `read_file`: `Error: Memory allocation failed` — `malloc` of ≤ 8193 bytes.
* `analyze_text`: `Error: Analyzer not initialized` — `main` calls
  `analyzer_init` before the loop.
* `find_patterns`: the `!initialized || !pattern` early `return` — same reason,
  and `pattern` is a stack buffer.
* `tokenizer_load_text`: the `!text` early `return` — never called with NULL.
* `advance_char`: `return '\0'` — only called after `peek_char() != '\0'`.
* `tokenizer_peek_token` and the `lookahead_valid` branch in
  `tokenizer_next_token` — `peek_token` is stored in the ops table but never
  invoked, so `lookahead_valid` is always 0.
* `peek_char() == '*'` in the comment test — short-circuited away by the quirk
  described above.

These are still translated (not deleted) so that the Rust source keeps the same
shape as the C.

---

## Verification effort

* 16 test groups / ~900 inputs in `tests/differential.rs`, all asserting stdout +
  stderr + exit status.
* One case per byte value `0x01`–`0xFF` through the tokenizer.
* 350 deterministic pseudo-random sessions (`randomised_sessions`,
  `randomised_raw_bytes`) built from a seeded xorshift generator.
* An additional 2700 randomised sessions were run out-of-tree during
  development (structured menu sessions plus unstructured random bytes); after
  the two fixes above, zero divergences remained.
