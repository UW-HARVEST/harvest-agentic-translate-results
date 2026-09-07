# Configuration-Surface Table

Mechanical branch inventory:

- Public entry points: `jumpnode` only.
- `operation_mode`: `0001`, `0002`, `0003`, `0004`, and default. Modes
  `0001`, `0002`, `0004`, and default reject in the shipped state and are
  covered by `ERRORS.md`.
- Valid mode `0003` formats both integer inputs with `%d`, computes twice the
  resulting byte length plus octal `010`, then adds only `flags & 0177`.
- No Cargo features are declared. The default and `--no-default-features`
  builds are still both exercised.

For every row, randomized values cover the full signed `int` domain with a
fixed seed and a mandatory boundary corpus containing zero, positive and
negative one-/multi-digit values, `INT_MIN`, and `INT_MAX`. The rows are the
cross-product classes that the only explicit valid-path option expression,
`flags & 0177`, distinguishes with respect to low and ignored high bits.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `jumpnode` | mode `0003`; `flags == 0` (low 7 bits zero, no high bits); randomized `node_id` and `depth` signed-int shapes | [x] |
| C2 | `jumpnode` | mode `0003`; flags use only low 7 bits and `flags & 0177 != 0`; randomized low-bit values plus randomized `node_id` and `depth` signed-int shapes | [x] |
| C3 | `jumpnode` | mode `0003`; at least one high bit set but `flags & 0177 == 0`, including positive and negative flags; randomized `node_id` and `depth` signed-int shapes | [x] |
| C4 | `jumpnode` | mode `0003`; both ignored high bits and nonzero low 7 bits set, including positive and negative flags; randomized `node_id` and `depth` signed-int shapes | [x] |

There is no executable target in either build description, so binary stdout
comparison is not applicable.

All rows pass under the default build and `--no-default-features`.
