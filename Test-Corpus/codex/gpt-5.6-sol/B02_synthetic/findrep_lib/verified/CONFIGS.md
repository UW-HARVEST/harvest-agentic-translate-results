# Configuration Surface

The C shared object exports all eight entry points below, although only
`findrep` is declared in `include/lib.h`. Randomized checks use fresh library
instances where initial state is specified.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| 1 | `add_to_accumulator` | Fresh accumulator; randomized integer pair | [x] |
| 2 | `add_to_accumulator` | Nonzero accumulated state; repeated randomized calls | [x] |
| 3 | `multiply_with_multiplier` | Fresh multiplier (`1`); randomized integer pair | [x] |
| 4 | `multiply_with_multiplier` | Prior nonzero multiplier; repeated randomized calls | [x] |
| 5 | `subtract_from_accumulator` | Fresh accumulator; randomized integer pair | [x] |
| 6 | `subtract_from_accumulator` | Nonzero accumulated state; repeated randomized calls | [x] |
| 7 | `divide_multiplier` | `b == 0`; multiplier must remain unchanged | [x] |
| 8 | `divide_multiplier` | `b != 0`; positive and negative operands, truncation toward zero | [x] |
| 9 | `process_octal_string` | `octal_val == 0`; writable destination | [x] |
| 10 | `process_octal_string` | Positive values, including `INT_MAX`; writable destination | [x] |
| 11 | `process_octal_string` | Negative values, including `INT_MIN`; writable destination | [x] |
| 12 | `find_and_replace_char` | Empty NUL-terminated string | [x] |
| 13 | `find_and_replace_char` | Nonempty string with no matching byte | [x] |
| 14 | `find_and_replace_char` | Match at first byte | [x] |
| 15 | `find_and_replace_char` | Match later or repeated; replace only first match | [x] |
| 16 | `find_and_replace_char` | Embedded NUL; matching bytes after NUL are ignored by `strlen` | [x] |
| 17 | `find_and_replace_char` | Search integer outside byte range; `memchr` uses its low unsigned byte | [x] |
| 18 | `validate_and_normalize` | Nonpositive value (`INT_MIN..=0`); unchanged | [x] |
| 19 | `validate_and_normalize` | Positive value below `0100`; clamp to `0100` | [x] |
| 20 | `validate_and_normalize` | Inclusive range `0100..=0777`; unchanged, including boundaries | [x] |
| 21 | `validate_and_normalize` | Value above `0777`, including `INT_MAX`; clamp to `0777` | [x] |
| 22 | `findrep` | Fresh state; zero active parameters; add/multiply modes skipped | [x] |
| 23 | `findrep` | One active parameter in slot 1/2; accumulator `<= 0150`; add only | [x] |
| 24 | `findrep` | One active parameter in slot 1/2; accumulator `> 0150`; subtract branch | [x] |
| 25 | `findrep` | One active parameter in slot 3/4; add receives two zeroes | [x] |
| 26 | `findrep` | Two active parameters in slots 1+2; multiply becomes zero, subtract runs | [x] |
| 27 | `findrep` | Two active parameters in slots 3+4; accumulator zero, multiplier/divide branch | [x] |
| 28 | `findrep` | Two active parameters split across slot groups; accumulator `<= 0150`, multiplier zero | [x] |
| 29 | `findrep` | Two active parameters split across slot groups; accumulator `> 0150`, multiplier zero | [x] |
| 30 | `findrep` | Three active, zero in slot 1/2; accumulator `<= 0150`, nonzero multiplier `> 0100`, both-active and divide branches | [x] |
| 31 | `findrep` | Three active, zero in slot 1/2; accumulator `> 0150`, subtract/both-active/divide branches | [x] |
| 32 | `findrep` | Three active, zero in slot 3/4; multiplier becomes zero, subtract runs, both-active suppressed | [x] |
| 33 | `findrep` | Four active; add, multiply, subtract, both-active, and divide branches | [x] |
| 34 | `add_to_accumulator`, `findrep` | Preexisting accumulator `> 0150`; zero active parameters still trigger subtract | [x] |
| 35 | `multiply_with_multiplier`, `findrep` | Preexisting multiplier `> 0100`; zero active parameters still trigger divide | [x] |
| 36 | `add_to_accumulator`, `findrep` | Preexisting nonzero accumulator and default multiplier; both-active branch without thresholds | [x] |
| 37 | `add_to_accumulator`, `multiply_with_multiplier`, `findrep` | Preexisting zero multiplier suppresses both-active branch | [x] |
| 38 | `add_to_accumulator`, `findrep` | Preload accumulator to `-18`; computed result is zero and falls back to `0777` | [x] |
| 39 | `findrep` | Multiple calls on one instance; accumulator, multiplier, and operation count persist | [x] |
