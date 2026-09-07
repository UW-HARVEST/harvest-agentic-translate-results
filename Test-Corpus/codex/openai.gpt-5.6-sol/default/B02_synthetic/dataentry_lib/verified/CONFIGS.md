# Configuration-surface table

The only public entry point is `dataentry(int mode, int param1, int param2,
int param3)`. There are no Cargo features and no executable target. Rows are
the cross-product of branches the C implementation actually distinguishes,
with equivalent values grouped when they take the same branch.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `dataentry` mode 1 | fallback count (`param1 <= 0`, effective count 5), target hit (`0 <= param2 < 5`); `param3` ignored | [x] |
| 2 | `dataentry` mode 1 | fallback count (`param1 <= 0`, effective count 5), target miss (`param2 < 0` or `param2 >= 5`); `param3` ignored | [x] |
| 3 | `dataentry` mode 1 | one entry (`param1 == 1`), target hit (`param2 == 0`); `param3` ignored | [x] |
| 4 | `dataentry` mode 1 | one entry (`param1 == 1`), target miss (`param2 != 0`); `param3` ignored | [x] |
| 5 | `dataentry` mode 1 | many entries (`param1 > 1`), target hit (`0 <= param2 < param1`); `param3` ignored | [x] |
| 6 | `dataentry` mode 1 | many entries (`param1 > 1`), target miss (`param2 < 0` or `param2 >= param1`); `param3` ignored | [x] |
| 7 | `dataentry` mode 2 | fallback count (`param1 <= 0`, effective count 3), zero multiplier (`param2 == 0`), so `param3` is ignored | [x] |
| 8 | `dataentry` mode 2 | fallback count (`param1 <= 0`, effective count 3), nonzero multiplier and nonzero total, then add `param3` | [x] |
| 9 | `dataentry` mode 2 | one entry (`param1 == 1`), zero multiplier, so `param3` is ignored | [x] |
| 10 | `dataentry` mode 2 | one entry (`param1 == 1`), nonzero multiplier and nonzero total, then add `param3` | [x] |
| 11 | `dataentry` mode 2 | many entries (`param1 > 1`), zero multiplier, so `param3` is ignored | [x] |
| 12 | `dataentry` mode 2 | many entries (`param1 > 1`), nonzero multiplier and nonzero total, then add `param3` | [x] |
| 13 | `dataentry` mode 2 | fallback count (`param1 <= 0`, effective count 3), nonzero multiplier produces wrapped total zero, so `param3` is ignored | [x] |
| 14 | `dataentry` mode 2 | one entry (`param1 == 1`), nonzero multiplier produces wrapped total zero, so `param3` is ignored | [x] |
| 15 | `dataentry` mode 2 | many entries (`param1 > 1`), nonzero multiplier produces wrapped total zero, so `param3` is ignored | [x] |
| 16 | `dataentry` mode 3 | lookup coordinate `(row, col) = (0, 0)`, then add `param3` | [x] |
| 17 | `dataentry` mode 3 | lookup coordinate `(row, col) = (0, 1)`, then add `param3` | [x] |
| 18 | `dataentry` mode 3 | lookup coordinate `(row, col) = (0, 2)`, then add `param3` | [x] |
| 19 | `dataentry` mode 3 | lookup coordinate `(row, col) = (1, 0)`, then add `param3` | [x] |
| 20 | `dataentry` mode 3 | lookup coordinate `(row, col) = (1, 1)`, then add `param3` | [x] |
| 21 | `dataentry` mode 3 | lookup coordinate `(row, col) = (1, 2)`, then add `param3` | [x] |
| 22 | `dataentry` mode 3 | lookup coordinate `(row, col) = (2, 0)`, then add `param3` | [x] |
| 23 | `dataentry` mode 3 | lookup coordinate `(row, col) = (2, 1)`, then add `param3` | [x] |
| 24 | `dataentry` mode 3 | lookup coordinate `(row, col) = (2, 2)`, then add `param3` | [x] |
| 25 | `dataentry` mode 3 | lookup coordinate `(row, col) = (3, 0)`, then add `param3` | [x] |
| 26 | `dataentry` mode 3 | lookup coordinate `(row, col) = (3, 1)`, then add `param3` | [x] |
| 27 | `dataentry` mode 3 | lookup coordinate `(row, col) = (3, 2)`, then add `param3` | [x] |
| 28 | `dataentry` default | any `mode` other than 1, 2, or 3; result is `8 * param1`; `param2` and `param3` ignored | [x] |
