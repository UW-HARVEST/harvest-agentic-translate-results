# Differential verification log

C reference: `c_src/build/driver` (cmake 3.28.6 / gcc, built from unmodified
`c_src/`).
Rust under test: `translation/target/{debug,release}/driver`.
Comparison: both run as subprocesses on identical stdin; stdout, stderr and
exit status compared byte for byte (`translation/tests/common/mod.rs`).

## Outcome

**No mismatch was found.** Every enumerated input class, plus 3,100 randomized
sessions, produced identical stdout, stderr and exit status. This file
therefore records the audit rather than a list of repairs: for each C behaviour
that *could* plausibly have been mistranslated, it names the C construct, what
it does, and how the Rust reproduces it.

Nothing below required a change to `translation/src/`. The pre-existing
translation already handled each case, and the `NOTE (faithful to the C)`
comments in the Rust source line up with what the differential runs confirmed.

## Quirks in the C that the tests specifically pin down

These are the places where a "reasonable" Rust translation would have diverged.
Each one is now covered by a named test.

### 1. Every `/` is scanned as a comment; the operator branch is dead for `/`

`tokenizer.c` `tokenizer_next_token()`:

```c
if (c == '/' && (peek_char() == '/' || peek_char() == '*')) {
    return scan_comment();
}
```

`peek_char()` has not advanced, so it still returns `c`. The condition reduces
to `c == '/'`. Consequently `a / b` yields a `COMMENT` token `"/"`, `x /= y`
yields `COMMENT "/"` followed by `OPERATOR "="`, and `/` never reaches
`scan_operator()`. A translation that "fixed" this to look at the *next*
character would classify division as an operator and diverge on token type,
distribution counts and complexity score.

Rust: `src/tokenizer.rs::next_token` keeps the redundant `self.peek_char()`
test. Tests: `tokenizer_paths::slash_always_starts_a_comment_scan`.

### 2. `token.column` is computed in `size_t` and truncated back to `int`

`tokenizer.c` `create_token()`:

```c
token.column = current_column - token.length;
```

`current_column` is `int`, `token.length` is `size_t`, so the subtraction is
unsigned 64-bit and the result is truncated into an `int`. For a token at the
start of a line this wraps negative — e.g. a 10-character identifier at column
1 reports `C-9`. Printing `%d` shows the negative value.

Rust: `current_column.wrapping_sub(length as i32)`, which agrees mod 2^32.
Tests: `tokenizer_paths::token_columns_wrap_negative_for_tokens_at_the_line_start`.

### 3. `analyze_text` throws away the newline tally it just computed

`analyzer.c`:

```c
case TOKEN_NEWLINE: result.line_count++; break;
...
result.line_count = lines;   /* cumulative, from get_stats */
```

The per-run newline count is overwritten by the tokenizer's *cumulative*
`total_lines_processed`, which `tokenizer_reset()` deliberately does not clear.
So "Lines:" grows across successive analyses and includes lines seen by menu
choice 6 as well. Same for "Characters:".

Rust: `src/analyzer.rs::analyze_text` performs the same overwrite.
Tests: `menu_dispatch::analyze_accumulates_tokenizer_statistics_across_calls`,
`file_paths::loading_a_file_twice_accumulates_statistics`,
`boundaries::interleaved_analyses_keep_the_static_state_growing`.

### 4. `break` inside a `switch` does not leave the `while` loop

`main.c` cases 2 and 5:

```c
if (!fgets(input, sizeof(input), stdin)) {
    break;          /* leaves the switch, not the while(1) */
}
```

On EOF at the "Enter filename: " / "Enter pattern to search: " prompt the
program falls out of the switch and prints the whole menu again; only the
*next* `fgets` at the top of the loop ends it. The visible effect is one extra
menu block on stdout.

Rust: `if let Some(line) = stdin.fgets(256)` with no early return.
Tests: `file_paths::eof_at_the_filename_prompt_reprints_the_menu`,
`analyzer_paths::pattern_search_eof_instead_of_a_pattern`.

### 5. A 255-byte input line silently terminates the input block

`fgets(line, 256, stdin)` reads at most 255 bytes. For a line of exactly 255
characters the newline is left in the stream, so the *next* `fgets` returns
`"\n"`, which `line[0] == '\n'` treats as the empty terminator line. The text
that follows is consumed as menu input instead of as text.

Rust: `CStdin::fgets(256)` reproduces the 255-byte cap and newline retention.
Tests: `boundaries::a_255_byte_line_ends_the_input_block_early`,
`boundaries::text_lines_around_the_fgets_buffer_size`.

### 6. Two size limits, three outcomes, for menu choice 2

- `size > MAX_BUFFER_SIZE` (i.e. > 8192) → `read_file` prints
  `Error: File too large` to stderr and returns NULL; no result block prints.
- exactly 8192 → `read_file` succeeds, then `tokenizer_load_text` rejects
  `length >= MAX_BUFFER_SIZE` with `Error: Input text too large`, and
  `analyze_text` adds `Error: Failed to load text`. Both on stderr, and a
  **zeroed** result block still prints on stdout.
- ≤ 8191 → analyzed normally.

Tests: `file_paths::file_size_boundaries_around_max_buffer_size`.

### 7. `scan_string` / `scan_comment` can run one byte past their loop guard

Both loops guard on `length < MAX_TOKEN_LENGTH - 2` (254) but their two-byte
branches (`\\` + escaped char; `*` + `/`) append two bytes, so `length` reaches
256 and the C then writes `buffer[256]` on a `char buffer[256]` — a one-byte
stack overrun. `create_token` afterwards clamps `length` to 255, so the token
text is unaffected.

Rust: `buffer[..length]` with `length` up to 256 on a `[u8; 256]` is in range,
and `create_token` applies the same clamp, so no panic and identical output.
Tests: `boundaries::escape_sequences_can_push_the_string_buffer_one_past_its_loop_guard`,
`boundaries::block_comment_star_pair_at_the_buffer_edge`.

### 8. `sscanf("%d")` saturates then truncates

glibc converts via `strtol` (clamping at `LONG_MAX`/`LONG_MIN` on overflow) and
stores the low 32 bits into `int choice`. So `4294967297` selects menu item 1,
`4294967303` selects 7 (and exits), and `99999999999999999999999` becomes -1.

Rust: `cio::sscanf_int` saturates to `i64::MIN`/`i64::MAX` then casts `as i32`.
Tests: `menu_dispatch::choice_integer_conversion_truncates_the_way_the_c_does`.

### 9. Everything is a C string, so NUL bytes truncate

`strncat`, `sscanf`, `strcspn`, `fopen`, `strstr` and the `content[read_size] =
'\0'` in `read_file` all treat their buffers as NUL-terminated. A NUL in stdin,
in a filename, in a search pattern, or in a loaded file discards the remainder.

Rust: `cio::cstr` is applied at each of those points.
Tests: `tokenizer_paths::nul_bytes_truncate_the_c_strings_they_appear_in`,
`menu_dispatch::nul_byte_truncates_the_choice_buffer`,
`file_paths::embedded_nul_truncates_the_file_contents`,
`file_paths::filename_containing_a_nul_is_truncated`,
`analyzer_paths::pattern_containing_a_nul_is_truncated`.

### 10. Bubble sort in `print_token_distribution` mutates the stored counts

The sort reorders `common_words`/`common_word_counts` in place, so the order
observed by a later call to choice 3 depends on the earlier call. It swaps only
on strict `<`, so equal counts keep their relative order per pass.

Rust: same in-place `swap` on the thread-local state.
Tests: `analyzer_paths::distribution_bubble_sort_ordering`,
`analyzer_paths::word_tracking_accumulates_across_separate_analyses`.

### 11. `strncat` bound saturates the 4096-byte text buffer at 4095

`strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1)` reaches `n == 0` once
`strlen(text) == 4095`; further lines are read and discarded but the loop keeps
running until an empty line or EOF.

Rust: `strncat_bounded` computes the same bound (and, importantly, the length
can never reach 4096, so the subtraction cannot underflow — confirmed by the
debug-profile run, which has overflow checks enabled).
Tests: `boundaries::accumulated_text_around_max_input_size`,
`boundaries::a_saturated_input_buffer_still_reads_and_discards_the_rest`.

### 12. `fopen` on a directory succeeds

`open(dir, O_RDONLY)` succeeds on Linux, so there is no "Could not open file"
message; the subsequent `fread` fails with `EISDIR` and the contents come out
empty, which is then analyzed as an empty string.

Rust: `File::open` behaves the same and the read error is swallowed the way
`fread` returning 0 is. Tests: `file_paths::a_directory_opens_but_cannot_be_read`.

### 13. Unseekable inputs (checked, not asserted)

For a FIFO or `/dev/stdin`, `fseek` fails and `ftell` returns -1; the C then
calls `malloc(0)` and `fread(content, 1, (size_t)-1, file)`, which is undefined
behaviour. Both binaries were run against a named FIFO and `/dev/stdin` and
produced identical stdout, stderr and status, but because the C path is UB this
is deliberately **not** encoded as a test assertion — its outcome is not a
specification. `/proc/version` (stat size 0, but readable) *is* asserted, since
that path is well defined: `ftell` gives 0 and nothing is read.

## Paths that no input can reach

Enumerated for completeness; they are unreachable in the C, so there is nothing
to compare.

- `tokenizer_peek_token()` and the `lookahead_valid` branch of
  `tokenizer_next_token()` — `main.c` only ever calls `next_token`, `reset`,
  `load_text` and `get_stats`.
- `Error: Analyzer not initialized` — `main` calls `analyzer_init` before the
  loop and `initialized` is never cleared.
- `find_patterns`' `!pattern` guard — the caller always passes a valid buffer.
- `interactive_tokenizer`'s `Failed to load text` — its buffer is capped at
  4095 bytes, always under the 8192-byte limit `load_text` checks.
- `Error: Memory allocation failed` — `malloc` of at most 8193 bytes.
- `TOKEN_WORD` (the `WORD` distribution row) and `TOKEN_WHITESPACE` — the
  tokenizer only ever emits `TOKEN_IDENTIFIER`, never `TOKEN_WORD`, and never
  emits a whitespace token.

## Coverage

97 tests across five files, all passing in both the debug and release profiles,
none `#[ignore]`d:

| file | tests | area |
| --- | --- | --- |
| `tests/menu_dispatch.rs` | 25 | `main`'s loop, `sscanf` outcomes, cases 1/6/7, EOF |
| `tests/tokenizer_paths.rs` | 18 | every `next_token` branch, all 31 keywords, all 255 byte values |
| `tests/analyzer_paths.rs` | 21 | distribution, bubble sort, complexity bands, pattern search |
| `tests/file_paths.rs` | 17 | `read_file` branches and the 8191/8192/8193 boundary |
| `tests/boundaries.rs` | 16 | `MAX_TOKEN_LENGTH`, `MAX_INPUT_SIZE`, `fgets` splits, soak |

`boundaries::deterministic_pseudorandom_soak` contributes 120 reproducible
randomized sessions. A further 3,100 randomized sessions (600 unstructured,
2,500 structured with file-loading) were run ad hoc during Phase C; all matched.
