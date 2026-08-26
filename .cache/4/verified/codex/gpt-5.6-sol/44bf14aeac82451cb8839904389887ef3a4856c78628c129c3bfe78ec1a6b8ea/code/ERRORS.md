# Error Surface

Mechanical search predicates:

```text
if(line != NULL)
if (fgets(inputBuffer, CHAR_ARRAY_SIZE, stdin) != NULL)  [bad]
if (fgets(inputBuffer, CHAR_ARRAY_SIZE, stdin) != NULL)  [goodB2G]
if (fabs(data) > 0.000001)
```

There are no error enums, assertions, `RETURN_ERROR` uses, `return -1`, or
`return NULL` statements. `CHAR_ARRAY_SIZE` is 20, so `fgets` accepts at most
19 input bytes per call.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `printLine` | `line == NULL` | Return without writing bytes. | [x] |
| 2 | `bad` | `fgets(inputBuffer, 20, stdin) == NULL` (EOF or stream read failure before any byte) | Write `fgets() failed.\n`, retain `data = 0.0F`, then write the platform C cast result of `100.0 / 0.0F` (`-2147483648\n` in this build). | [x] |
| 3 | `good` (`goodB2G`) | `fgets(inputBuffer, 20, stdin) == NULL` (EOF or stream read failure before any byte) | After `50\n`, write `fgets() failed.\nThis would result in a divide by zero\n`. | [x] |
| 4 | `good` (`goodB2G`) | `!(fabs(data) > 0.000001)` after successful `fgets`/`atof`; this includes finite `fabs(data) <= 0.000001` and NaN | After `50\n`, write `This would result in a divide by zero\n`. | [x] |

Generic FFI boundaries: row 1 is the only public nullable data pointer. The
`main` entry point ignores both `argc` and `argv`, including a null `argv`.
No public API accepts a length or enum, so zero/oversized lengths and invalid
enum discriminants are not applicable.
