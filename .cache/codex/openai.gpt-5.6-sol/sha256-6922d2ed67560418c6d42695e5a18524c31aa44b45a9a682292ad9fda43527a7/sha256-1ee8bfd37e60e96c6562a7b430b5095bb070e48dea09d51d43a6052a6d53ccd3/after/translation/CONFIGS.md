# Configuration Surface

The crate declares no Cargo features and the C source has no conditional
compilation. The rows below are derived from all public C definitions and each
runtime branch or input shape they distinguish. Arithmetic rows use values
whose operations remain in the C `int` range because signed overflow is
undefined in C.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `add_to_accumulator` | Fresh state; arbitrary safe negative/zero/positive `a` and `b` | [x] |
| C02 | `add_to_accumulator` | Existing accumulator; repeated calls verify cumulative state and operation count | [x] |
| C03 | `multiply_with_multiplier` | Fresh state; arbitrary safe negative/zero/positive `a` and `b` | [x] |
| C04 | `multiply_with_multiplier` | Existing multiplier; repeated calls verify cumulative state and operation count | [x] |
| C05 | `subtract_from_accumulator` | Fresh state; arbitrary safe negative/zero/positive `a` and `b` | [x] |
| C06 | `subtract_from_accumulator` | Existing accumulator; repeated calls verify cumulative state and operation count | [x] |
| C07 | `divide_multiplier` | `b == 0`; multiplier remains unchanged and operation count increments | [x] |
| C08 | `divide_multiplier` | `b != 0`; positive and negative divisors use C integer truncation | [x] |
| C09 | `process_octal_string` | `octal_val == 0`; sufficiently sized destination | [x] |
| C10 | `process_octal_string` | Positive `octal_val`, including `INT_MAX`; sufficiently sized destination | [x] |
| C11 | `process_octal_string` | Negative `octal_val`, including `INT_MIN`; sufficiently sized destination | [x] |
| C12 | `validate_and_normalize` | Negative or zero input; returned unchanged | [x] |
| C13 | `validate_and_normalize` | Positive input below octal `0100` (64); clamped to 64 | [x] |
| C14 | `validate_and_normalize` | Input exactly octal `0100` (64) | [x] |
| C15 | `validate_and_normalize` | Input strictly between 64 and octal `0777` (511) | [x] |
| C16 | `validate_and_normalize` | Input exactly octal `0777` (511) | [x] |
| C17 | `validate_and_normalize` | Input above 511; clamped to 511 | [x] |
| C18 | `find_and_replace_char` | Empty string; no searchable bytes | [x] |
| C19 | `find_and_replace_char` | Search byte absent, including search for terminating NUL | [x] |
| C20 | `find_and_replace_char` | One matching byte; first match replaced by `X` | [x] |
| C21 | `find_and_replace_char` | Multiple matching bytes; only first match replaced | [x] |
| C22 | `find_and_replace_char` | Search integer outside unsigned-byte range; `memchr` conversion semantics | [x] |
| C23 | `findrep` | Fresh state; zero active parameters, no add/multiply dispatch | [x] |
| C24 | `findrep` | Fresh state; one active parameter, add dispatch only; each parameter position exercised | [x] |
| C25 | `findrep` | Fresh state; two active parameters, add and multiply dispatch; all position pairs exercised | [x] |
| C26 | `findrep` | Fresh state; three active parameters, add and multiply dispatch | [x] |
| C27 | `findrep` | Fresh state; four active parameters, add and multiply dispatch | [x] |
| C28 | `findrep` | Parameters from each normalization class: negative, below 64, in-range, and above 511 | [x] |
| C29 | `findrep` | Add makes accumulator `<= 0150` (104); subtract dispatch skipped | [x] |
| C30 | `findrep` | Add makes accumulator `> 104`; subtract dispatch taken | [x] |
| C31 | `findrep` | Multiply makes multiplier zero; `both_active` branch skipped | [x] |
| C32 | `findrep` | Accumulator and multiplier nonzero; `both_active` branch taken | [x] |
| C33 | `findrep` | Multiplier `<= 0100` (64); divide dispatch skipped | [x] |
| C34 | `findrep` | Multiplier `> 64`; divide dispatch taken and affects subsequent state | [x] |
| C35 | low-level arithmetic then `findrep` | Precondition all three static variables through public low-level calls; composed stateful pipeline | [x] |
| C36 | `findrep` then low-level arithmetic then `findrep` | Repeated end-to-end calls preserve accumulator, multiplier, and operation count | [x] |

Every row is exercised with deterministic randomized values or randomized
repetitions around its fixed branch-defining boundary.
