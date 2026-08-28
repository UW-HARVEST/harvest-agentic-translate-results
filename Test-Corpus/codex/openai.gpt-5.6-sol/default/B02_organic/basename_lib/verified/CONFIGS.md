# Configuration Surface

The public surface has one entry point and no runtime options, modes, flags,
element types, lengths, counts, formats, byte-order choices, feature gates, or
compile-time feature combinations. The rows below are the reachable outcomes
of the `s1`/`s2` separator branches in `tool_basename`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `tool_basename` | No options; no `/` or `\` (empty and non-empty strings) | [x] |
| 2 | `tool_basename` | No options; one or more `/`, no `\` | [x] |
| 3 | `tool_basename` | No options; one or more `\`, no `/` | [x] |
| 4 | `tool_basename` | No options; both separator types, last `/` occurs later | [x] |
| 5 | `tool_basename` | No options; both separator types, last `\` occurs later | [x] |

For every row, comparison includes the returned pointer offset, returned suffix
bytes through the terminating NUL, and confirmation that the input buffer is
unchanged.
