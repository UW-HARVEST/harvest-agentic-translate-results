# Configuration Surface

The sole public and lowest-level entry point is `driver(int x, int y)`. There
are no runtime options, modes, flags, element types, formats, byte-order
choices, compile-time feature branches, convenience wrappers, or lower-level
public calls.

Rows below are the branch partitions mechanically derived from:

- outer loop: `x > 0 || y > 0`;
- special jump: `x == 1 && y == 4`;
- x block: `x > 0`;
- continue: `y == 0`;
- inner jump: `x < 3`;
- signed `int` boundary shapes.

“Bounded prefix” means comparing a deterministic output prefix in isolated
child processes. This is required for very large positive values and for
`x > 0, y < 0`, where the C control flow does not terminate after reaching
`x < 3`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x <= 0, y <= 0`: outer loop is skipped, including zero and `INT_MIN` | [x] |
| 2 | `driver` | `x > 0, y == 0`: x block runs, then the y-zero branch continues the outer loop | [x] |
| 3 | `driver` | `x <= 0, y > 0`: x block is skipped and `x < 3` loops through labels until y reaches zero | [x] |
| 4 | `driver` | `x == 1, y == 4`: special jump skips the first x block | [x] |
| 5 | `driver` | `x > 0, y > 0`, not special, and x becomes `< 3`: inner jump revisits both labeled blocks | [x] |
| 6 | `driver` | `x > 0, y > 0`, not special, and x remains `>= 3`: inner jump is not taken and the outer loop repeats | [x] |
| 7 | `driver` | `x > 0, y < 0`: bounded prefix of the nonterminating label cycle | [x] |
| 8 | `driver` | `INT_MAX` in x or y with the other argument nonpositive: bounded prefix of valid but impractically long execution | [x] |
