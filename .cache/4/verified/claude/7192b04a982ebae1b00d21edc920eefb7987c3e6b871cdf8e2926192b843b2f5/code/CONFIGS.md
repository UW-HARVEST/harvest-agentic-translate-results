# CONFIGS.md — CONFIGURATION-SURFACE TABLE (valid inputs)

## Public entry points

`c_src/include/lib.h` declares exactly one public function:

```c
cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);
```

There is no convenience wrapper and no higher layer — `parse_number` *is* the
lowest-level entry point, and it is driven directly (a `parse_buffer` is
constructed by hand and the `cJSON` out-parameter is pre-poisoned) in every row
below. The two internal function-like macros (`can_access_at_index`,
`buffer_at_offset`) are not exported and are exercised through `parse_number`.

## Axes the C actually branches on

Derived from the `if` / `switch` / loop conditions in `c_src/src/lib.c`:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| **A. `input_buffer` / `content`** | non-NULL (valid path) | line 23 (`NULL` cases are in `ERRORS.md`) |
| **B. `offset`** | `0`; interior (`0 < offset < length`); `length - 1` | `can_access_at_index`, `buffer_at_offset` |
| **C. `length` vs. real data** | `length == real size`; `length <` token end (truncating mid-token, no `'\0'`); `length >` real size but scan stops on a non-charset byte | `offset + index < length` |
| **D. scanned byte class** | `'0'..'9'` / `'+'` / `'-'` / `'e'` / `'E'` (→ `number_string_length++`) — the `has_decimal_point == false` path | `switch` cases, lines 35–50 |
| **E. `'.'` present** | `has_decimal_point == true` → the extra rewrite loop at lines 72–82 runs | line 52–55, 72 |
| **F. terminator** | none (buffer end reached); non-charset byte (`,` `]` `}` space `\t` `"` `:` `\0` `a`..`z` `0x80..0xFF`) hits `default:` → `goto loop_end` | line 57–58 |
| **G. `strtod` consumption** | full temp buffer consumed (`after_end == start + number_string_length`); strict prefix consumed (`after_end < start + len`) | line 110 `offset += after_end - number_c_string` |
| **H. magnitude class** | `number >= INT_MAX`; `number <= (double)INT_MIN`; otherwise `(int)number` truncation-toward-zero | lines 95–106 |
| **I. sign / zero** | `+0`, `-0` (sign of zero must survive into `valuedouble`), positive, negative | via `strtod` |
| **J. exponent form** | none; `e`/`E`; `e+`/`e-`; multi-digit exponent; huge exponent (`+inf` overflow); tiny exponent (subnormal / underflow to 0) | charset accepts `e`, `E`, `+`, `-` |
| **K. `item` out-fields written** | all three of `type`, `valueint`, `valuedouble` on success; **none** on failure | lines 92–108 |
| **L. `depth`** | never touched (must be preserved) | absent from `lib.c` |

## Table — one row per combination the C treats differently

Every row is exercised with **many randomized inputs** (`SEED = 0x5EED_1234_ABCD_0001`,
a deterministic SplitMix64 generator) against **both** `.so` files, comparing the
returned `cJSON_bool`, all three `cJSON` fields (`valuedouble` compared by raw
bit pattern so `-0.0`, `+inf`, `-inf` and NaN payloads must match exactly) and
all four `parse_buffer` fields.

| #   | entry point(s)  | configuration (options set + input shape)                                                                                              | test | [x] |
|-----|-----------------|----------------------------------------------------------------------------------------------------------------------------------------|------|-----|
| C1  | `parse_number`  | axes D,F,K: pure-digit token, `offset = 0`, `length == size`, no terminator, `1..18` random digits (no `'.'` → rewrite loop skipped)      | `cfg_c1_pure_digits` | [x] |
| C2  | `parse_number`  | axes D,I: leading `'+'` / `'-'` + random digits, full consumption                                                                        | `cfg_c2_signed_integers` | [x] |
| C3  | `parse_number`  | axes E,G: `has_decimal_point == true` — `int.frac` with random digit counts on both sides, full consumption                              | `cfg_c3_decimal_point` | [x] |
| C4  | `parse_number`  | axes E,I: sign + decimal point, plus the leading-`'.'` (`".5"`) and trailing-`'.'` (`"5."`) shapes                                        | `cfg_c4_signed_decimal` | [x] |
| C5  | `parse_number`  | axis J: `e` / `E` exponent, no exponent sign, random 1–3-digit exponent                                                                   | `cfg_c5_exponent_unsigned` | [x] |
| C6  | `parse_number`  | axis J: `e`/`E` with explicit `+`/`-` exponent sign, random mantissa (int and fractional) — the widest charset row                       | `cfg_c6_exponent_signed` | [x] |
| C7  | `parse_number`  | axes H,J: overflow to `±inf` (`1e309 … 1e99999`) → `valuedouble` is `±HUGE_VAL`, `valueint` saturates                                    | `cfg_c7_overflow_inf` | [x] |
| C8  | `parse_number`  | axes H,J: underflow (`1e-309 … 1e-99999`, subnormals) → `valuedouble` subnormal or `±0.0`, `valueint == 0`                               | `cfg_c8_underflow_subnormal` | [x] |
| C9  | `parse_number`  | axis H: `number >= INT_MAX` boundary sweep — random values in `[INT_MAX - 4, INT_MAX + 2^20]` rendered as decimals, incl. exact `2147483647` | `cfg_c9_int_max_boundary` | [x] |
| C10 | `parse_number`  | axis H: `number <= (double)INT_MIN` boundary sweep — random values in `[INT_MIN - 2^20, INT_MIN + 4]`, incl. exact `-2147483648`          | `cfg_c10_int_min_boundary` | [x] |
| C11 | `parse_number`  | axis H: in-range truncation — random `f64` in `(INT_MIN, INT_MAX)` printed with 17 significant digits, checking `(int)number` rounding toward zero for both signs | `cfg_c11_in_range_truncation` | [x] |
| C12 | `parse_number`  | axes I,K: `"0"`, `"-0"`, `"+0"`, `"0.0"`, `"-0.0"`, `"0e0"`, `"-0e-0"` — sign-of-zero bit must match                                      | `cfg_c12_zeroes` | [x] |
| C13 | `parse_number`  | axis B: `offset > 0` — the token starts at a random interior offset, preceded by random non-charset junk; `offset` must advance from there  | `cfg_c13_nonzero_offset` | [x] |
| C14 | `parse_number`  | axis F: every distinct terminator byte class (`,` `]` `}` `:` `"` space `\t` `\n` `\r` `\0` `a` `x` `A` `/` `0x80` `0xFF`) after a valid token | `cfg_c14_all_terminators` | [x] |
| C15 | `parse_number`  | axis C: `length` truncates mid-token so the visible prefix is still parsable (`"12345"` seen as `"12"`); relies on `length`, not `'\0'`   | `cfg_c15_truncated_but_parsable` | [x] |
| C16 | `parse_number`  | axes C,F: `length` larger than the token, scan stopped by a non-charset byte (incl. `length = SIZE_MAX`)                                   | `cfg_c16_length_beyond_token` | [x] |
| C17 | `parse_number`  | axis G: strict-prefix consumption — charset-valid but only partly numeric (`"1e"`, `"1e+"`, `"1.2.3"`, `"1-2"`, `"1+2"`, `"1e5e5"`, `"3..4"`, `"7ee7"`, `"9--"`), randomized combinations | `cfg_c17_prefix_consumption` | [x] |
| C18 | `parse_number`  | axes B,C,D,E,F,G,H,I,J **cross-product fuzz**: fully random byte strings over the alphabet `0-9 + - . e E , ] } \0 space a` with random `length`/`offset` (incl. `offset >= length`), 20 000 cases — the only row that can hit combinations not hand-enumerated | `cfg_c18_random_alphabet_fuzz` | [x] |
| C19 | `parse_number`  | pure random bytes (`0x00..0xFF`) with random `length`/`offset`, 20 000 cases — covers the `default:` label for every possible byte value  | `cfg_c19_random_bytes_fuzz` | [x] |
| C20 | `parse_number`  | axis K/L: same `cJSON`/`parse_buffer` reused across a **sequence** of calls, each advancing `offset` through a multi-number document (`"1,2.5,-3e2,…"`) until it fails — verifies the composed pipeline / cumulative `offset` arithmetic, not a single call | `cfg_c20_sequential_document` | [x] |
| C21 | `parse_number`  | axis D: very long tokens (100 … 4096 digits, with and without `'.'`, with huge exponents) — stresses `number_string_length`, the rewrite loop, and `strtod`'s big-decimal path | `cfg_c21_long_tokens` | [x] |
| C22 | `parse_number`  | axis E: `'.'`-heavy tokens where the rewrite loop rewrites *many* bytes (`"1.......2"`, random `'.'` runs) — `has_decimal_point == true` with `strtod` consuming only a prefix | `cfg_c22_many_decimal_points` | [x] |
| C23 | `parse_number`  | axis B: `offset == length - 1` (single last byte visible), randomized last byte over the full charset                                     | `cfg_c23_offset_last_byte` | [x] |
| C24 | `parse_number`  | round-trip of `f64` bit patterns: random `u64` reinterpreted as a finite `f64`, printed with `{:.17e}` and re-parsed — exercises `strtod` correct rounding over the whole exponent range | `cfg_c24_f64_roundtrip` | [x] |
| C25 | `parse_number`  | valid input through **misaligned** `cJSON *` / `parse_buffer *` (skew 1..7 on item, on buffer, and on both) — the C makes no alignment promise | `misaligned_pointers_behave_identically` | [x] |
| C26 | `parse_number`  | **exhaustive**: every string of length 1..5 over the full accepted charset `[0-9+-.eE]` (813 615 inputs) | `exhaustive_charset_len_1_to_4`, `exhaustive_charset_len_5` | [x] |
| C27 | `parse_number`  | **exhaustive**: every string of length 1..4 over charset ∪ stop bytes (`,`, `\0`, space, `a`) — 152 000 inputs | `exhaustive_with_stop_bytes_len_1_to_4` | [x] |
| C28 | `parse_number`  | **exhaustive**: every 3-byte charset content × every `length` ∈ 0..3 × every `offset` ∈ 0..4 (67 500 combinations) | `exhaustive_len_offset_matrix` | [x] |
| C29 | `parse_number`  | **exhaustive**: every one of the 256 byte values as sole content, as leader, and as terminator, across `length` ∈ 0..1 and `offset` ∈ 0..2 | `exhaustive_single_byte_all_values`, `exhaustive_byte_class_cross_charset` | [x] |
| C30 | `parse_number`  | read-guard sweep: the byte at index `length` is always an in-charset digit, so any read past `length` changes the result — cut position swept over all-charset contents at every `offset` | `exhaustive_read_guard_at_length_boundary` | [x] |
| C31 | `parse_number`  | all axes at once, 6 independent seeds × 20 000 cases (incl. random NULL axes), plus huge-`length` and sequence variants | `fuzz_all_axes_multi_seed`, `fuzz_huge_length_with_stop_byte`, `fuzz_sequences` | [x] |

## Harness properties that make these rows meaningful

* **Read guard.** Every content buffer is allocated as `case.content ++ b"88888888"`
  while `parse_buffer::length` is set from `case.content` only. The bytes just
  past `length` are therefore always *in-charset digits*, so an implementation
  that reads even one byte too far produces a different number instead of
  silently reading a harmless heap byte. (This is what turns the
  `can_access_at_index` off-by-one mutants from "survived" into "killed".)
* **Poisoned out-parameters.** `cJSON.type`/`valueint`/`valuedouble` and
  `parse_buffer.depth` are pre-filled with distinctive sentinels, so "field not
  written" and "field written when it should not be" are both observable.
* **Bitwise `f64` comparison.** `valuedouble` is compared as raw `u64` bits, so
  `-0.0` vs `+0.0`, `±inf` and NaN payloads must match exactly.
* **Fresh buffers per implementation.** Each side gets its own copy of the
  content bytes, so a stray write by one cannot influence the other.
* **Same libc `strtod`.** Both `.so`s resolve `strtod` from the process's libc,
  so any value/rounding/end-pointer difference would be a genuine control-flow
  difference rather than a libm discrepancy.
