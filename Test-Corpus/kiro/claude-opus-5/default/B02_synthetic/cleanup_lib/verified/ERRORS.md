# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every rejection
construct. Raw evidence:

```
$ grep -n "return"                       -> 76: return result;          (only return in the TU)
$ grep -n "goto\|^cleanup:"              -> 44, 68 (goto cleanup;), 74 (label)
$ grep -n "assert"                       -> (none)
$ grep -n "if *("                        -> 42, 66, 84
$ grep -n "NULL"                         -> 37 (init), 86 (dead store)
$ grep -niE "RETURN_ERROR|errno|perror|exit\(|abort"  -> (none)
$ grep -n "#define\|enum"                -> 28, 29 (STRINGIZE / TO_STRING only)
```

Findings that shape the table:

* There is **no error enum, no `RETURN_ERROR` macro, no `errno` use, no
  `assert`, no `exit`/`abort`, and no `return -1` / `return NULL`** anywhere in
  the library. `cleanup` has exactly one `return` statement (`return result;`),
  so **every** path — success *and* rejection — returns the accumulator. There
  is no error sentinel to distinguish; the observable "error result" of a
  rejection is (a) the diagnostic written to stdout and (b) the value of
  `result` at the moment the `goto` is taken.
* `print_result` and `cleanup_resources` return `void` and have no rejection
  logic other than `cleanup_resources`'s null guard.
* The only magic constants are the switch labels `10/20/30/40`, the loop bound
  `4`, and the buffer size `50` (used both for `malloc` and as `snprintf`'s
  truncation limit).

## Table

`[x]` = a differential test exists, calls both `.so` files, and passes.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [ ] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `cleanup` | `lib.c:42` string-validation rejection: `strncmp(input_str, expected_str, strlen(expected_str)) != 0`. Both operands are the in-function literal `"VALID"`, so the trigger is **structurally unreachable** from the FFI boundary — no argument can make it fire. | never taken. Had it fired: print `Input string validation failed.\n`, `goto cleanup`, `cleanup_resources(NULL)` (no-op), `return 0`. Verified as: over 3 000 randomised inputs both libraries agree on return + stdout, neither ever emits the diagnostic, and the success path's non-zero accumulator proves the `return 0` path was not taken. | `err01_string_validation_branch_unreachable_in_both` | [x] |
| 2 | `cleanup` | `lib.c:66` allocation rejection: `malloc(50 * sizeof(char))` returns `NULL`. Not reachable by choosing arguments (50 bytes always succeeds); only reachable under heap exhaustion, which is not an *input*. | print `Memory allocation failed.\n`, `goto cleanup`, `cleanup_resources(NULL)` (no-op), `return result` — the switch accumulator is **still returned**, not an error code. Verified as: over 3 000 randomised inputs neither library emits the diagnostic and both emit the success line instead, with identical returns. | `err02_malloc_failure_branch_unreachable_in_both` | [x] |
| 3 | `cleanup_resources` | `lib.c:84` null guard: `dynamic_str == NULL` | no-op; no free, no output, returns normally. Must not crash. | `err03_cleanup_resources_null_is_noop` | [x] |
| 4 | `cleanup` | integer overflow of the accumulator: `result += numbers[i]` (`lib.c:60`) with `INT_MAX` / `INT_MIN` arguments. C signed overflow is UB; the compiled ground truth wraps two's-complement. | whatever the compiled C `.so` returns, bit-for-bit. Rust uses `wrapping_add` and matches on 16 hand-picked wrap shapes plus 10 000 randomised large-magnitude quadruples — and also under the **debug** profile, where overflow checks are on, proving no panic path. | `err04_accumulator_overflow_matches_compiled_c` | [x] |
| 5 | `print_result` | `label == NULL` passed to `printf("%s: %d\n", label, result)` | glibc's `%s` prints the literal `(null)`, i.e. `(null): <result>\n`. Asserted both as C-vs-Rust equality *and* against that exact literal. | `err05_print_result_null_label`, `row22_print_result_null_label` | [x] |
| 6 | `cleanup_resources` | non-null pointer that was **not** obtained from `malloc` — e.g. a stack address. `lib.c:84` only checks for null, so `free` runs and behaviour is undefined. | **Intentionally not executed**: it would abort the process in *both* libraries and prove nothing. The row is discharged by asserting the only *rejectable* pointer value (null) is treated identically; the defined non-null case is row 7. | `err06_non_malloc_pointer_is_out_of_contract` | [x] |
| 7 | `cleanup_resources` | valid, live `malloc`-obtained pointer (the only non-null input with defined behaviour), including `malloc(0)` | frees it silently; the dead store `dynamic_str = NULL;` at `lib.c:86` writes the local parameter copy only and is unobservable. 1 000 randomised sizes + `malloc(0)`. | `err07_cleanup_resources_live_pointer`, `row24`, `row25` | [x] |

## Generic FFI boundary cases (mandated even though absent from the C's own checks)

| # | case | expected | test | [ ] |
|---|---|---|------|-----|
| G1 | `cleanup` with all-zero arguments (and zero mixed with each label in each position) | `result == 0` / identical | `g01_all_zero_arguments`, `row11` | [x] |
| G2 | `cleanup` at the extremes of `int`: `INT_MIN`, `INT_MIN+1`, `-1`, `1`, `INT_MAX-1`, `INT_MAX` in every position and every ordered pair | identical wrapped return | `g02_int_extremes_in_every_position`, `row09`, `row10` | [x] |
| G3 | `cleanup` one and two steps past each switch label (`±1`, `±2` of `10/20/30/40`) plus `0, 9, 41, 50, -10..-40` | falls to `default`, `result += value` | `g03_one_step_past_each_switch_label`, `row13` (exhaustive `8^4`) | [x] |
| G4 | out-of-range "enum" values across the FFI boundary. The library declares **no enum**; the nearest analogue is `cleanup`'s `int` arguments, whose `switch` has 4 named labels plus `default`, so every `int` outside `{10,20,30,40}` is an out-of-range variant. Covered with label-aliasing values (`10+256`, `10+65536`, …), negated labels, and both `i32` extremes, each alone and interleaved with genuine labels — plus 40 000 full-range/biased randomised quadruples. | identical | `g04_out_of_range_enum_variants`, `row14`, `row15` | [x] |
| G5 | `print_result` label of length 0 | `": <result>\n"` | `g05_print_result_zero_length_label`, `row18` | [x] |
| G6 | `print_result` oversized label — 1 KiB, 64 KiB±1, 1 MiB ("oversized length" probe; `printf` imposes no cap here) | full label echoed | `g06_print_result_oversized_label`, `row19` | [x] |
| G7 | `print_result` label containing `%s %d %n %% %p %x %*s %hhn %1000000d` — the label is a `printf` *argument*, so no format interpretation may occur | `%` echoed verbatim | `g07_print_result_format_specifier_label`, `row20` | [x] |
| G8 | `print_result` label with non-UTF-8 bytes: every single byte `0x80..0xFF`, all 255 non-NUL bytes in one label, and truncated UTF-8 sequences | bytes echoed verbatim; the Rust wrapper must not attempt UTF-8 validation | `g08_print_result_high_byte_label`, `row21` | [x] |
| G9 | `cleanup_resources` null (zero-length/absent buffer) | no-op — same as row 3 | `err03`, `row23` | [x] |
| G10 | interop: value returned by `cleanup` fed straight into `print_result` in one captured stdout window | identical stdout | `g10_cleanup_result_fed_into_print_result`, `row26` | [x] |

## Result

All 7 error-surface rows and all 10 generic boundary rows have a passing
differential test: `phase_c_errors` → **16 passed, 0 failed**, under both the
release and debug profiles and under every feature combination (there is only
the default one).

## Harness credibility (negative controls)

A table of green ticks is only meaningful if the harness can actually go red.
Three single-token mutations were injected into `translation/src/lib.rs`, the
cdylib rebuilt, and the suite re-run; each was caught, and the file was restored
byte-identically afterwards:

| mutation | channel exercised | caught by |
|---|---|---|
| `wrapping_add(20)` → `wrapping_add(21)` in the `10 →20` fall-through | return value | `row07`: `C ret=120` vs `Rust ret=124` |
| `"Processed numbers: %s"` → `"Processed numbers; %s"` | stdout bytes | `row16` byte-diff at offset 17 (`0x3A` vs `0x3B`) |
| `"%s: %d\n"` → `"%s : %d\n"` in `print_result` | stdout bytes | `err05`: `"(null): 0\n"` vs `"(null) : 0\n"` |

A fourth failure mode — a **stale** `.so` silently passing, because
`crate-type = ["cdylib"]` is not refreshed by `cargo test` — is now blocked by
`assert_so_is_fresh` in `tests/common/mod.rs`, verified by `touch`ing
`src/lib.rs` and observing `STALE ARTIFACT`.

