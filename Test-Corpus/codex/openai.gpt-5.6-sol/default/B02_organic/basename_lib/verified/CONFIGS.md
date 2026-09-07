# Configuration surface

The public header exposes one entry point and no runtime modes, flags, element
types, formats, byte-order options, or feature-controlled APIs. The rows below
cover the input shapes distinguished by the `strrchr` results and pointer-order
branch in `tool_basename`, plus zero-length and trailing-separator boundaries.
Randomized rows include one and many occurrences of the indicated separators.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `tool_basename` | empty C string (zero-length boundary; neither separator exists) | [x] |
| 2 | `tool_basename` | non-empty string with neither `/` nor `\` | [x] |
| 3 | `tool_basename` | one or many `/` separators and no `\`; final `/` is not trailing | [x] |
| 4 | `tool_basename` | one or many `\` separators and no `/`; final `\` is not trailing | [x] |
| 5 | `tool_basename` | both separator kinds occur and the last `/` is later than the last `\` | [x] |
| 6 | `tool_basename` | both separator kinds occur and the last `\` is later than the last `/` | [x] |
| 7 | `tool_basename` | string ends in `/`, so the returned basename is empty | [x] |
| 8 | `tool_basename` | string ends in `\`, so the returned basename is empty | [x] |

Each row passes hundreds of fixed-seed randomized cases under both the default
build and `--no-default-features`.
