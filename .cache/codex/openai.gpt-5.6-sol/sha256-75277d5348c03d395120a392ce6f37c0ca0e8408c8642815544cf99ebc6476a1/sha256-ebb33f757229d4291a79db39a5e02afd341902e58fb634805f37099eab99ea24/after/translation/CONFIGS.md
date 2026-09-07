# Configuration surface

Mechanical source axes:

- Public entry points: `merge_sort` only.
- Cargo features: none declared. The effective default and
  `--no-default-features` configurations were both verified.
- Runtime options, modes, flags, formats, element types, and byte-order
  choices: none.
- Input shape branch: recursion stops when `hi - lo <= 1`; larger inputs split
  at `(lo + hi) / 2`, making odd and even counts distinct shapes.
- Merge branches: take the left item when its `sort_bits` is less than or equal
  to the right item's value; otherwise take the right item; take the remaining
  left side after the right side is exhausted.
- Although the source contains a `texture_id` tie comparison, the preceding
  `sort_bits <=` branch already returns for equality, so equal `sort_bits`
  always preserve left-before-right order regardless of `texture_id`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `merge_sort` | empty input (`size = 0`) | [x] |
| 2 | `merge_sort` | singleton input (`size = 1`) | [x] |
| 3 | `merge_sort` | two elements with left `sort_bits <=` right | [x] |
| 4 | `merge_sort` | two elements with left `sort_bits >` right | [x] |
| 5 | `merge_sort` | even count at least four, mixed ordering and merge exhaustion | [x] |
| 6 | `merge_sort` | odd count at least three, mixed ordering and uneven split | [x] |
| 7 | `merge_sort` | repeated equal `sort_bits`, arbitrary/distinct `texture_id` values | [x] |
| 8 | `merge_sort` | boundary values `INT_MIN` and `INT_MAX` in mixed-size inputs | [x] |

Each row is exercised with many deterministic randomized inputs, and both the
result buffer and scratch buffer are compared byte-for-byte.
