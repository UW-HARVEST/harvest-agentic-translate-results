# Configuration-surface table

Mechanical branch axes from `c_src/src/lib.c`:

- state: `accumulator`, `multiplier`, and `operation_count`;
- normalization shape: negative, zero, positive below `0100`, inclusive
  `0100..0777`, and above `0777`;
- `divide_multiplier`: zero versus nonzero divisor;
- string search: empty/nonempty and first matching byte present/absent
  (`memchr` scans only `strlen(str)` bytes);
- `findrep`: active parameter count `0`, `1`, or `>=2`; accumulator
  `<= 0150` versus `> 0150`; multiplier zero/nonzero and `<= 0100` versus
  `> 0100`; final result zero versus nonzero.

The table contains the branch-distinct cross-product combinations exercised by
the C implementation. Rows sharing a test still use many fixed-seed randomized
inputs within that configuration.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `validate_and_normalize` | negative values, including `INT_MIN` | [x] |
| 2 | `validate_and_normalize` | zero | [x] |
| 3 | `validate_and_normalize` | positive `1..077` (1..63), below lower bound | [x] |
| 4 | `validate_and_normalize` | inclusive normal range `0100..0777` (64..511), including both boundaries | [x] |
| 5 | `validate_and_normalize` | above upper bound `>0777` (511), including `INT_MAX` | [x] |
| 6 | `add_to_accumulator` | fresh state; positive, zero, and negative operands | [x] |
| 7 | `add_to_accumulator` | repeated calls accumulating prior state | [x] |
| 8 | `subtract_from_accumulator` | fresh state; `a-b` positive, zero, and negative | [x] |
| 9 | `subtract_from_accumulator` | repeated/interleaved calls using prior accumulator state | [x] |
| 10 | `multiply_with_multiplier` | fresh multiplier; positive operands | [x] |
| 11 | `multiply_with_multiplier` | zero in either operand makes multiplier zero | [x] |
| 12 | `multiply_with_multiplier` | negative operand parity and repeated multiplication | [x] |
| 13 | `divide_multiplier` | nonzero positive divisor; truncating integer division | [x] |
| 14 | `divide_multiplier` | nonzero negative divisor; truncation toward zero/sign change | [x] |
| 15 | `process_octal_string` | zero `octal_val` | [x] |
| 16 | `process_octal_string` | positive values, including octal digit boundaries | [x] |
| 17 | `process_octal_string` | negative values | [x] |
| 18 | `find_and_replace_char` | empty string | [x] |
| 19 | `find_and_replace_char` | nonempty string with search byte absent | [x] |
| 20 | `find_and_replace_char` | match at first byte | [x] |
| 21 | `find_and_replace_char` | match in middle/end; only first match replaced | [x] |
| 22 | `find_and_replace_char` | `search_char` outside unsigned-byte range (C `memchr` conversion) | [x] |
| 23 | `find_and_replace_char` | search byte is NUL; terminator excluded by `strlen` length | [x] |
| 24 | `findrep` | fresh state, all four parameters zero (`active_params == 0`) | [x] |
| 25 | `findrep` | fresh state, exactly one active parameter; each of four positions | [x] |
| 26 | `findrep` | fresh state, two active parameters (`active_params >= 2`) | [x] |
| 27 | `findrep` | fresh state, three or four active parameters | [x] |
| 28 | `findrep` | parameters spanning negative/zero/below-bound/in-range/above-bound normalization shapes | [x] |
| 29 | `findrep` | preloaded accumulator `<= 0150` (104), so subtract branch is not taken | [x] |
| 30 | `findrep` | preloaded or newly raised accumulator `> 0150`, so subtract branch is taken | [x] |
| 31 | `findrep` | multiplier becomes zero, so `both_active` is false and divide branch is not taken | [x] |
| 32 | `findrep` | multiplier nonzero and `<= 0100` (64): `both_active` true, divide branch false | [x] |
| 33 | `findrep` | multiplier `> 0100`: `both_active` true and post-result divide branch taken | [x] |
| 34 | `findrep` | repeated calls preserve all three static state variables | [x] |
| 35 | `findrep` + all low-level stateful exports | interleaved public calls alter state consumed by the composed pipeline | [x] |
| 36 | `findrep` | state/input combination producing final `result == 0`, replaced by `0777` | [x] |
| 37 | `add_to_accumulator`, `subtract_from_accumulator`, `multiply_with_multiplier` | signed arithmetic at `INT_MIN`/`INT_MAX` boundaries, matching the compiled C machine behavior | [x] |
| 38 | all scalar entry points and `findrep` | fixed-seed randomized values across the full 32-bit input domain | [x] |
