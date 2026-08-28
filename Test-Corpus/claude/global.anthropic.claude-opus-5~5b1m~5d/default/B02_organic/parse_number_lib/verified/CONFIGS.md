# CONFIGS.md — Phase A: configuration-surface table

Mechanically derived from the branch structure of `c_src/src/lib.c`.

## Public entry points

`include/lib.h` declares exactly one function, and it is also the lowest-level
one (there are no convenience wrappers in this library):

```c
cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);
```

So every row below drives `parse_number` directly, through the `.so` export, for
both C and Rust.

## Axes the C actually branches on

There are **no** runtime option flags, no modes, no `#ifdef`-selected behaviour
(the only `#ifdef`s in `lib.h` are `true`/`false` macro re-definitions, which are
unconditional in effect). The "configuration" is therefore entirely the *state of
the two structs passed in* plus the *shape of the bytes*:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A. `input_buffer` pointer | `NULL`, non-`NULL` | lib.c:23 |
| B. `content` pointer | `NULL`, non-`NULL` | lib.c:23 |
| C. `offset` vs `length` | `offset == 0`, `0 < offset < length`, `offset == length`, `offset > length`, `offset == SIZE_MAX` (wrap in `offset + index`) | lib.c:8 (`can_access_at_index`) |
| D. `length` | `0`, `1`, small, ≥ scanned run, huge | lib.c:8 |
| E. byte class at each scan step | digit `0`–`9` (10 case arms), `+`, `-`, `e`, `E` (share an arm), `.` (own arm, sets `has_decimal_point`), anything else (`default:` → `goto loop_end`) | lib.c:33–59 |
| F. `has_decimal_point` | `false` (replacement loop skipped), `true` (replacement loop runs over `number_string_length` bytes) | lib.c:72–82 |
| G. scanned length `number_string_length` | `0` (⇒ `malloc(1)`, `strtod("")`), `1`, many, very long (≥ 4 KiB, beyond any `strtod` fast path) | lib.c:63,69 |
| H. `strtod` consumption `after_end - number_c_string` | `0` (error), `< number_string_length` (partial: `offset` advances less than the scan), `== number_string_length` (full) | lib.c:85,110 |
| I. `strtod` magnitude vs saturation bounds | `number >= INT_MAX`, `number <= (double)INT_MIN`, in between; incl. `+inf`/`-inf` from range overflow, `±0`, denormal/underflow-to-zero, exact halfway rounding | lib.c:95–106 |
| J. sign of the truncated value | positive (trunc toward 0 = floor), negative (trunc toward 0 = ceil) | lib.c:105 |
| K. `depth` field | any value — never read, never written (must be preserved identically) | absent from lib.c |
| L. `item` initial contents | pre-filled sentinel — must be fully overwritten on success, fully preserved on failure | lib.c:92–108 |

`item == NULL` is *not* an axis: the C omits the check and simply faults (see
`ERRORS.md` row M1), so it is unobservable/uncomparable.

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomized inputs** (fixed-seed xorshift64\*
PRNG, ≥ 200 cases per generative row) unless the row is a single distinguished
state. Byte-for-byte comparison covers: the `cJSON_bool` return, `item->type`,
`item->valueint`, the raw 64 bits of `item->valuedouble`, and all four
`parse_buffer` fields (`content`, `length`, `offset`, `depth`) after the call.

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|-------------------------------------------|-----|
| C01 | `parse_number` | A=`NULL` buffer | [x] |
| C02 | `parse_number` | B=`NULL` content, random `length`/`offset`/`depth` | [x] |
| C03 | `parse_number` | D=`length 0`, C=`offset 0`, non-null content | [x] |
| C04 | `parse_number` | C=`offset == length`, random valid bytes before it | [x] |
| C05 | `parse_number` | C=`offset > length` (1 … `length+64`, and `SIZE_MAX/2`) | [x] |
| C06 | `parse_number` | C=`offset == SIZE_MAX` (wrapping `offset + index`) | [x] |
| C07 | `parse_number` | E=`default` on the first byte: every byte value 0–255 not in `[0-9+\-eE.]`, `length 1..8`, `offset 0` | [x] |
| C08 | `parse_number` | E=all 256 byte values as the single content byte (`length 1`) — covers each switch arm incl. every digit, `+`, `-`, `e`, `E`, `.` | [x] |
| C09 | `parse_number` | G=1 accepted byte, F=false: `"0".."9"`, `"+"`, `"-"`, `"e"`, `"E"` | [x] |
| C10 | `parse_number` | G=1 accepted byte, F=true: `"."` | [x] |
| C11 | `parse_number` | F=false, plain integers, H=full, I=in range, J=positive — random 1–10 digit integers | [x] |
| C12 | `parse_number` | F=false, plain integers with leading `-`, J=negative — random | [x] |
| C13 | `parse_number` | F=false, plain integers with leading `+` — random | [x] |
| C14 | `parse_number` | F=true, decimals `d+.d+`, random mantissa/fraction lengths | [x] |
| C15 | `parse_number` | F=true, `.d+` (no integer part) and `d+.` (trailing point) | [x] |
| C16 | `parse_number` | F=false, exponent forms `d+[eE][+-]?d+`, all 4 exponent spellings | [x] |
| C17 | `parse_number` | F=true, full `[+-]?d*.d*[eE][+-]?d+` grammar, randomized | [x] |
| C18 | `parse_number` | H=partial: accepted-but-unparsable tail, e.g. `"1e"`, `"1e+"`, `"1.2e"`, `"12--3"`, `"1+2"`, `"1.2.3"`, `"1-2"`, `"5ee5"`, `"3EE"` — `offset` must advance only over what `strtod` ate | [x] |
| C19 | `parse_number` | E=`default` after a valid prefix: `"123abc"`, `"1.5}"`, `"7,"`, `"-2 "`, `"0]"` — scan stops early, then H=full | [x] |
| C20 | `parse_number` | B1 bound: valid number running exactly to `length` with **garbage bytes past `length`** in the allocation (must not be read) | [x] |
| C21 | `parse_number` | C=`0 < offset < length`: number embedded mid-buffer, junk before and after | [x] |
| C22 | `parse_number` | I=`number >= INT_MAX`: `2147483647`, `2147483648`, `2147483647.5`, `1e10`, `9007199254740993`, random ≥ 2^31 | [x] |
| C23 | `parse_number` | I=`number <= (double)INT_MIN`: `-2147483648`, `-2147483649`, `-2147483648.0000001`, `-1e10`, random ≤ -2^31 | [x] |
| C24 | `parse_number` | I=just inside the bounds: `2147483646.999...`, `-2147483647.999...`, `±0`, `±1` | [x] |
| C25 | `parse_number` | I=`+inf` via overflow: `"1e309"`, `"1e999"`, `"9"*400`, huge exponents | [x] |
| C26 | `parse_number` | I=`-inf` via overflow: `"-1e309"`, `"-1e999"`, `"-9"+`"9"*400` | [x] |
| C27 | `parse_number` | I=underflow / denormal: `"1e-309"`, `"1e-320"`, `"1e-400"`, `"4.9e-324"`, `"-1e-400"` (signed zero) | [x] |
| C28 | `parse_number` | I=rounding-sensitive: 17–25 significant-digit mantissas, exact-halfway ties (must match libc `strtod` bit-for-bit) | [x] |
| C29 | `parse_number` | G=very long: 4 096 / 10 000 accepted bytes (long digit runs, long zero runs, long exponents) | [x] |
| C30 | `parse_number` | G=very long with F=true: thousands of `.` characters (exercises the replacement loop at scale) | [x] |
| C31 | `parse_number` | K: `depth` = 0, 1, `SIZE_MAX`, random — must be unchanged after both success and failure | [x] |
| C32 | `parse_number` | L: `item` pre-filled with sentinels; check full overwrite on success and full preservation on every failure path | [x] |
| C33 | `parse_number` | repeated/streaming use: call `parse_number` in a loop over one buffer containing several numbers separated by delimiters, feeding the advanced `offset` back in (composed pipeline, not a single call) | [x] |
| C34 | `parse_number` | pure fuzz: random bytes (biased toward `[0-9+\-eE.]`) with random `length`/`offset`/`depth`, 200 000 cases | [x] |
| C35 | `parse_number` | pure fuzz: uniformly random bytes over 0–255, random `length`/`offset`, 50 000 cases | [x] |

## Traceability: row -> test

Every row above is checked off because a named test exercises it and passes
against both `.so`s. `cargo test -- --list` names:

| row | test |
|-----|------|
| C01 | `phase_b_valid::c01_null_input_buffer` |
| C02 | `phase_b_valid::c02_null_content` |
| C03 | `phase_b_valid::c03_zero_length` |
| C04 | `phase_b_valid::c04_offset_equals_length` |
| C05 | `phase_b_valid::c05_offset_past_length` |
| C06 | `phase_b_valid::c06_offset_size_max_wraps` |
| C07 | `phase_b_valid::c07_every_rejected_first_byte` |
| C08 | `phase_b_valid::c08_every_single_byte` |
| C09 | `phase_b_valid::c09_single_accepted_byte_no_decimal_point` |
| C10 | `phase_b_valid::c10_single_dot_sets_has_decimal_point` |
| C11 | `phase_b_valid::c11_random_plain_integers` |
| C12 | `phase_b_valid::c12_random_negative_integers` |
| C13 | `phase_b_valid::c13_random_plus_signed_integers` |
| C14 | `phase_b_valid::c14_random_decimals` |
| C15 | `phase_b_valid::c15_leading_and_trailing_decimal_point` |
| C16 | `phase_b_valid::c16_exponent_forms` |
| C17 | `phase_b_valid::c17_full_grammar_random` |
| C18 | `phase_b_valid::c18_partial_strtod_consumption` |
| C19 | `phase_b_valid::c19_scan_stops_at_unaccepted_byte` |
| C20 | `phase_b_valid::c20_scan_bound_is_length_not_nul` |
| C21 | `phase_b_valid::c21_number_embedded_mid_buffer` |
| C22 | `phase_b_valid::c22_saturate_at_int_max` |
| C23 | `phase_b_valid::c23_saturate_at_int_min` |
| C24 | `phase_b_valid::c24_just_inside_the_saturation_bounds` |
| C25 | `phase_b_valid::c25_overflow_to_positive_infinity` |
| C26 | `phase_b_valid::c26_overflow_to_negative_infinity` |
| C27 | `phase_b_valid::c27_underflow_and_denormals` |
| C28 | `phase_b_valid::c28_rounding_sensitive_mantissas` |
| C29 | `phase_b_valid::c29_very_long_accepted_runs` |
| C30 | `phase_b_valid::c30_very_long_with_decimal_points` |
| C31 | `phase_b_valid::c31_depth_is_preserved` |
| C32 | `phase_b_valid::c32_item_overwrite_and_preservation` |
| C33 | `phase_b_valid::c33_streaming_multiple_numbers_one_buffer` |
| C34 | `phase_b_valid::c34_fuzz_biased_toward_number_bytes` |
| C35 | `phase_b_valid::c35_fuzz_uniform_bytes` |

Cross-cutting matrices that no single row owns (Phase D):

| coverage | test |
|----------|------|
| the whole `shape x offset x length` cross-product, exhaustive to length 3 | `phase_d_matrix::exhaustive_shape_offset_length_matrix` |
| exhaustive 5- and 6-wide sweep over one representative byte per switch arm | `phase_d_matrix::exhaustive_representative_alphabet_length_six` |
| forms libc `strtod` accepts but the C scanner filters first (hex floats, `nan(...)`, `infinity`, whitespace, `_` separators) | `phase_d_matrix::strtod_special_forms_are_filtered_by_the_scanner` |
| every accepted byte proven load-bearing (prefixes, deletions, duplications, substitutions) | `phase_d_matrix::each_accepted_byte_is_load_bearing` |
| 300 000-case sweep with all axes varied at once | `phase_d_matrix::wide_fuzz_all_axes` |
| long inputs x partial `strtod` consumption x non-zero offsets | `phase_d_matrix::long_inputs_with_partial_consumption` |
