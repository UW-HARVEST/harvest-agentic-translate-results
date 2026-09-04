# Differential testing log

The C program in `c_src/` is the ground truth. Both executables are built and
then driven as subprocesses; for every input, stdout, stderr and the exit status
are compared byte for byte.

* C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
* Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
* Tests: `cd translation && cargo test` (76 tests, none ignored or skipped)

The test suite lives in `translation/tests/`:

| file | what it drives |
|---|---|
| `common/mod.rs` | spawns both binaries, builds the fixture files, compares the three observables |
| `menu.rs` | the `main` loop: `fgets` on the choice line and `sscanf(input, "%d", &choice)` |
| `analyze.rs` | menu entry `1` and, through it, every branch of `tokenizer_next_token` |
| `load_file.rs` | menu entry `2`: every branch of `read_file` and the two size limits |
| `report.rs` | menu entries `3`, `4`, `5`, `6` |
| `streams.rs` | `stdout` block buffering vs. unbuffered `stderr`, and death by `SIGPIPE` |
| `fuzz.rs` | deterministic randomised sessions, random bytes, random small-alphabet bytes |

---

## Mismatches found and fixed

### 1. `read_file`: streams that reject `SEEK_END` (procfs, sysfs)

*Input:* `2\n/proc/self/status\n3\n7\n` (likewise `/proc/version`,
`/proc/self/cmdline`, `/sys/kernel/mm/transparent_hugepage/enabled`)

*C behaviour:* `fseek(file, 0, SEEK_END)` returns 0 and `ftell(file)` returns 0
for these files (their `st_size` is 0), so `size == 0`, `fread` reads nothing,
`content` is the empty string and the program prints a full
`=== Analysis Results ===` block of zeros on stdout. Nothing on stderr.

*Rust behaviour before the fix:* `File::seek(SeekFrom::End(0))` issues a bare
`lseek(fd, 0, SEEK_END)`, which procfs rejects with `EINVAL`. The translation
mapped any seek error to `size = -1` and then took a "negative size means
`malloc` failed" branch, printing `Error: Could not …`/`Error: Memory
allocation failed` on stderr and skipping the results block on stdout.

*Cause:* the C code **ignores** the return value of `fseek`. A failed `fseek`
leaves the stream position untouched, and the following `ftell` reports that
position — it does not report an error. The Rust port had conflated "seeking to
the end failed" with "`ftell` failed".

*Fix (`src/main.rs`, `read_file`):* on a seek error, fall back to
`file.stream_position()` (the `lseek(fd, 0, SEEK_CUR)` that `ftell` performs)
and only use `-1` when that fails too.

### 2. `read_file`: non-seekable streams (FIFOs, pipes)

*Input:* `2\n<path-to-a-fifo>\n3\n7\n`

*C behaviour:* on a FIFO both `fseek` and `ftell` fail (`ESPIPE`), so
`size == -1`. `malloc(size + 1)` is therefore `malloc(0)`, which **succeeds** and
returns a zero-byte block; `fread(content, 1, (size_t)-1, file)` then asks the
kernel to read `SIZE_MAX` bytes into that empty block, `read` fails with
`EFAULT` and `fread` reports 0 bytes. `content[0]` is set to `'\0'`, so the
program analyses an empty string: a results block of zeros on stdout and nothing
on stderr.

*Rust behaviour before the fix:* `size = -1` was treated as an allocation
failure, printing `Error: Memory allocation failed` on stderr and no results
block.

*Cause:* the assumption that `malloc(size + 1)` fails for a negative `size` is
wrong — the expression is `malloc(0)`, which succeeds. The failure that actually
happens is inside `fread`, and it is indistinguishable from a short read.

*Fix (`src/main.rs`, `read_file`):* when `size < 0`, allocate nothing, read
nothing and return an empty string — no error message, and no
`Error: File too large` either, because `-1 > MAX_BUFFER_SIZE` is false.

Both mismatches lived in the same `if size < 0` block, but they are distinct:
the first is about *which* of `fseek`/`ftell` failed, the second about what C
does once `size` really is `-1`.

---

## C behaviour that was verified to already match

These all looked like candidates for a mismatch and were checked explicitly;
the translation was already faithful.

* **`sscanf("%d")` overflow.** glibc converts through `strtol`, which saturates
  at `LONG_MAX`/`LONG_MIN`; the result is then truncated to `int`. So
  `99999999999999999999` becomes `-1` → `Invalid choice`, while `4294967303`
  truncates to `7` and exits. Values such as `2147483648` and `4294967297` wrap
  the same way in both programs. (`menu.rs::choice_integer_truncation_and_overflow`)
* **`sscanf` returning `EOF` vs `0`.** The C code only tests `!= 1`, so a blank
  line and `abc` both print `Invalid input`.
* **`fgets` splits long lines.** A 300-byte choice line is consumed 255 bytes at
  a time, so its tail is parsed as the next choice; the same is true of the
  filename in case `2` and of the pattern in case `5`.
  (`menu.rs::choice_line_longer_than_the_buffer`,
  `load_file.rs::filename_longer_than_the_buffer`)
* **`break` inside a `switch`.** In cases `2` and `5` the `break` taken when
  `fgets` returns `NULL` leaves the `switch`, not the `while (1)`; the menu is
  printed once more before the loop finally ends at EOF.
  (`load_file.rs::eof_right_after_the_choice`,
  `report.rs::find_pattern_eof_after_the_choice`)
* **The unreachable comment test.** `if (c == '/' && (peek_char() == '/' ||
  peek_char() == '*'))` calls `peek_char()` *before* the `'/'` is consumed, so it
  still returns `'/'` and the condition is true for every `'/'`. Every slash is
  therefore scanned as a comment, including `/=`, `/%` and a lone `/`.
  (`analyze.rs::comments`)
* **Token-length clipping.** `scan_string` and the multi-line branch of
  `scan_comment` stop at `MAX_TOKEN_LENGTH - 2` accumulated bytes but can then
  append one or two more, reaching 256; `create_token` clamps `length` to 255 and
  `strncpy` copies exactly that many bytes. `scan_word` stops at 255.
  (`analyze.rs::string_length_boundaries`, `identifier_length_boundaries`,
  `comment_length_boundaries`)
* **Negative column numbers.** `create_token` computes
  `current_column - token.length` in `size_t` arithmetic and assigns the wrapped
  result to an `int`, which reproduces plain wrapping two's-complement
  subtraction; tokens that start a line print negative columns.
  (`report.rs::interactive_tokenizer_negative_columns`)
* **`strncat` clamping.** `strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1)`
  copies only up to the first NUL of `line` and stops once 4095 bytes have
  accumulated, so text after an embedded NUL is dropped and a long paste is
  silently truncated. (`analyze.rs::fills_the_input_buffer`,
  `high_bytes_and_nuls`)
* **The two size limits are different.** `read_file` rejects
  `size > MAX_BUFFER_SIZE`, so 8193 bytes gives `Error: File too large`, while
  8192 bytes passes that check and is then rejected by `tokenizer_load_text`
  (`length >= MAX_BUFFER_SIZE`) as `Error: Input text too large`, which
  `analyze_text` turns into `Error: Failed to load text` — followed by a results
  block of zeros. 8191 bytes loads fine. (`load_file.rs::size_limits`)
* **`fopen` on a directory succeeds** on Linux; `fread` then fails with `EISDIR`
  and the program analyses an empty string.
  (`load_file.rs::directory_instead_of_file`)
* **Cumulative statistics.** `tokenizer_reset` deliberately keeps
  `total_lines_processed`/`total_chars_processed`, and `analyzer_init` is only
  called once, so `Lines:`/`Characters:` and the token distribution keep growing
  across repeated analyses, and `find_patterns` inflates them further because it
  re-tokenises the buffer. (`analyze.rs::repeated_analysis_is_cumulative`,
  `report.rs::find_pattern_repeats_and_resets`)
* **`track_word` caps at 100 distinct words**, the bubble sort in
  `print_token_distribution` only swaps on a strict `<` (so ties keep their
  insertion order), and only the top 10 are printed.
  (`report.rs::distribution_word_table_boundaries`, `distribution_with_tied_counts`)
* **Interactive truncation fires after token 101**, because `count` is
  incremented before the `count > 100` test.
  (`report.rs::interactive_tokenizer_truncation_boundary`)
* **An empty pattern matches every token**, because `strstr(s, "")` returns `s`.
  (`report.rs::find_pattern_matches`)
* **The "C" locale is never changed**, so `isalpha`/`isdigit`/`isspace` are
  ASCII-only and bytes ≥ 0x80 become `ERROR` tokens.
  (`analyze.rs::high_bytes_and_nuls`)
* **Stream buffering.** C's `stdout` is block buffered when it is not a
  terminal while `stderr` is unbuffered, so error messages can overtake earlier
  `printf` output when both are merged onto one descriptor. The translation
  reproduces this (`src/cio.rs`). (`streams.rs`)
* **`SIGPIPE`.** The Rust runtime ignores `SIGPIPE`; `src/cio.rs::restore_sigpipe`
  puts the default disposition back, so a closed stdout kills both programs with
  signal 13 instead of letting the Rust one exit 0.
  (`streams.rs::dies_from_sigpipe_when_stdout_is_closed`)
* **Exit status is always 0** on every normal path (`return 0` from case `7` and
  the fall-through at EOF).

---

## Coverage notes

Paths that exist in the C source but cannot be reached from `main`, and so have
no test:

* `analyze_text`'s `Error: Analyzer not initialized` and `find_patterns`'s
  `!initialized` early return — `analyzer_init` is called unconditionally before
  the menu loop.
* `tokenizer_load_text(NULL)` returning `-1` — the only callers pass real buffers.
* `interactive_tokenizer`'s `Failed to load text` — its accumulator is 4096 bytes,
  always below `MAX_BUFFER_SIZE`.
* `read_file`'s `Error: Memory allocation failed` — the only way to reach it
  would be a genuine `malloc` failure for at most 8193 bytes.
* `tokenizer_peek_token` / `TOKEN_WORD` / `TOKEN_WHITESPACE` — never used by any
  caller, so no token of those types is ever produced.

Beyond the enumerated cases, the suite runs 540 generated sessions and random
byte strings (`fuzz.rs`); the 104-case corpus in `_ref/cases` also passes. No
further differences were observed.
