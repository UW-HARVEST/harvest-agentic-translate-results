# ERRORS.md — error / rejection surface table (Phase A + Phase C)

Mechanically derived by grepping `c_src/src/*.c` for every `return` that reports
failure, every `fprintf(stderr, ...)`, every `break` that abandons a scan, every
`MAX_*` bound and every null check.  One row per distinct rejection.

`[x]` = a differential test constructs that exact condition, calls **both** the C
and the Rust `.so` and asserts the same result/sentinel.

Test locations: `tests/errors.rs` (most rows), `tests/uninitialized.rs` (E18a,
E24 — need a virgin process), `tests/runner_scenarios.rs` (E34, E35 and the
virgin-process/NULL-ops rows, run out of process through
`examples/ffi_runner.rs`), `tests/driver_e2e.rs` (E36–E42).

| # | function (C line) | trigger (exact invalid input / condition) | expected C result | [x] |
|---|-------------------|-------------------------------------------|-------------------|-----|
| E1 | `tokenizer_load_text` (319) | `text == NULL` | returns `-1`, **no** message | [x] |
| E2 | `tokenizer_load_text` (324) | `strlen(text) == MAX_BUFFER_SIZE` (8192) | `stderr: "Error: Input text too large\n"`, returns `-1`, buffer unchanged | [x] |
| E3 | `tokenizer_load_text` (324) | `strlen(text) > 8192` (e.g. 20000) | same as E2 | [x] |
| E4 | `tokenizer_load_text` (324) | `strlen(text) == 8191` (last accepted length) | returns `0`, no message | [x] |
| E5 | `peek_char` (64) | `current_position >= buffer_length` (empty buffer / after last token) | `'\0'` → `next_token` yields `TOKEN_EOF` with `value ""`, `length 0`, `column = current_column` | [x] |
| E6 | `advance_char` (71) | called at end of buffer | returns `'\0'` and does **not** bump `total_chars_processed` (observable through `tokenizer_get_stats` after repeated `next_token` at EOF) | [x] |
| E7 | `create_token` (98) | `length >= MAX_TOKEN_LENGTH` (only reachable from `scan_string`, length 256) | `token.length` clamped to `255`, value truncated to 255 bytes | [x] |
| E8 | `scan_word` (113) | identifier longer than `MAX_TOKEN_LENGTH-1` (256+ alnum/`_` chars) | token cut at 255 chars, the rest becomes the following token(s) | [x] |
| E9 | `scan_number` (138) | second `.` in a number (`1.2.3`) | scan stops before the second `.`; `.` then scanned as `TOKEN_PUNCTUATION` | [x] |
| E10 | `scan_number` (134) | number longer than 255 chars | token cut at 255 chars | [x] |
| E11 | `scan_string` (160) | string literal body longer than `MAX_TOKEN_LENGTH-2` | loop stops, closing quote **not** appended unless the very next char is the quote; `token.length` 255 (see E7) | [x] |
| E12 | `scan_string` (157) | unterminated string at end of buffer (`"abc`) | token contains the opening quote and body, no closing quote | [x] |
| E13 | `scan_string` (159) | newline inside string (`"abc\ndef"`) | scan stops before `\n`, no closing quote | [x] |
| E14 | `scan_string` (162-166) | trailing backslash at end of buffer (`"ab\`) | escape char consumed, nothing after it | [x] |
| E15 | `scan_comment` (191) | `//` comment that runs to end of buffer / `\n` | comment token ends at `\n` (exclusive) or EOF | [x] |
| E16 | `scan_comment` (193) | `//` comment longer than 255 chars | token cut at 255 chars | [x] |
| E17 | `scan_comment` (200) | unterminated `/*` comment (EOF before `*/`) | token is the rest of the buffer, `length ≤ 255` | [x] |
| E17b | `scan_comment` (200) | `/*` comment longer than 254 chars | loop stops at bound, no `*/` in the token | [x] |
| E17c | `scan_comment` (187/196) | lone `/` (next char is neither `/` nor `*`, e.g. `a / b`) | `TOKEN_COMMENT` with value `"/"` — `/` is **never** an operator because line 282 re-reads `peek_char()` | [x] |
| E18 | `tokenizer_get_stats` (339-341) | `lines`/`tokens`/`chars` any combination of `NULL` | each non-NULL pointer written, NULL ones skipped, no crash | [x] |
| E18a | `analyze_text` (70) | called before `analyzer_init` | `stderr: "Error: Analyzer not initialized\n"`, all-zero `analysis_result_t` | [x] |
| E19 | `analyze_text` (76) | `text == NULL` (`ops.load_text` returns -1) | `stderr: "Error: Failed to load text\n"`, all-zero result, no `stderr` message from the tokenizer | [x] |
| E20 | `analyze_text` (76) | text ≥ 8192 bytes | `stderr: "Error: Input text too large\n"` **then** `"Error: Failed to load text\n"`, all-zero result | [x] |
| E21 | `track_word` (59) | 101st *distinct* identifier | word silently dropped (`num_common_words` stays 100); `result.word_count` still counts it | [x] |
| E22 | `track_word` (60) | identifier longer than `MAX_TOKEN_LENGTH-1` | stored truncated to 255 bytes | [x] |
| E23 | `find_patterns` (196) | `pattern == NULL` | returns immediately, **no** output at all | [x] |
| E24 | `find_patterns` (196) | analyzer not initialized | returns immediately, no output | [x] |
| E25 | `find_patterns` (209) | empty pattern `""` | `strstr` matches every token → every token printed | [x] |
| E26 | `calculate_complexity_score` (190) | `score < 0` (comments outnumber keywords/operators) | clamped to `0` | [x] |
| E27 | `print_token_distribution` (143) | every `token_type_counts[i] == 0` | only the two headers printed, no rows | [x] |
| E28 | `print_token_distribution` (151) | `num_common_words == 0` | `int` loop bound `-1` → no sort, no word lines | [x] |
| E29 | `print_token_distribution` (169) | more than 10 distinct words | only the top 10 printed | [x] |
| E30 | `read_file` (104) | non-existent path | `stderr: "Error: Could not open file '<path>'\n"`, returns `NULL` | [x] |
| E31 | `read_file` (104) | `filename == NULL` | `fopen` fails with `EFAULT`; glibc `%s` prints `(null)` → `stderr: "Error: Could not open file '(null)'\n"`, `NULL` | [x] |
| E31b | `read_file` (104) | unreadable path (mode `0`) | same message as E30, `NULL` | [x] |
| E31c | `read_file` (113) | directory path | `fopen`+`ftell` succeed, `fread` fails → returns an **empty** string (not NULL) | [x] |
| E32 | `read_file` (113) | file larger than `MAX_BUFFER_SIZE` (8193 bytes) | `stderr: "Error: File too large\n"`, `NULL` | [x] |
| E32b | `read_file` (113) | file of exactly 8192 bytes (`size > MAX_BUFFER_SIZE` is false) | returns the content; the following `analyze_text` then rejects it via E20 | [x] |
| E33 | `read_file` (120) | `malloc` failure | `stderr: "Error: Memory allocation failed\n"`, `NULL` | n/a — cannot be provoked without an allocator fault injector (documented, not tested) |
| E34 | `interactive_tokenizer` (71) | 8192+ bytes of stdin (`ops.load_text != 0`) — unreachable in practice, `input` is capped at 4095 bytes; reachable with a foreign `ops` | `stdout: "Failed to load text\n"`, returns | [x] (via a stub `ops`) |
| E35 | `interactive_tokenizer` (95) | more than 100 tokens | 101 token lines then `"... (truncated, too many tokens)\n"` | [x] |
| E36 | `main` (153) | menu line without a decimal (`""`, `"x\n"`, `" \n"`, `"+\n"`) | `"Invalid input\n"` | [x] |
| E37 | `main` (231) | numeric choice outside 1..7 (`0`, `8`, `-1`, `99999999999999999999`) | `"Invalid choice\n"` | [x] |
| E38 | `main` (149) | EOF on the menu prompt (`fgets` → NULL) | loop breaks, exit status 0 | [x] |
| E39 | `main` (178/212) | EOF at the "Enter filename:"/"Enter pattern:" prompt | `break` out of the `switch` (not the loop) → menu printed once more, then EOF ends the loop | [x] |
| E40 | `main` (168) | more than 4095 bytes of text for choice 1 (`MAX_INPUT_SIZE - strlen(text) - 1 == 0`) | `strncat` appends nothing; only the first 4095 bytes are analysed | [x] |
| E41 | `interactive_tokenizer` (68) | more than 4095 bytes of stdin | same `strncat` saturation as E40 | [x] |
| E42 | `main` (149) | menu line longer than 255 bytes | `fgets` returns the first 255 bytes; the remainder is parsed as the next menu line | [x] |

## Undefined behaviour in the C source (deliberately not asserted)

These are reachable only by feeding the library something the C code cannot
survive; there is no "expected C result" to match, so they are documented rather
than tested.

| C site | UB |
|--------|----|
| `analyzer.c:85` `token_type_counts[token.type]++` | a foreign `tokenizer_ops_t.next_token` returning `type < 0` or `type >= 20` writes outside the 20-int array. Types `12..19` stay in bounds and **are** tested (`tests/errors.rs::enum_out_of_range_via_custom_ops`). |
| `main.c:89` `token_type_names[token.type]` | same, for the 12-entry name table |
| `analyzer.c:76`, `main.c:71` | `tokenizer_ops_t` with `NULL` members → call through a null function pointer. The C build dies from `SIGSEGV` with its buffered `stdout` discarded, and this **is** compared: `tests/runner_scenarios.rs::null_ops_dispatch_dies_identically` asserts the same signal and the same (empty) output for `analyze_text`, `find_patterns` and `interactive_tokenizer`. `src/ffi.rs::null_ops_member` faults on purpose instead of panicking, because a Rust panic message would be an observable difference. |
| `tokenizer.c:176` | `scan_string` writes `buffer[256]` (one past the 256-byte stack array) for a maximal string literal; the returned token is unaffected and *is* compared (E7/E11) |
| `main.c:126` | `read_file` on a non-seekable file (`ftell` → -1) calls `fread(content, 1, (size_t)-1, ...)` into a 0-byte allocation |
| `tokenizer.c:90` etc. | `isspace`/`isalpha`/`isalnum`/`isdigit` are passed a *signed* `char`; glibc's table is defined for `-128..255`, so bytes `0x80..0xFF` classify as "not space/alpha/digit/alnum" — matched by `cio::c_is*` and covered by C13/C14 in `CONFIGS.md` |

## Generic FFI boundaries (not a `RETURN_ERROR` in the C source, but tested anyway)

| # | condition | expected C result | [x] |
|---|-----------|-------------------|-----|
| G1 | `NULL` passed to every pointer parameter of the public API (`tokenizer_load_text`, `tokenizer_get_stats` ×7 masks, `analyze_text`, `find_patterns`, `read_file`) | see E1/E18/E19/E23/E31 — never a crash, always the documented sentinel | [x] |
| G2 | an interior NUL byte in the text/pattern/filename arguments | everything from the first NUL on is invisible (`strlen`/`strncpy`/`strstr` semantics) | [x] `tests/errors.rs::embedded_nul_truncates_like_strlen` |
| G3 | non-UTF-8 byte sequences in a filename | passed through byte for byte, and echoed byte for byte in the error message | [x] `tests/errors.rs::non_utf8_filenames` |
| G4 | token type values with no `token_type_t` variant (12..19) returned by a foreign `tokenizer_ops_t` | counted in `token_type_counts[]`, matched by no `switch` case, invisible in `print_token_distribution` | [x] `tests/errors.rs::enum_out_of_range_via_custom_ops` |
| G5 | `tokenizer_ops_t` whose members are all `NULL`, used only by entry points that do not dispatch (`print_token_distribution`, `calculate_complexity_score`, `print_menu`, `print_analysis_result`) | works normally | [x] `tests/runner_scenarios.rs::null_ops_installed_then_pure_functions` |
| G6 | `stdout` closed by the reader while the program is writing | the C build dies from `SIGPIPE` | [x] `tests/driver_e2e.rs::c50c_stdout_closed_early` (the Rust runtime ignores `SIGPIPE`, so `src/main.rs` restores the default disposition) |
| G7 | `stdout` is a terminal (line buffered) instead of a pipe/file (block buffered) | same bytes, and the same stdout/stderr interleaving | [x] `tests/driver_e2e.rs::c50b_tty_line_buffering` |
| G8 | `stdin` *and* `stdout` are the same terminal (fully interactive) | reading line-buffered `stdin` flushes line-buffered `stdout` first, so the unterminated `"Choice: "`/`"Enter filename: "` prompts appear **before** the next `stderr` message | [x] `tests/driver_e2e.rs::c50e_interactive_terminal` |
| G9 | `stdout` redirected to a file system whose `st_blksize` is not 4096 | glibc uses `min(st_blksize, BUFSIZ)` as the buffer size, which changes where the flushes fall | emulated by `cio::stdio_block_size`; only the 4096 (pipe/file) and 1024 (terminal) cases exist on the test machine and both are covered by C50/C50b/C50d/C50e |
