# CONFIGS.md — configuration surface table (Phase A, gates Phase B)

## Enumeration of the axes the C actually branches on

Derived from `c_src/include/lib.h` (public surface) and every `if` / `switch` /
`#ifdef` in `c_src/src/lib.c`.

**Public entry points (complete set).** `include/lib.h` declares exactly one
function: `parse_number(cJSON *const, parse_buffer *const)`. There are no
convenience wrappers and no one-shot helpers — `parse_number` *is* the
lowest-level entry point, so "test the low-level API, not just the wrappers"
reduces to driving `parse_number` directly with fully hand-built
`parse_buffer` / `cJSON` state, which is what every row below does.

**Runtime options / modes / flags.** The library exposes no setters and no
global mode. The entire configuration is carried in the two caller-owned
structs, so the option axes are the struct fields:

| axis | field / source | states the C distinguishes |
|------|----------------|----------------------------|
| A1 `content` | `parse_buffer.content` | `NULL` (→ ERRORS E2) vs non-`NULL`; also *what bytes it holds* (axes A5–A9) |
| A2 `length` | `parse_buffer.length` | `0`; small; `== offset`; `< offset`; `SIZE_MAX` (oversized) |
| A3 `offset` | `parse_buffer.offset` | `0`; interior `0 < offset < length`; `== length`; `> length`; wrapping (`offset + i` overflows `size_t`) |
| A4 `depth` | `parse_buffer.depth` | declared in the public struct but **never read or written** by `parse_number` — must round-trip unchanged for arbitrary values |
| A5 accepted-run length | `switch` in the scan loop | `0`; `1`; many; runs that reach `length` with no terminator |
| A6 `has_decimal_point` | `case '.'` sets it; gates the `'.' → decimal_point` rewrite loop | `false` (loop skipped) vs `true` (loop runs over `number_string_length` bytes) |
| A7 exponent spelling | `case 'e'` / `case 'E'` | absent; `e`; `E`; `e+`/`e-`/`E+`/`E-` |
| A8 sign spelling | `case '+'` / `case '-'` | absent; leading `-`; leading `+`; interior/duplicated signs (accepted by the scan, rejected mid-string by `strtod`) |
| A9 terminator | `default: goto loop_end` | no terminator (run ends at `length`); terminator is `'\0'`; terminator is whitespace; terminator is any of the other 241 non-accepted byte values |
| A10 `strtod` consumption | `number_c_string == after_end` | zero bytes consumed (→ E4, `false`); partial (`after_end` inside the run — `offset` advances less than the run); full (`after_end` at the `'\0'`) |
| A11 magnitude regime | `number >= INT_MAX` / `number <= (double)INT_MIN` / `else (int)number` | `+inf`; `> INT_MAX`; exactly `INT_MAX`; in-range positive; `+0.0`; `-0.0`; subnormal; in-range negative; exactly `INT_MIN`; `< INT_MIN`; `-inf` |
| A12 `item` initial state | `cJSON.type` / `.valueint` / `.valuedouble` | arbitrary garbage on entry — the C writes all three only on the success path, none on failure paths |

**Compile-time (`#ifdef`) branches.** `lib.h` has `#ifdef true` / `#ifdef false`
guards, which only `#undef` a possible pre-existing macro; they select no
alternative code. `lib.c` has no `#ifdef` at all. `translation/Cargo.toml`
declares no `[features]`. Therefore there is exactly **one** build
configuration; Phase D's cross-feature requirement collapses onto it.

## Table — one row per meaningful combination the C treats differently

Every row is exercised with **many randomized inputs** (seeded `SplitMix64`,
fixed seed `0x5EED_1234_ABCD_EF01`) against both `.so`s, comparing the return
value, all three `cJSON` fields (`valuedouble` compared **by bit pattern**), and
all four `parse_buffer` fields.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `parse_number` | A3=0, A6=false, A7=absent, A8=absent, A9=`'\0'`, A11=in-range positive small integer — plain unsigned integer, random digit count 1..9 | `c1_plain_positive_int` | [x] |
| C2 | `parse_number` | as C1 but A8=leading `-` → A11=in-range negative | `c2_plain_negative_int` | [x] |
| C3 | `parse_number` | as C1 but A8=leading `+` (accepted by scan **and** by `strtod`) | `c3_leading_plus_int` | [x] |
| C4 | `parse_number` | A6=**true** (decimal point present → rewrite loop runs), A7=absent, random fraction, A11=in-range | `c4_fraction_no_exponent` | [x] |
| C5 | `parse_number` | A6=true, A7=`e` lowercase, unsigned exponent, A11=in-range | `c5_fraction_lower_e` | [x] |
| C6 | `parse_number` | A6=true, A7=`E` uppercase, unsigned exponent, A11=in-range | `c6_fraction_upper_e` | [x] |
| C7 | `parse_number` | A6=true, A7=`e+`/`e-`/`E+`/`E-` (all four), A8 random leading sign, full float spelling | `c7_full_float_all_exponent_spellings` | [x] |
| C8 | `parse_number` | A6=false, A7=`e`/`E` with signed exponent, integer mantissa (`1e5`, `-2E-3`) | `c8_int_mantissa_with_exponent` | [x] |
| C9 | `parse_number` | A11=`+inf` via huge exponent (`e999`…`e400000`) → E7 saturation, `valuedouble == inf` | `c9_positive_infinity` | [x] |
| C10 | `parse_number` | A11=`-inf` via `-…e999` → E8 saturation, `valuedouble == -inf` | `c10_negative_infinity` | [x] |
| C11 | `parse_number` | A11 boundary sweep around `INT_MAX`: `2147483645 … 2147483649` incl. `.0`, `.5`, `.9999999` and `2147483647` exactly (`>=` boundary) | `c11_int_max_boundary` | [x] |
| C12 | `parse_number` | A11 boundary sweep around `INT_MIN`: `-2147483646 … -2147483650` incl. `-2147483648` exactly (`<=` boundary, must saturate) | `c12_int_min_boundary` | [x] |
| C13 | `parse_number` | A11=`+0.0` / `-0.0` (`"0"`, `"-0"`, `"0.0"`, `"-0.0"`, `"-0e5"`) — sign-of-zero must round-trip in `valuedouble` bits | `c13_signed_zero` | [x] |
| C14 | `parse_number` | A11=subnormal / underflow (`1e-320`, `4.9e-324`, `1e-999`) — `strtod` sets `ERANGE`, C ignores it; `valueint` must be `0` | `c14_subnormal_and_underflow` | [x] |
| C15 | `parse_number` | A11=in-range with many significant digits (17–40 digits) — exercises `strtod` rounding identically | `c15_high_precision_mantissa` | [x] |
| C16 | `parse_number` | A9=**no terminator**: the accepted run runs exactly to `length`, and the bytes at/after `length` are poison (`0xFF`, more digits). Verifies `can_access_at_index` bound and that nothing past `length` is read | `c16_run_ends_at_length_with_poison_after` | [x] |
| C17 | `parse_number` | A9=terminator is a **random non-accepted byte**, run length 1..12, poison after | `c17_random_terminator_byte` | [x] |
| C18 | `parse_number` | A9 sweep over **all 256 byte values** as the byte following a valid `"12"` prefix (the 15 accepted values extend the run; the other 241 stop it) | `c18_terminator_sweep_all_256` | [x] |
| C19 | `parse_number` | A3=**interior** `0 < offset < length` with digits *before* `offset` that must be ignored, and a terminator after the run | `c19_interior_offset` | [x] |
| C20 | `parse_number` | A10=**partial consumption**: the accepted run is longer than what `strtod` accepts (`"1.2.3"`, `"1e"`, `"1e+"`, `"1-2"`, `"12e-"`, `"1.2E"`, `"5+"`, `".5.5"`) → `offset` advances by `after_end - start` only | `c20_partial_strtod_consumption` | [x] |
| C21 | `parse_number` | A5/A8: **duplicated / interior signs** accepted by the scan (`"+-1"`, `"1+1"`, `"-+2"`, `"3-4e5"`) — mixes partial consumption and E4 | `c21_duplicated_and_interior_signs` | [x] |
| C22 | `parse_number` | A5=**long run** (256–4096 accepted bytes) of random accepted characters — stress on the rewrite loop, the `memcpy`, and `strtod`'s partial stop | `c22_long_random_accepted_runs` | [x] |
| C23 | `parse_number` | A6=true with **many** `'.'` characters (rewrite loop iterates over most of the buffer) | `c23_many_decimal_points` | [x] |
| C24 | `parse_number` | A4=`depth` set to random values incl. `0`, `1`, `SIZE_MAX`; A12=`item` pre-filled with random garbage (incl. NaN bit patterns in `valuedouble`, `INT_MIN`/`INT_MAX`/random in `valueint`, random in `type`) on both success and failure paths | `c24_depth_and_item_garbage_roundtrip` | [x] |
| C25 | `parse_number` | **repeated calls on the same buffer** (streaming): parse a whitespace/comma-separated list of numbers, calling `parse_number` N times and advancing `offset` from the previous call's result — exercises the composed pipeline rather than one isolated call | `c25_streaming_repeated_calls` | [x] |
| C26 | `parse_number` | A2=`length` **shorter than the real allocation** (extra valid digits live past `length`) combined with interior `offset` — the classic off-by-one region | `c26_length_shorter_than_allocation` | [x] |
| C27 | `parse_number` | **fully random byte soup**: random `length` 0..64, random bytes from a biased alphabet (accepted chars over-represented), random `offset` in `0..=length+2`, random `depth`, random `item` garbage — 20 000 cases | `c27_random_byte_soup` | [x] |
| C28 | `parse_number` | **fully random ASCII soup** over the whole `0x00..=0xFF` range with random `offset`/`length`, 20 000 cases — catches value-dependent and out-of-range-index bugs the shaped rows above cannot | `c28_random_full_byte_range_soup` | [x] |
| C29 | `parse_number` | **locale axis** (`LC_NUMERIC`): `C`, `POSIX`, `C.utf8`, `de_DE.utf8`, `de_DE`, `fr_FR.utf8`, `ru_RU.utf8` × dot- and comma-spelled numbers. `strtod(3)` is locale-sensitive and the C's `decimal_point` variable exists because of it, so this is a genuine configuration axis even though the C hard-codes `'.'`. Each locale runs in a forked child (`setlocale` is process-global). | `h5_locale_dependent_strtod` | [x] |
| C30 | `parse_number` | **aliased out-parameters**: `item` and `input_buffer` overlapping in one arena at relative offsets 0/8/16/24/32. The C writes `item->{valuedouble,valueint,type}` and only then re-reads `input_buffer->offset`, so a translation that caches `offset` across the `item` writes diverges. Whole arena compared byte-for-byte. | `h1_aliased_item_and_buffer` | [x] |
| C31 | `parse_number` | `content` **non-NULL but unmapped** (`0x1`, `0xDEAD`, `SIZE_MAX`, `1<<47`, …) combined with `offset >= length` so the bound check guarantees a zero-length scan — the C computes `content + offset` and `memcpy(…, 0)` without dereferencing, so it must return `false` rather than fault. | `h2_bogus_nonnull_content_with_zero_length_scan` | [x] |
| C32 | `parse_number` | **misaligned** `cJSON` / `parse_buffer` pointers (byte skew 1..7), which a C caller inside a packed buffer can produce. | `h3_misaligned_struct_pointers` | [x] |
| C33 | `parse_number` | the **same `item` reused** across an interleaved success/failure sequence, verifying a failure never partially overwrites a previous success. | `h4_item_reuse_across_success_and_failure` | [x] |

## Mutation-sensitivity evidence

Passing tests only prove the suite *ran*; they do not prove it is *sensitive*.
`mutation_check.sh` injects 37 targeted bugs into the Rust translation and
requires the suite to fail on each. Result: **32 killed, 5 provably-equivalent
survivors, 0 blind spots.** The five survivors are semantically equivalent to
the original, each with a proof recorded next to it in the script:

1. `>= INT_MAX` → `> INT_MAX` — differs only at exactly `2147483647.0`, where the
   `else` branch computes `(int)2147483647.0 == INT_MAX` anyway.
2. `<= INT_MIN` → `< INT_MIN` — same argument at exactly `-2147483648.0`.
3. never setting `has_decimal_point` — the rewrite loop replaces `'.'` with
   `decimal_point`, which *is* `'.'`, so the loop is a no-op.
4. `wrapping_add` → `saturating_add` in the bound check — `offset + i` can never
   overflow while the loop runs (induction: reaching iteration `i` requires
   `offset + (i-1) < length <= SIZE_MAX`, hence `offset + i <= SIZE_MAX`).
5. writing `type` before `valueint` — two stores to distinct addresses with no
   intervening read of either, or of anything they could alias.
