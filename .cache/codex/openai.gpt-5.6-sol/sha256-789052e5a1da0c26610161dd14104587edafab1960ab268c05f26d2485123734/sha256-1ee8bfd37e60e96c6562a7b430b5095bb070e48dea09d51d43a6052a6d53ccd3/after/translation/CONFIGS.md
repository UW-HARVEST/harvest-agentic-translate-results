# Configuration surface

Rows are branch-equivalence configurations derived from every exported
function and every `if` in `../c_src/src/lib.c`. Integer cases avoid C signed
overflow and division overflow, which are undefined behavior rather than valid
C configurations. “Fresh” means a newly loaded copy of the shared library,
whose state is `accumulator = 0`, `multiplier = 1`, and
`operation_count = 0`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `add_to_accumulator` | fresh state; both inputs zero | [x] |
| 2 | `add_to_accumulator` | fresh/stateful sequence; positive, negative, and mixed-sign inputs | [x] |
| 3 | `multiply_with_multiplier` | fresh state; one or both inputs zero, making multiplier zero | [x] |
| 4 | `multiply_with_multiplier` | fresh/stateful sequence; positive, negative, and mixed-sign nonzero inputs | [x] |
| 5 | `subtract_from_accumulator` | fresh state; equal inputs (zero delta) | [x] |
| 6 | `subtract_from_accumulator` | fresh/stateful sequence; positive and negative deltas | [x] |
| 7 | `divide_multiplier` | `b == 0`; division skipped but count incremented | [x] |
| 8 | `divide_multiplier` | `b != 0`; positive and negative divisors, including non-exact truncating division | [x] |
| 9 | `process_octal_string` | `octal_val == 0` | [x] |
| 10 | `process_octal_string` | positive values, including `1`, octal boundaries, and `INT_MAX` | [x] |
| 11 | `process_octal_string` | negative values, including `-1` and `INT_MIN` (`%o` observes unsigned representation) | [x] |
| 12 | `find_and_replace_char` | empty NUL-terminated string | [x] |
| 13 | `find_and_replace_char` | nonempty string; searched byte absent | [x] |
| 14 | `find_and_replace_char` | searched byte present once | [x] |
| 15 | `find_and_replace_char` | searched byte present multiple times; only first occurrence replaced | [x] |
| 16 | `find_and_replace_char` | search value outside unsigned-byte range; `memchr` conversion finds/does not find its low byte | [x] |
| 17 | `find_and_replace_char` | search value is NUL; terminator excluded by `strlen`, so no replacement | [x] |
| 18 | `find_and_replace_char` | long valid NUL-terminated string | [x] |
| 19 | `validate_and_normalize` | value negative, including `INT_MIN`; returned unchanged | [x] |
| 20 | `validate_and_normalize` | value zero; returned unchanged | [x] |
| 21 | `validate_and_normalize` | positive value `1..63`; clamped to octal `0100` (64) | [x] |
| 22 | `validate_and_normalize` | value exactly octal `0100` (64) | [x] |
| 23 | `validate_and_normalize` | value `65..510`; returned unchanged | [x] |
| 24 | `validate_and_normalize` | value exactly octal `0777` (511) | [x] |
| 25 | `validate_and_normalize` | value `512..INT_MAX`; clamped to octal `0777` (511) | [x] |
| 26 | `findrep` | fresh state; zero active parameters, no add/multiply call, accumulator inactive | [x] |
| 27 | `findrep` | fresh state; one active parameter in position 1 or 2, add runs and accumulator becomes active | [x] |
| 28 | `findrep` | fresh state; one active parameter only in position 3 or 4, add runs with two zero operands and accumulator stays inactive | [x] |
| 29 | `findrep` | fresh state; at least two active parameters, multiply runs; `p3 * p4 == 0` | [x] |
| 30 | `findrep` | fresh state; at least two active parameters, nonzero multiplier `<=` octal `0100`; divide branch not taken | [x] |
| 31 | `findrep` | fresh state; at least two active parameters, multiplier `>` octal `0100`; post-result divide branch taken | [x] |
| 32 | `findrep` | normalized `p1 + p2 >` octal `0150`; subtract branch taken | [x] |
| 33 | `findrep` | accumulator nonzero and multiplier nonzero; both-active contribution taken | [x] |
| 34 | `findrep` | accumulator zero or multiplier zero; both-active contribution skipped | [x] |
| 35 | `findrep` | each parameter position traverses negative, zero, low-positive clamp, in-range, and high-positive clamp classes | [x] |
| 36 | low-level mutators then `findrep` | preloaded positive/negative/zero accumulator and multiplier plus nonzero operation count exercise all state-dependent branches | [x] |
| 37 | all stateful integer entry points | randomized multi-call sequences verify independent persistent state and operation-count interactions | [x] |

Cargo feature axes: none. `Cargo.toml` declares no `[features]` table, so the
only build configuration is the default/no-feature build.

Validation modes completed:

- default Cargo mode: **passed**
- `--no-default-features`: **passed**
