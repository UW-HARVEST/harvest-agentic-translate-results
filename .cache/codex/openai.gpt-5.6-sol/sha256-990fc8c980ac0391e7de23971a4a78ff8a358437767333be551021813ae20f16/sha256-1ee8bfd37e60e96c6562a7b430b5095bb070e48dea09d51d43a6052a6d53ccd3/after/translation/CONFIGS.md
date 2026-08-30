# Configuration Surface

The C library has no runtime options, modes, flags, feature conditionals,
lengths, element types, formats, or byte-order branches. Its branch axes are:

- `foo`: whether `strchr` finds zero, one, or multiple occurrences.
- `driver`: the independent occurrence counts for fixed bytes `A` and `x`.
- Input shape: empty and nonempty NUL-terminated byte strings. Embedded NUL
  ends the C string, so bytes after it are outside the API input.

`foo(in, '\0')` is excluded from the valid surface because the C loop advances
past the string terminator and then reads outside the object (undefined
behavior). All other byte values are valid, including bytes above `0x7f`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `foo` | empty input; non-NUL target; zero occurrences | [x] |
| 2 | `foo` | nonempty input; non-NUL target; zero occurrences | [x] |
| 3 | `foo` | nonempty input; non-NUL target; exactly one occurrence | [x] |
| 4 | `foo` | nonempty input; non-NUL target; multiple occurrences | [x] |
| 5 | `driver` | empty input; `A` count 0, `x` count 0 | [x] |
| 6 | `driver` | nonempty input; `A` count 0, `x` count 0 | [x] |
| 7 | `driver` | nonempty input; `A` count 0, `x` count 1 | [x] |
| 8 | `driver` | nonempty input; `A` count 0, `x` count many | [x] |
| 9 | `driver` | nonempty input; `A` count 1, `x` count 0 | [x] |
| 10 | `driver` | nonempty input; `A` count 1, `x` count 1 | [x] |
| 11 | `driver` | nonempty input; `A` count 1, `x` count many | [x] |
| 12 | `driver` | nonempty input; `A` count many, `x` count 0 | [x] |
| 13 | `driver` | nonempty input; `A` count many, `x` count 1 | [x] |
| 14 | `driver` | nonempty input; `A` count many, `x` count many | [x] |
