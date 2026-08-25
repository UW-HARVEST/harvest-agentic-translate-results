# Error Surface

The C sources contain no assertions and no error-return macro. Each row below
comes from an explicit rejection, error token, or public-pointer guard in
`analyzer.c` or `tokenizer.c`. Rows 8-10 record the generic FFI boundaries
required by the verification protocol.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `analyze_text` | analyzer has not been initialized (`!initialized`) | print `Error: Analyzer not initialized\n` to stderr and return an all-zero `analysis_result_t` | [x] |
| 2 | `analyze_text` | configured `load_text(text)` callback returns nonzero | print `Error: Failed to load text\n` to stderr and return an all-zero `analysis_result_t` | [x] |
| 3 | `find_patterns` | analyzer has not been initialized (`!initialized`) | return without output | [x] |
| 4 | `find_patterns` | `pattern == NULL` | return without output | [x] |
| 5 | `tokenizer_load_text` | `text == NULL` | return `-1` | [x] |
| 6 | `tokenizer_load_text` | `strlen(text) >= MAX_BUFFER_SIZE` (`8192`) | print `Error: Input text too large\n` to stderr and return `-1` | [x] |
| 7 | `tokenizer_next_token` | current byte matches none of newline, identifier, number, quote, slash/comment, operator, or punctuation branches | return a one-byte token with type `TOKEN_ERROR` (`11`) | [x] |
| 8 | `analyze_text` | initialized with normal tokenizer operations, then `text == NULL` | tokenizer load returns `-1`; analyzer prints `Error: Failed to load text\n` and returns an all-zero result | [x] |
| 9 | `tokenizer_get_stats` | any one or all output pointers are `NULL` | skip each null output independently; no error or write through null | [x] |
| 10 | `analyze_text` | callback returns out-of-range `token_type_t` value `12` (one past `TOKEN_ERROR`, but still inside the C count array) | accept token, increment internal slot 12, ignore it in the result switch, and continue to EOF | [x] |
