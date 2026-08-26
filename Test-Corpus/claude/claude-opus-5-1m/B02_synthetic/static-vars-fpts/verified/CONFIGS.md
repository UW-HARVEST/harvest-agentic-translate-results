# CONFIGS.md — configuration surface table (Phase A + Phase B)

Axes derived mechanically from the C source (not from docs):

**Runtime options / modes the public API can set**

| axis | states | where the C branches on it |
|------|--------|-----------------------------|
| stdio buffering | `stdout` line buffered on a terminal, block buffered (`st_blksize`, 4096 for a pipe/file, clamped to `BUFSIZ`) otherwise; `stderr` unbuffered; reading a line-buffered `stdin` flushes the line-buffered `stdout` | glibc `filedoalloc.c`, `fileops.c` — reached through every `printf`/`fgets` in `main.c` |
| loaded buffer | `tokenizer_load_text` sets `input_buffer`/`buffer_length` and resets position | `tokenizer.c:318-336` |
| scan position | `current_position`, `current_line`, `current_column` | `peek_char`/`advance_char` |
| lookahead slot | `lookahead_valid` 0/1 set by `tokenizer_peek_token` | `tokenizer.c:247-250, 302-308` |
| cumulative stats | `total_{lines,tokens,chars}_processed`, **never** reset by `tokenizer_reset` | `tokenizer.c:310-316, 338-342` |
| analyzer `initialized` | 0 before `analyzer_init`, 1 after | `analyzer.c:70, 196` |
| analyzer `tokenizer_ops` | the 5 function pointers installed by `analyzer_init`; every analyzer entry point dispatches through them | `analyzer.c:76, 83, 125, 203, 208` |
| analyzer accumulators | `token_type_counts[20]`, `common_words[100]`, `common_word_counts[100]`, `num_common_words` — accumulate across `analyze_text` calls, only reset by `analyzer_init` | `analyzer.c:39-47, 85, 49-65` |
| menu mode (driver) | choices 1,2,3,4,5,6,7, non-numeric, out-of-range | `main.c:158-234` |

**Input shapes the C special-cases**: empty / 1 char / many; `\n` vs other
whitespace (`' ' \t \v \f \r`); alpha vs `_` vs digit first char; the 31
keywords vs other identifiers; `.` inside vs outside a number; `"` vs `'`
strings, escapes, unterminated; `//` vs `/*` vs lone `/`; each of the 16
operator chars and the 11 two-char operators; each of the 9 punctuation chars;
unknown bytes (`# @ $ \ backtick`) and bytes `0x80..0xFF`; token-length
boundaries 254/255/256; buffer-length boundaries 0/1/8191/8192; embedded NUL;
CRLF; >10 and >100 distinct words; >100 tokens.

`[x]` = both `.so`s were driven through this configuration with **many
randomized inputs** (fixed seed) or the exact boundary values, and every byte of
the observable output matched.

## Low-level tokenizer entry points

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `tokenizer_load_text` + `tokenizer_next_token` loop | empty text `""` | [x] |
| C2 | `tokenizer_load_text` + `next_token` loop | single character of each class: alpha, `_`, digit, `"`, `'`, `/`, each operator char, each punctuation char, unknown byte, `\n`, `' '` | [x] |
| C3 | `tokenizer_load_text` + `next_token` loop | all 31 keywords, each alone and mixed with identifiers that are prefixes/suffixes of keywords (`if`, `iff`, `_if`, `IF`) | [x] |
| C4 | `tokenizer_load_text` + `next_token` loop | identifiers with digits/underscores; identifier at buffer end without trailing separator | [x] |
| C5 | `tokenizer_load_text` + `next_token` loop | numbers: `0`, `123`, `1.5`, `.5` (punct+number), `1.`, `1.2.3`, `007`, digits+alpha (`12ab`) | [x] |
| C6 | `tokenizer_load_text` + `next_token` loop | strings: `""`, `"a"`, `'x'`, mixed quotes `"'"`, escapes `"a\"b"`, `"a\\"`, unterminated (EOF), unterminated (newline), backslash at EOF | [x] |
| C7 | `tokenizer_load_text` + `next_token` loop | comments: `//`, `// text`, `//` at EOF, `/* */`, `/**/`, `/* *`, `/* * /`, `/*` unterminated, lone `/`, `/=`, `//*` | [x] |
| C8 | `tokenizer_load_text` + `next_token` loop | all 11 two-char operators, each also as a non-pair (`=+`, `<>`, `&|`, `-<`), and 3-char runs (`==='`, `<<=`) | [x] |
| C9 | `tokenizer_load_text` + `next_token` loop | whitespace only: `' '`, `\t`, `\v`, `\f`, `\r`, and mixtures; leading/trailing whitespace around tokens (column tracking) | [x] |
| C10 | `tokenizer_load_text` + `next_token` loop | multi-line input: `\n` at start/end, consecutive `\n`, CRLF (`\r\n`), 200 lines (line/column tracking) | [x] |
| C11 | `tokenizer_load_text` + `next_token` loop | token-length boundaries: identifier/number/`//`-comment of 254, 255, 256, 300 chars; string body of 253, 254, 255, 300 chars; `/*` comment of 253, 254, 300 chars | [x] |
| C11b | `tokenizer_load_text` + `next_token` loop | multi-line `/* */` comments that reset `current_column` in the middle of a token, so that `create_token`'s `current_column - token.length` goes **negative** (81 shapes × 3 newline placements) | [x] |
| C12 | `tokenizer_load_text` + `next_token` loop | buffer-length boundaries: 1, 8190, 8191 bytes of text | [x] |
| C13 | `tokenizer_load_text` + `next_token` loop | text containing bytes `0x80..0xFF` (signed-`char` `isalpha`/`isspace` arguments) | [x] |
| C14 | `tokenizer_load_text` + `next_token` loop | **randomized** byte soup: 200 random inputs over the full alphabet of interesting characters (incl. `0x01..0xFF`), 0-600 bytes each, fixed seed | [x] |
| C15 | `tokenizer_load_text` + `next_token` loop | **randomized** C-like source text: 200 random inputs built from keywords/identifiers/numbers/strings/comments/operators/punctuation/newlines, fixed seed | [x] |
| C16 | `tokenizer_peek_token` / `tokenizer_next_token` interleavings | randomized scripts of 400 operations over `peek`/`next` (peek is idempotent, next consumes the lookahead) on randomized text | [x] |
| C17 | `tokenizer_reset` | reset mid-stream, then re-tokenize to EOF; reset at EOF; reset twice; reset before any load | [x] |
| C18 | `tokenizer_get_stats` | after every step of a randomized script (`load`/`next`/`peek`/`reset`), all three out-params — stats accumulate across `reset` and across `load_text` | [x] |
| C19 | `tokenizer_load_text` called repeatedly | 2nd/3rd load with shorter text (stale buffer bytes must not be visible), with longer text, with `""` | [x] |
| C20 | `tokenizer_next_token` past EOF | call 5 extra times after `TOKEN_EOF` (`total_tokens_processed` keeps rising, chars do not) | [x] |
| C21 | `get_tokenizer_ops` | drive the whole tokenizer **only** through the returned function pointers (`load_text`/`next_token`/`peek_token`/`reset`/`get_stats`) on randomized text | [x] |

## Analyzer entry points

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C22 | `analyzer_init(get_tokenizer_ops())` + `analyze_text` | own ops; text = each single-token class (word/keyword/number/string/comment/operator/punct/error/newline) | [x] |
| C23 | `analyzer_init` + `analyze_text` | own ops; **randomized** C-like text, 200 samples (all 8 result fields compared) | [x] |
| C24 | `analyzer_init` + `analyze_text` ×N | repeated analysis without re-init: counts/`common_words`/stats accumulate; `line_count`/`char_count` come from the cumulative tokenizer stats | [x] |
| C25 | `analyzer_init` twice | re-init resets `token_type_counts`/`common_word_counts`/`num_common_words` but **not** `common_words` contents nor the tokenizer stats | [x] |
| C26 | `analyzer_init(foreign ops)` + `analyze_text` | the ops struct of the *other* library (C ops installed into the Rust analyzer and vice versa) — verifies real dispatch through the function pointers | [x] |
| C27 | `analyzer_init(stub ops)` + `analyze_text` | test-supplied `tokenizer_ops_t` returning a fixed token script (incl. types 12..19, empty values, huge `length`) | [x] |
| C28 | `analyze_text` + `calculate_complexity_score` | scores driven by keyword/operator/punctuation/comment mixes: 0 punctuation, 9 punctuation (`/10` → 0), 10, 25; comment-heavy (negative → clamp), keyword-heavy | [x] |
| C29 | `analyze_text` + `print_token_distribution` | 0 words, 1 word, 9, 10, 11, 100, 101 distinct words; repeated words with tied counts (bubble-sort order); called twice in a row | [x] |
| C30 | `analyze_text` + `find_patterns` | patterns: `""`, 1 char, whole token, longer than every token, pattern with `\` and quote chars, pattern matching only a keyword/comment/string; 200 randomized (text, pattern) pairs | [x] |
| C31 | `find_patterns` twice / after `tokenizer_reset` | second call re-scans from the start and keeps bumping the cumulative stats | [x] |
| C32 | `analyzer_init` + `find_patterns` with no `analyze_text` | pattern search over the buffer left by a bare `tokenizer_load_text` | [x] |
| C33 | `analyze_text` with text at the 8191/8192 boundary | 8191 (accepted) and 8192 (rejected) byte texts | [x] |

## `main.c` entry points

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C34 | `print_menu` | called once / twice in a row (stdout buffering) | [x] |
| C35 | `print_analysis_result` | all-zero result; result with `SIZE_MAX` fields; 200 randomized results (`%zu` formatting) | [x] |
| C36 | `print_analysis_result` | the result value returned by `analyze_text` on randomized text (pipeline: analyze → print) | [x] |
| C37 | `read_file` | regular file: empty, 1 byte, 8191, 8192 bytes, binary bytes incl. embedded NUL, no trailing newline | [x] |
| C38 | `read_file` + `analyze_text` | full pipeline on a temp file of randomized C-like text | [x] |
| C39 | `interactive_tokenizer(get_tokenizer_ops())` | stdin: empty, immediate blank line, 1 line, 3 lines, line without trailing `\n`, >100 tokens, >4095 bytes, 8192+ bytes, lines longer than 255 bytes | [x] |
| C40 | `main` | full REPL, driven by the driver-binary comparison (see below) | [x] |

## Driver (`main`) end-to-end — `tests/driver_e2e.rs`

The C `driver` executable and the Rust `driver` binary are run with identical
stdin and their stdout/stderr/exit status compared byte for byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C41 | `main` | choice 7 immediately; EOF immediately; empty line; non-numeric; `0`/`8`/`-1`/huge number | [x] |
| C42 | `main` | choice 1 with 0/1/3 lines of text, then 3 (distribution), 4 (score) | [x] |
| C43 | `main` | choice 1 twice (accumulating analyzer state), then 3, 4, 5 | [x] |
| C44 | `main` | choice 2 with: missing file, directory, empty file, 8192-byte file, 8193-byte file, `(null)`-ish empty filename | [x] |
| C45 | `main` | choice 5 before any analysis, with empty pattern, with a matching pattern | [x] |
| C46 | `main` | choice 6 with several lines, >100 tokens, blank first line | [x] |
| C47 | `main` | choices 3/4 before any analysis (empty accumulators) | [x] |
| C48 | `main` | randomized menu scripts: 60 scripts of 1-12 randomly chosen menu operations with randomized payloads, fixed seed | [x] |
| C49 | `main` | 4096+ byte payload for choice 1 (`strncat` saturation) and 256+ byte menu lines | [x] |
| C50 | `main` | stdout redirected to a pipe/file (block buffering) vs the same script with stdout+stderr merged into one file (flush ordering), including >4096 bytes of stdout with stderr writes before/after and 60 consecutive stderr messages | [x] |
| C50b | `main` | stdout **and** stderr on a pseudo terminal: C switches to line buffering, so the interleaving changes | [x] |
| C50d | `main` | stdout **and** stderr merged into the same **pipe** (`prog 2>&1 \| cat`), >8 KiB of output: glibc sizes its buffer from `st_blksize`, which differs between a pipe and a regular file | [x] |
| C50c | `main` | the reader of stdout exits early (`… \| head -c 64`): the C build dies from `SIGPIPE` | [x] |
| C50e | `main` | **fully interactive**: stdin *and* stdout/stderr on the same terminal, so the C `stdin` stream is line buffered too and glibc flushes the line-buffered `stdout` (the `"Choice: "` prompt) before every read | [x] |

## Where each row is verified

| rows | test file |
|------|-----------|
| C1-C21, C11b | `tests/tokenizer_valid.rs` |
| C22-C25, C27-C33 | `tests/analyzer_valid.rs` |
| C26 | `tests/cross_ops.rs` (own process: it deliberately desynchronises the two tokenizers' cumulative counters) |
| C34-C38 | `tests/mainc_valid.rs` |
| C39, C40 (+ virgin-process and flush-at-exit scenarios) | `tests/runner_scenarios.rs` via `examples/ffi_runner.rs` |
| C41-C50e | `tests/driver_e2e.rs` |
| ABI/layout and symbol parity | `tests/layout.rs` |
| harness self-check (proves the comparisons see real data) | `tests/harness_selfcheck.rs` |

Every row is exercised for **both** feature combinations and for the `dev` and
`release` profiles by `./check_features.sh`.
