# Configuration Surface

Build-time axes:

- Cargo declares no features and CMake declares no options.
- The only valid build combination is the empty feature set:
  `--no-default-features --features ""`.

Runtime/API axes mechanically present in `c_src/src/main.c`:

- Public entry point: `main` only (`foo` is `static`).
- Input parser: two decimal `int` conversions through `scanf("%d %d", ...)`.
- Loop/label predicates: `x > 0 || y > 0`, `x == 1 && y == 4`, `x > 0`,
  `y == 0`, and `x < 3`.
- There are no runtime options, modes, flags, element types, byte-order
  choices, pointer arguments, length arguments, or enum arguments.

Rows below are the pruned cross-product of conversion shape and loop predicate
states that the C code treats differently. Randomized values stay in small
ranges because output volume is proportional to the values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `main` | Two valid integers with `x <= 0` and `y <= 0`; outer loop is initially false and stdout is empty | [x] |
| 2 | `main` | Two valid integers with `x > 0` and `y == 0`; each outer iteration takes `x > 0` then `y == 0` | [x] |
| 3 | `main` | Two valid integers with `x == 0` and `y > 0`; `x > 0` is false and `x < 3` repeatedly jumps to `label1` | [x] |
| 4 | `main` | Two valid integers with `x < 0` and `y > 0`; same label cycle as row 3 with a negative `x` state | [x] |
| 5 | `main` | Exact pair `x == 1`, `y == 4`; initial shortcut jumps directly to `label2` and skips one `x` action | [x] |
| 6 | `main` | Both positive, initial `1 <= x <= 3`, excluding `(1,4)`; after decrement, `x < 3` cycles through `label1` in one outer iteration | [x] |
| 7 | `main` | `x >= 4`, `y > 0`, with `y` exhausted while post-decrement `x >= 3`; execution transitions to the `y == 0` outer-loop path | [x] |
| 8 | `main` | `x >= 4`, `y > 0`, with enough `y` to reach post-decrement `x < 3`; execution transitions into the inner `label1` cycle | [x] |
| 9 | `main` | `x > 0`, `y < 0`; `y == 0` never becomes true and the C operation does not terminate; compare a bounded output prefix and nontermination | [x] |
| 10 | `main` | Valid conversions split across spaces/tabs/newlines, with optional signs and trailing non-converted bytes; `%d %d` consumes the same two values | [x] |

Invalid conversion shapes are listed separately in `ERRORS.md`.
