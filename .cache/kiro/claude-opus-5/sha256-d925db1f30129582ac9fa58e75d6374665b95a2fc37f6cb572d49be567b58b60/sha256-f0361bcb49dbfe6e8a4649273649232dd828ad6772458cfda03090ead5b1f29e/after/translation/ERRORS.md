# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library defines **no** error
enum, no `RETURN_ERROR` macro, no `assert`, and never returns `NULL` or `-1` as
a sentinel. Its entire rejection surface consists of *guard conditions*
(`if (b != 0)`, `if (found)`), *range clamps* (`lower_threshold` /
`upper_threshold`), and one *sentinel substitution* (`result == 0 -> 0777`).
Each distinct guarded branch below is one row.

Grep evidence: every `if`/`return` in `src/lib.c` is at lines 38, 44, 50, 54,
58, 69, 81, 82, 83, 84, 85, 89, 126, 132, 137, 142, 157, 161, 169, 173.

Octal constants in the source: `0100` = 64, `0150` = 104, `0777` = 511,
`0123` = 83, `010` = 8, `01`/`02`/`03`/`04` = 1/2/3/4.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| 1  | `divide_multiplier` | `b == 0` (divisor rejected, line 54) | division **skipped**; `operation_count++` still happens; returns `multiplier` unchanged | [x] |
| 2  | `divide_multiplier` | `b == 1` (no-op divisor, boundary just past 0) | `multiplier /= 1`; returns `multiplier` unchanged, count incremented | [x] |
| 3  | `divide_multiplier` | `b < 0` (negative divisor accepted, C truncates toward zero) | `multiplier` = C truncating division (e.g. `1 / -3 == 0`) | [x] |
| 4  | `divide_multiplier` | `b` = `INT_MIN` | `multiplier / INT_MIN` → `0` for any `multiplier != INT_MIN` | [x] |
| 5  | `validate_and_normalize` | `value == 0` → `is_nonzero == 0`, clamp branch skipped (line 81) | returns `0` **unclamped** (not `0100`) | [x] |
| 6  | `validate_and_normalize` | `value < 0` → `value > 0` false, clamp branch skipped | returns `value` **unclamped** (negatives are *not* raised to `0100`) | [x] |
| 7  | `validate_and_normalize` | `value == INT_MIN` (extreme negative) | returns `INT_MIN` unchanged | [x] |
| 8  | `validate_and_normalize` | `0 < value < 0100` (under lower threshold, line 82) | returns `0100` (= 64) | [x] |
| 9  | `validate_and_normalize` | `value == 0100 - 1` (= 63, one step below range) | returns `64` | [x] |
| 10 | `validate_and_normalize` | `value == 0100` (= 64, exactly at lower bound) | returns `64` (pass-through, no clamp) | [x] |
| 11 | `validate_and_normalize` | `value == 0777` (= 511, exactly at upper bound) | returns `511` (pass-through, `>` is strict) | [x] |
| 12 | `validate_and_normalize` | `value == 0777 + 1` (= 512, one step past range, line 84) | returns `511` | [x] |
| 13 | `validate_and_normalize` | `value == INT_MAX` (oversized) | returns `511` | [x] |
| 14 | `find_and_replace_char` | needle absent from string → `memchr` returns `NULL`, `if (found)` false (line 69) | string left **completely unmodified** | [x] |
| 15 | `find_and_replace_char` | empty string (`strlen == 0`, zero length passed to `memchr`) | no match possible; string unmodified | [x] |
| 16 | `find_and_replace_char` | `search_char == 0` (NUL) — the terminator is *outside* the `strlen` span | never found; string unmodified | [x] |
| 17 | `find_and_replace_char` | `search_char > 255` (e.g. `0x141`) — C `memchr` truncates to `unsigned char` | matches `'A'` (`0x41`); first `'A'` replaced by `'X'` | [x] |
| 18 | `find_and_replace_char` | `search_char < 0` (e.g. `-191`) — truncates to `unsigned char` `0x41` | matches `'A'`; first `'A'` replaced by `'X'` | [x] |
| 19 | `find_and_replace_char` | `search_char == 'X'` (already the replacement) | first `'X'` overwritten with `'X'`; string unchanged in value | [x] |
| 20 | `findrep` | all four params `0` → `active_params == 0`, so `>= mode_add` (1) and `>= mode_multiply` (2) both false | none of `operations[0..1]` invoked; state untouched by them | [x] |
| 21 | `findrep` | `active_params == 1` → `>= 1` true but `>= 2` false | only `operations[0]` invoked | [x] |
| 22 | `findrep` | `accumulator <= 0150` (= 104) → subtract branch rejected (line 142) | `operations[2]` **not** invoked | [x] |
| 23 | `findrep` | `accumulator == 0` **or** `multiplier == 0` → `both_active` false (line 157) | `accumulator + multiplier` **not** added to `result` | [x] |
| 24 | `findrep` | `multiplier <= 0100` (= 64) → divide branch rejected (line 161) | `operations[3]` **not** invoked | [x] |
| 25 | `findrep` | computed `result == 0` → `!result_exists` (line 169) | return value replaced by sentinel `0777` (= 511) | [x] |
| 26 | `process_octal_string` | `octal_val == 0` | writes `"Octal: 00, Decimal: 0"` (the literal `0` prefix plus `%o` of 0) | [x] |
| 27 | `process_octal_string` | `octal_val < 0` — `%o` reinterprets as `unsigned`, `%d` stays signed | e.g. `-1` → `"Octal: 037777777777, Decimal: -1"` | [x] |
| 28 | `process_octal_string` | `octal_val == INT_MIN` (widest `%o` and `%d` output) | `"Octal: 020000000000, Decimal: -2147483648"` (41 bytes, still fits `buffer[50]`) | [x] |

## Generic FFI boundary conditions (covered even though not table rows)

| condition | C behaviour | tested |
|-----------|-------------|--------|
| out-of-range "enum"/mode ints passed as `search_char` | no enum type exists; any `int` accepted and truncated to `unsigned char` — rows 16–19 | [x] |
| out-of-range ints for `validate_and_normalize` / `findrep` params | no enum type exists; every `int` is in-domain — rows 5–13, 20–25 | [x] |
| zero length | rows 15, 26 (`strlen == 0`, value `0`) | [x] |
| oversized length / value | rows 12, 13, 28 (`INT_MAX`, `INT_MIN`, `> 0777`) | [x] |
| signed overflow in `+`, `-`, `*` (C UB, wraps at `-O0`) | wrapping two's-complement; Rust uses `wrapping_*` | [x] |
| NULL `char*` to `process_octal_string` / `find_and_replace_char` | dereferences NULL → SIGSEGV in **both** implementations. Undefined behaviour; deliberately **not** exercised (a crashing test proves nothing and kills the harness). Rust matches by also dereferencing raw pointers with no null check. | n/a |
| `INT_MIN / -1` in `divide_multiplier` | C UB; on x86-64 raises SIGFPE. Deliberately **not** exercised. Rust uses `wrapping_div` (returns `INT_MIN`) rather than panicking. | n/a |

## Phase C results

Test file: `tests/phase_c_errors.rs` — `errNN_…` maps 1:1 onto rows 1–28, plus
three generic boundary sweeps and the harness self-check.

```
cargo test --test phase_c_errors
test result: ok. 32 passed; 0 failed
```

| test | rows covered |
|------|--------------|
| `err01_divide_by_zero_skips_division` … `err04_divide_by_int_min` | 1–4 |
| `err05_validate_zero_is_not_clamped` … `err13_validate_int_max` | 5–13 |
| `err14_needle_absent…` … `err19_needle_equals_replacement_char` | 14–19 |
| `err20_findrep_no_active_params…` … `err25_findrep_zero_result_becomes_sentinel_0777` | 20–25 |
| `err26_octal_zero` … `err28_octal_int_min_widest_output` | 26–28 |
| `generic_out_of_domain_int_sweep` | out-of-domain ints on every parameter (14 extremes, 14² pairs per stateful fn, every `findrep` slot) |
| `generic_zero_and_oversized_lengths` | zero / one-byte / 120-byte strings |
| `generic_all_byte_values_as_needle` | all 256 needle values, plus ±256 offsets, against a haystack holding every non-NUL byte |
| `harness_state_isolation` | proves the fresh-`dlopen` trick works |

Every row asserts the **specific** value, not just "both failed": e.g. row 1
pins the returned multiplier to `1000`, row 12 pins `validate(512) == 511`,
row 27 pins the exact bytes `"Octal: 037777777777, Decimal: -1"`.

### Row 25 needed a derived input

The random sweep never produced `result == 0`, so the `0777` sentinel branch was
initially untested (the first version of this test reported
`sentinel observed 0 times`). Solving the arithmetic by hand gives it: with all
params `0` the only contributions are the memchr offset `9`,
`accumulator + multiplier`, and `operation_count * 010`, so pre-seeding with
`add_to_accumulator(-18, 0)` yields `9 + (-18 + 1) + 1*8 == 0`. The test now
drives the branch three independent ways plus a −200..0 scan, and **asserts**
the sentinel was observed at least once.

## Negative control (does the suite actually catch bugs?)

`scripts/mutation_check.sh` and `scripts/mutation_check2.sh` inject known bugs
into `src/lib.rs` one at a time, rebuild, and require the suite to fail.

**26 / 26 behaviour-changing mutants caught**, including: octal signedness,
both threshold constants, dropping the `value > 0` guard, sign-extending the
`memchr` comparison, both `findrep` state thresholds, the memchr offset, the
sentinel value, both static initialisers, the `operation_count` scale factor,
each `operation_count++`, the subtract sign, swapped operation arguments,
`&&` → `||` in `both_active`, and both mode thresholds.

**3 mutants correctly NOT caught** — they are semantically unobservable through
the public ABI, so flagging them would be a false positive:

| mutant | why it is unobservable |
|--------|------------------------|
| `0123` → `0124` in `findrep`'s `process_octal_string` call | writes only to the local `message[100]`, which `findrep` never reads back into `result` |
| `'O'` → `'o'` in `findrep`'s `find_and_replace_char` call | same — mutates only `message` |
| `"Function …"` → `"function …"` | does not move the `'p'` that `memchr` finds, so the offset stays `9` |

`message`, `final_message` and the `strcpy` into them are dead computations in
the C source too. That the offset path *is* covered is proven separately by the
`search_buffer_shift` mutant (prepends a byte so the offset becomes `10`) —
caught, 20 failing tests.

A further three mutants were tried and dropped as **equivalent**, not missed:
`value <= lower_threshold` (64 → 64 either way), `value >= upper_threshold`
(511 → 511), and `*p == needle as c_char` (identical low 8 bits to `as u8`).
