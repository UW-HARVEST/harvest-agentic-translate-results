# Configuration Surface

## Build Matrix

`Cargo.toml` has no `[features]` table and CMake has no options or conditional
definitions. There is one valid build-time configuration:

| # | Cargo invocation | C configuration | [ ] |
|---|------------------|-----------------|-----|
| B1 | `--no-default-features` with no feature names | default CMake configuration | [x] |

## Runtime And Input Matrix

Rows are derived from public-header entry points and each distinct branch or
boundary in `analyzer.c` and `tokenizer.c`. Token text is randomized with a
fixed seed within the listed shape unless the row is a fixed boundary.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `get_tokenizer_ops` | retrieve all five callbacks and invoke through the returned function pointers | [x] |
| 2 | `tokenizer_load_text`, `tokenizer_next_token` | empty C string; immediate EOF token | [x] |
| 3 | `tokenizer_load_text` | one-byte valid input | [x] |
| 4 | `tokenizer_load_text` | largest valid input, length `MAX_BUFFER_SIZE - 1` (`8191`) | [x] |
| 5 | `tokenizer_next_token` | spaces, tab, vertical tab, form feed, and carriage return skipped before a token | [x] |
| 6 | `tokenizer_next_token` | newline token, including line/column transition | [x] |
| 7 | `tokenizer_next_token` | alphabetic/underscore start with alphanumeric/underscore continuation; non-keyword identifier | [x] |
| 8 | `tokenizer_next_token` | each of the 32 exact keyword strings | [x] |
| 9 | `tokenizer_next_token` | integer number | [x] |
| 10 | `tokenizer_next_token` | number containing one decimal point | [x] |
| 11 | `tokenizer_next_token` | number encountering a second decimal point; number ends and dot is dispatched separately | [x] |
| 12 | `tokenizer_next_token` | closed double-quoted and single-quoted strings | [x] |
| 13 | `tokenizer_next_token` | string containing backslash plus escaped byte | [x] |
| 14 | `tokenizer_next_token` | unterminated string stopped by EOF | [x] |
| 15 | `tokenizer_next_token` | unterminated string stopped by newline, leaving newline for the next token | [x] |
| 16 | `tokenizer_next_token` | slash followed by slash; single-line comment stopped by newline/EOF | [x] |
| 17 | `tokenizer_next_token` | slash followed by star; terminated multi-line comment | [x] |
| 18 | `tokenizer_next_token` | slash followed by star; unterminated multi-line comment | [x] |
| 19 | `tokenizer_next_token` | bare slash or slash followed by a non-comment byte; C still emits a one-byte comment because it re-peeks the slash | [x] |
| 20 | `tokenizer_next_token` | each single operator byte in `+-*/%=<>!&|^~?:` | [x] |
| 21 | `tokenizer_next_token` | each recognized two-byte operator (`==`, `!=`, `<=`, `>=`, `&&`, `||`, `++`, `--`, `->`, `<<`, `>>`) | [x] |
| 22 | `tokenizer_next_token` | operator followed by a byte that does not form a recognized pair | [x] |
| 23 | `tokenizer_next_token` | each punctuation byte in `(){}[];,.` | [x] |
| 24 | `tokenizer_next_token` | word/number at the 255-byte token cap; remaining bytes become subsequent tokens | [x] |
| 25 | `tokenizer_next_token` | string at the `MAX_TOKEN_LENGTH - 2` scan cap | [x] |
| 26 | `tokenizer_next_token` | line and block comments at their distinct token scan caps | [x] |
| 27 | `tokenizer_peek_token`, `tokenizer_next_token` | empty lookahead: first peek scans once; repeated peek and next return the cached token | [x] |
| 28 | `tokenizer_reset`, `tokenizer_peek_token` | reset clears a pending lookahead and restores position/line/column | [x] |
| 29 | `tokenizer_get_stats` | non-null lines/tokens/chars outputs after mixed tokenization; counts are cumulative | [x] |
| 30 | `tokenizer_reset`, `tokenizer_get_stats` | reset rewinds input but deliberately preserves cumulative statistics | [x] |
| 31 | `analyzer_init` | initialize with callbacks from `get_tokenizer_ops`; reset analyzer tracking arrays | [x] |
| 32 | `analyze_text` | empty text | [x] |
| 33 | `analyze_text` | randomized mixes of identifiers, keywords, numbers, operators, comments, strings, punctuation, whitespace, newlines, and unknown bytes | [x] |
| 34 | `analyze_text` | repeated analyses; returned line/character fields use cumulative tokenizer statistics | [x] |
| 35 | `analyze_text`, `print_token_distribution` | repeated and tied words; bubble-sort ordering and counts | [x] |
| 36 | `analyze_text`, `print_token_distribution` | more than 100 distinct words and more than 10 displayed words; tracking/display caps | [x] |
| 37 | `print_token_distribution` | initialized analyzer with no analyzed tokens | [x] |
| 38 | `calculate_complexity_score` | keywords/operators plus punctuation counts below, at, and above the `/ 10` threshold | [x] |
| 39 | `calculate_complexity_score` | comment subtraction makes the score negative; clamp to zero | [x] |
| 40 | `find_patterns` | non-empty substring with zero, one, and many token matches; tokenizer reset and coordinates | [x] |
| 41 | `find_patterns` | empty pattern; `strstr(value, "")` matches every non-EOF token | [x] |
