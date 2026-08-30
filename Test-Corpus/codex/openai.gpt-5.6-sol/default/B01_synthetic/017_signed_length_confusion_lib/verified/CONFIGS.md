# Configuration Surface

The crate defines no Cargo features, and the C source has no runtime mode,
option, flag, `switch`, or conditional-compilation branch. Rows below cover
both dynamic exports and partition the memory-safe valid input shapes at every
size boundary visible in the C implementation (`0`, `1`, interior, and `99`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printLine` | Non-null pointer to a valid NUL-terminated byte string; randomized empty and non-empty contents without interior NUL bytes. | [x] |
| 2 | `driver` | `data == 0`; copy count is empty and `dest[0]` is terminated. | [x] |
| 3 | `driver` | `data == 1`; one source byte is copied and `dest[1]` is terminated. | [x] |
| 4 | `driver` | `2 <= data <= 98`; randomized interior copy lengths. | [x] |
| 5 | `driver` | `data == 99`; maximum in-bounds copy/index boundary. | [x] |

Rejected `printLine(NULL)` and `driver(data >= 100)` configurations are
enumerated in `ERRORS.md`. Negative `driver` values produce undefined behavior
in C and are not valid configurations.
