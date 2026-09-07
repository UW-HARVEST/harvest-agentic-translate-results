# CONFIGS.md — configuration-surface table (Phase A, gate for Phase B)

## The public surface

`c_src/include/lib.h` declares exactly **one** function:

```c
cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);
```

There is no init/config function, no global, no `#ifdef`-guarded mode, and no
convenience wrapper layered over a lower-level entry point — `parse_number`
*is* the lowest-level entry point and it is called directly by every test.
"Configuration" for this library is therefore carried entirely by:

* the **`parse_buffer` state** the caller sets up (`content`, `length`,
  `offset`, `depth`), and
* the **`cJSON` pre-state** (`type`, `valueint`, `valuedouble`), which the C
  only partially overwrites, so the caller's bytes are observable on the
  failure paths.

## Axes the C actually branches on

| axis | values the C distinguishes | source |
|---|---|---|
| A1 `input_buffer` | NULL / non-NULL | line 23 |
| A2 `content` | NULL / non-NULL | line 23 |
| A3 `offset` | `0` / `0 < offset < length` / `== length` / `> length` / `SIZE_MAX` (wrapping `offset + index`) | line 8 |
| A4 `length` | `0` / `1` / small / large / bogus-huge | line 8 |
| A5 byte class in scan loop | `0`–`9` / `+` / `-` / `e` / `E` (all → `number_string_length++`) · `.` (→ also sets `has_decimal_point`) · anything else (→ `goto loop_end`) | lines 34–58 |
| A6 `has_decimal_point` | `false` (skip replacement loop) / `true` (run it) | lines 72–83 |
| A7 `strtod` consumption | all of the temp string / a strict prefix of it / none (→ error) | line 85 |
| A8 saturation branch | `number >= (double)INT_MAX` / `number <= (double)INT_MIN` / in-range `(int)number` | lines 95–106 |
| A9 sign | `+` / `-` / `-0.0` (sign bit only) | via `strtod` |
| A10 magnitude | overflow → `±inf` / underflow → `±0` or subnormal / normal / needs >17-digit rounding | via `strtod` |
| A11 run termination | run stops at a non-accepted byte / run reaches `length` | line 8 vs `default:` |
| A12 `content` NUL-termination | NUL-terminated / not terminated at all (garbage follows) | lines 68–70 |
| A13 `depth` | never read/written — must be preserved bit-exactly | (absent from `lib.c`) |
| A14 `item` pre-state | tag `== cJSON_Number` / tag out-of-range; sentinel `valueint`/`valuedouble` | lines 92–108 |

## Configuration rows (each verified with randomized inputs, seed `0x5EED_C0FFEE`)

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `parse_number` | A3=`0`, A4=small, run = 1–9 digits reaching end of buffer, A6=false, A7=all, A8=in-range, A11=reaches `length`, A12=not terminated | `cfg_row01_digits_to_end` | [x] |
| 2 | `parse_number` | A3=`0`, digit run followed by a random non-accepted terminator byte, A6=false, A11=early stop | `cfg_row02_digits_then_terminator` | [x] |
| 3 | `parse_number` | A3=`0 < offset < length` (mid-buffer), digit run at `offset`, junk before and after | `cfg_row03_mid_buffer_offset` | [x] |
| 4 | `parse_number` | A3=`== length` (empty accessible range), A4>0, A7=none | `cfg_row04_offset_at_length` | [x] |
| 5 | `parse_number` | A3=`> length` (offset past end), A7=none | `cfg_row05_offset_past_length` | [x] |
| 6 | `parse_number` | A3=`SIZE_MAX` and `SIZE_MAX - k` — exercises the wrapping `offset + index` in `can_access_at_index` | `cfg_row06_offset_size_max_wrap` | [x] |
| 7 | `parse_number` | A4=`0` with non-NULL `content`, A7=none | `cfg_row07_length_zero` | [x] |
| 8 | `parse_number` | A4=`1`, single accepted byte (digit / `+` / `-` / `.` / `e` / `E`) — mixes A7=all and A7=none | `cfg_row08_length_one_each_accepted_byte` | [x] |
| 9 | `parse_number` | A6=**true**, fractional form `<digits>.<digits>`, A8=in-range | `cfg_row09_decimal_fraction` | [x] |
| 10 | `parse_number` | A6=true, leading `.` (`".<digits>"`) — `strtod` accepts | `cfg_row10_leading_decimal_point` | [x] |
| 11 | `parse_number` | A6=true, trailing `.` (`"<digits>."`) — `strtod` accepts and consumes the `.` | `cfg_row11_trailing_decimal_point` | [x] |
| 12 | `parse_number` | A6=true, **two** decimal points (`"1.2.3"`) → A7=**strict prefix**, offset advances by less than the scanned run | `cfg_row12_two_decimal_points_partial` | [x] |
| 13 | `parse_number` | exponent, lowercase `e`, A6=false (`"<d>e<d>"`) | `cfg_row13_exponent_lower_e` | [x] |
| 14 | `parse_number` | exponent, uppercase `E` (`"<d>E<d>"`) | `cfg_row14_exponent_upper_e` | [x] |
| 15 | `parse_number` | exponent with explicit `+`/`-` sign (`"<d>e+<d>"`, `"<d>e-<d>"`) | `cfg_row15_signed_exponent` | [x] |
| 16 | `parse_number` | **dangling** exponent (`"<d>e"`, `"<d>E"`, `"<d>e+"`, `"<d>e-"`) → A7=strict prefix; `strtod` backs up to before the `e` | `cfg_row16_dangling_exponent_partial` | [x] |
| 17 | `parse_number` | **repeated** exponent (`"1e2e3"`, `"1E2E3"`) → A7=strict prefix | `cfg_row17_repeated_exponent_partial` | [x] |
| 18 | `parse_number` | leading `+` (`"+<d>"`) — accepted by the scan *and* by `strtod` | `cfg_row18_leading_plus` | [x] |
| 19 | `parse_number` | leading `-` (`"-<d>"`), A9=negative, A8=in-range | `cfg_row19_leading_minus` | [x] |
| 20 | `parse_number` | A9=`-0.0` (`"-0"`, `"-0.0"`, `"-0e5"`) — sign bit of `valuedouble` must survive; `valueint` must be `0` | `cfg_row20_negative_zero` | [x] |
| 21 | `parse_number` | embedded sign mid-run (`"1-2"`, `"1+2"`, `"1--2"`) — scan accepts the whole run, `strtod` stops at the sign → A7=strict prefix | `cfg_row21_embedded_sign_partial` | [x] |
| 22 | `parse_number` | A10=overflow → `+inf` (`"1e999"`, `"9"*400`), A8=`>= INT_MAX` | `cfg_row22_overflow_to_pos_inf` | [x] |
| 23 | `parse_number` | A10=overflow → `-inf` (`"-1e999"`), A8=`<= INT_MIN` | `cfg_row23_overflow_to_neg_inf` | [x] |
| 24 | `parse_number` | A10=underflow: `"1e-999"` → `0`, `"1e-320"`/`"4.9e-324"` → **subnormal**, `"-1e-999"` → `-0.0` | `cfg_row24_underflow_and_subnormal` | [x] |
| 25 | `parse_number` | A8 boundary: values straddling `(double)INT_MAX` — `2147483646/7/8`, `2147483647.5`, `nextafter` neighbours | `cfg_row25_int_max_boundary` | [x] |
| 26 | `parse_number` | A8 boundary: values straddling `(double)INT_MIN` — `-2147483647/8/9`, `-2147483647.5` | `cfg_row26_int_min_boundary` | [x] |
| 27 | `parse_number` | A10=>17 significant digits, random long mantissa + random exponent → exercises `strtod` correct rounding identically on both sides | `cfg_row27_long_mantissa_rounding` | [x] |
| 28 | `parse_number` | A4=large: accepted run of 1–4096 bytes (large `malloc` size), random accepted-charset soup | `cfg_row28_very_long_accepted_run` | [x] |
| 29 | `parse_number` | **property fuzz**, charset restricted to `[0-9+\-eE.]` — every byte accepted, so A5/A6/A7 hit every combination at random; both A7=all/prefix/none occur | `cfg_row29_fuzz_accepted_charset` | [x] |
| 30 | `parse_number` | **property fuzz**, full `0..=255` random bytes at a random `offset` in a random-length buffer — A11 early stop dominates; covers arbitrary junk | `cfg_row30_fuzz_arbitrary_bytes` | [x] |
| 31 | `parse_number` | A12=**not NUL-terminated**: numeric run occupies the final bytes of the allocation with no NUL anywhere (verifies the C's own `'\0'` append is reproduced) | `cfg_row31_unterminated_content` | [x] |
| 32 | `parse_number` | A4=bogus-huge `length` (`SIZE_MAX`, `length ≫ allocation`) with a non-accepted byte inside the real allocation stopping the scan | `cfg_row32_bogus_huge_length` | [x] |
| 33 | `parse_number` | **stateful pipeline**: one buffer holding many numbers separated by delimiters, `parse_number` called repeatedly so each call starts from the `offset` the previous call left — the composed multi-call path | `cfg_row33_sequential_multi_call_pipeline` | [x] |
| 34 | `parse_number` | A14: `item` pre-filled with an out-of-range type tag and sentinel `valueint`/`valuedouble` (incl. NaN and `-0.0` sentinels), on both the success and the failure path | `cfg_row34_item_prestate_sentinels` | [x] |
| 35 | `parse_number` | A13: `depth` set to `0`, `1`, `SIZE_MAX`, random — must be preserved bit-exactly (checked on every row by `assert_same`, plus explicitly here) | `cfg_row35_depth_preserved` | [x] |
| 36 | `parse_number` | hex/keyword-looking input (`"0x1A"`, `"0X1f"`, `"nan"`, `"inf"`, `"NaN"`, `"Infinity"`, `"null"`, `"true"`) — the scan's charset stops before `strtod` could ever see a hex float or a NaN/inf keyword | `cfg_row36_hex_and_keyword_lookalikes` | [x] |
| 37 | `parse_number` | leading whitespace / NUL (`" 12"`, `"\t12"`, `"\n12"`, `"\012"`) — `strtod` *would* skip whitespace but the scan stops first | `cfg_row37_leading_whitespace_or_nul` | [x] |
| 38 | `parse_number` | ABI check: `size_of`/`align_of`/field offsets of `cJSON` and `parse_buffer` as seen by the C `.so` vs the Rust `.so` | `abi_struct_layout_matches_c` | [x] |

| 39a | `parse_number` | **EXHAUSTIVE** — every string of length 1–4 over the 16-byte alphabet `0123456789+-eE.}` (69 904 inputs). Covers every combination of scan byte-class, `has_decimal_point`, and all three `strtod` consumption outcomes with no reliance on the RNG. | `cfg_row39a_exhaustive_len1_to_4` | [x] |
| 39b | `parse_number` | **EXHAUSTIVE** — every length-5 string over the same alphabet (1 048 576 inputs) | `cfg_row39b_exhaustive_len5` | [x] |
| 39c | `parse_number` | **EXHAUSTIVE** — every 2-byte input over the full `0..=255` range, no charset restriction (65 536 inputs) | `cfg_row39c_exhaustive_all_byte_pairs` | [x] |
| 39d | `parse_number` | **EXHAUSTIVE** — every length-3 string over a 24-byte mixed alphabet (accepted bytes + whitespace + NUL + JSON delimiters + `xX`) × **every start offset 0..=3** (55 296 calls) | `cfg_row39d_exhaustive_len3_every_offset` | [x] |
| 39e | `parse_number` | **EXHAUSTIVE** — every length-6 string over the 16-byte alphabet (16 777 216 inputs) | `cfg_row39e_exhaustive_len6` | [x] |
| 39f | `parse_number` | **EXHAUSTIVE** — every length-7 string over the 16-byte alphabet (268 435 456 inputs). `#[ignore]`d for runtime; executed and passing (`cargo test --release -- --ignored`, 31 s). | `cfg_row39f_exhaustive_len7` | [x] |

| 40 | `parse_number` | **`LC_NUMERIC` locale axis.** The C hard-codes `decimal_point = '.'` (so its localisation loop is a no-op) but then delegates to libc `strtod`, whose radix character *is* locale-dependent. Verified under the `C` locale **and** under `de_DE.utf8` (comma radix, where `strtod` stops at `'.'` instead of consuming it — changing both the value and the resulting `offset`). Includes an exhaustive length-1..=4 sweep over an alphabet containing **both** `.` and `,`. | `tests/locale.rs::locale_axis_c_de_and_fr` | [x] |

Every row is driven through **both** `.so` files via `libloading` (never by
calling the Rust function directly), and every call compares, byte-for-byte:
the `cJSON_bool` return value, the full 16-byte `cJSON` image (so `type`,
`valueint`, and the *bit pattern* of `valuedouble`, incl. NaN payload and the
sign of zero), and the full 32-byte `parse_buffer` image (so `content`,
`length`, `offset`, `depth`).
