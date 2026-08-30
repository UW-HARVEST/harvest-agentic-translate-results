# Error Surface

Mechanically derived from the two `if` conditions in `src/driver.c`. The C API
has no error return type or error enum, so rejection is observable only through
suppressed work/output.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] 1 | `printLine` | `line == NULL` (inverse of `if (line != NULL)`) | Return `void` without calling `printf`; emit zero bytes. |
| [x] 2 | `driver` | `data >= 100` (inverse of `if (data < 100)`) | Skip `strncpy` and indexed termination; print the initially empty `dest`, emitting exactly `"\n"`. |

`driver(data < 0)` is not rejected by C: it enters the copy branch, converts
the negative count to `size_t`, and indexes before `dest`. That execution has
undefined behavior and therefore has no stable C result to place in this
error-surface table or compare in-process.

There are no `RETURN_ERROR` uses, error enums, `assert` calls, explicit
`return -1`/`return NULL` statements, switches, or public enum parameters.
