# Error Surface

The source contains no error-return macro, `return -1`, `return NULL`,
assertion, error enum, or null-pointer check. Its only explicit range
rejections are normalization branches:

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `validate_and_normalize` | `value > 0 && value < 0100` (decimal 64) | Return `0100` (decimal 64) | [x] |
| 2 | `validate_and_normalize` | `value > 0777` (decimal 511) | Return `0777` (decimal 511) | [x] |

`process_octal_string` and `find_and_replace_char` accept pointers but do not
check them. A null pointer therefore has no C error result; it violates the C
precondition and terminates with `SIGSEGV` on the test platform. Phase C tests
this observable behavior in isolated subprocesses. There are no length
parameters, public enums, or documented integer ranges in this API.
