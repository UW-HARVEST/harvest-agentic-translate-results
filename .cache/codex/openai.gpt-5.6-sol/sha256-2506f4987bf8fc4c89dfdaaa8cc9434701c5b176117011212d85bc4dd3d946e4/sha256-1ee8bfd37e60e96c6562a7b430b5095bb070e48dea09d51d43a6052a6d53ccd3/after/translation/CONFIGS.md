# Configuration Surface

The public API has no runtime option object, mode, flag, format, element type,
byte order, compile-time feature, or conditional implementation branch. The
matrix below is the pruned cross-product of the branches and data shapes that
the C implementation does distinguish:

- `static_alias`: caller-owned pointer versus a pointer aliasing the internal
  static, and `<`, `==`, or `>` relative to the current internal value.
- `driver`: negative, zero, one, or many iterations, with the initial value
  `<`, `==`, or `>` the current internal value when an iteration runs.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|---|
| 1 | `static_alias` | caller-owned `outer`, `*outer < inner` | [x] |
| 2 | `static_alias` | caller-owned `outer`, `*outer == inner` | [x] |
| 3 | `static_alias` | caller-owned `outer`, `*outer > inner` | [x] |
| 4 | `static_alias` | reuse the caller-owned pointer returned by a prior `< inner` call | [x] |
| 5 | `static_alias` | feed the returned `&inner` pointer into the next direct call, so `outer` aliases `inner` | [x] |
| 6 | `driver` | `iterations < 0`; no loop iterations and no output | [x] |
| 7 | `driver` | `iterations == 0`; empty operation and no output | [x] |
| 8 | `driver` | `iterations == 1`, `initial_value < inner` | [x] |
| 9 | `driver` | `iterations == 1`, `initial_value == inner` | [x] |
| 10 | `driver` | `iterations == 1`, `initial_value > inner` | [x] |
| 11 | `driver` | `iterations > 1`, `initial_value < inner` | [x] |
| 12 | `driver` | `iterations > 1`, `initial_value == inner` | [x] |
| 13 | `driver` | `iterations > 1`, `initial_value > inner` | [x] |

Cargo configuration inventory:

- `Cargo.toml` declares no features and no default feature set.
- The sole feature combination is therefore the empty set. It is verified
  both normally and with `--no-default-features`.
