# Error Surface

The source has no error enum, error-return macro, assertion, or public
length/enum argument. These rows cover every explicit null/range rejection and
the unsafe below-range input admitted by the C condition. Process results were
measured against the default C build.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `printLine` | `line == NULL` | return `void` without writing any bytes | [x] |
| 2 | `main` | `fgets(inputBuffer, 14, stdin) == NULL` because stdin is at EOF or reports a read error | call `printLine("fgets() failed.")`, retain `data == -1`, then enter the `data < 100` block; the default build terminates with `SIGSEGV` and emits no buffered stdout bytes | [x] |
| 3 | `main` | `atoi(inputBuffer) < 0` | enter the `data < 100` block and convert the negative count to `size_t` for `strncpy`; the default build terminates with `SIGSEGV` and emits no stdout bytes | [x] |
| 4 | `main` | `atoi(inputBuffer) >= 100` (including the one-past-maximum value `100`) | reject the copy by skipping the `data < 100` block, write one newline for empty `dest`, and return `0` | [x] |

There are no C enum parameters for out-of-range enum testing and no public
length parameter to test independently. Input longer than 13 bytes is a valid
`fgets` truncation shape and is covered in `CONFIGS.md`.
