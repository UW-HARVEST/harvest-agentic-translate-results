# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the axes `c_src/src/lib.c` actually branches on.

## Axes the C code distinguishes

**Public entry points** (the complete set — `lib.h` declares exactly one):

* `parse_number(cJSON *item, parse_buffer *input_buffer)` — lowest level and
  only entry point. There are no convenience wrappers; the two internal macros
  `can_access_at_index` / `buffer_at_offset` are driven through it.

**Runtime "options" / state the caller sets** (there is no option struct; the
configuration IS the `parse_buffer` state plus the pre-existing `cJSON` state):

| axis | values the C treats differently |
|---|---|
| `input_buffer->content` | non-NULL / NULL (lib.c:23) |
| `input_buffer->length` | 0, 1, exact-fit, longer than the numeric run, `SIZE_MAX` |
| `input_buffer->offset` | 0, mid-buffer (window starts inside the text), `== length`, `> length`, `SIZE_MAX` (wrap) |
| `input_buffer->depth` | never read — pass-through, must be preserved |
| `item` pre-state (`type`, `valueint`, `valuedouble`) | overwritten on success, preserved on failure |
| character class of each scanned byte | `0-9` / `+` / `-` / `e` / `E` (→ `number_string_length++`), `.` (→ also `has_decimal_point = true`), anything else (→ `goto loop_end`) |
| `has_decimal_point` | false → skip the replacement loop; true → run the `'.'` → `decimal_point` loop (lib.c:72-82) |
| `strtod` consumption | `after_end == number_c_string` (reject) vs `after_end > number_c_string` (accept, `offset += after_end - number_c_string`) |
| `strtod` partial consumption | `after_end` short of `number_string_length` (e.g. `"1e+"`, `"1."`, `"12e"`) vs full consumption |
| saturation branch (lib.c:95-106) | `number >= INT_MAX` / `number <= (double)INT_MIN` / in-range `(int)` truncation |

There are **no `#ifdef` build options** in `c_src` (the only preprocessor
conditionals are the unconditional `#ifdef true`/`#ifdef false` redefinition
guards in `lib.h`), and **no `[features]`** in `translation/Cargo.toml`, so
there is exactly one build configuration on each side.

The project builds **no binary executable** — `c_src/CMakeLists.txt` contains
only `add_library(driver SHARED src/lib.c)` — so there is no stdout comparison
to make.

## Configuration matrix

Every row is driven through the `.so` exports of BOTH libraries via
`libloading`, with many randomized inputs per row (fixed seed
`0x5EED_C0DE_1234_5678`, xorshift64* PRNG), and the full observable output is
compared byte-for-byte: return value, `item.type`, `item.valueint`,
`item.valuedouble` (compared as raw `u64` bits), and all four `parse_buffer`
fields.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `parse_number` | plain non-negative integers, `offset=0`, `length` = exact fit, no decimal point (`has_decimal_point=false` path) — randomized 1–18 digits | `cfg01_plain_integers` | [x] |
| 2 | `parse_number` | leading-`-` and leading-`+` integers, exact fit | `cfg02_signed_integers` | [x] |
| 3 | `parse_number` | decimals with `'.'` → exercises the `has_decimal_point` replacement loop (lib.c:72-82), randomized int/frac digit counts | `cfg03_decimals` | [x] |
| 4 | `parse_number` | scientific notation, all four exponent shapes: `e`/`E` × `+`/`-`/bare, randomized mantissa and 1–3 exponent digits | `cfg04_scientific` | [x] |
| 5 | `parse_number` | decimal + scientific combined (`-1.2345e-17`), full `[0-9.+-eE]` alphabet | `cfg05_decimal_scientific` | [x] |
| 6 | `parse_number` | numeric run followed by a terminator byte that hits `default: goto loop_end` — `,`, `]`, `}`, `"`, ` `, `\t`, `\n`, `:`, `\0`, random non-class byte — asserts `offset` advance stops at the run | `cfg06_trailing_delimiter` | [x] |
| 7 | `parse_number` | `offset > 0`: window starts mid-buffer, bytes before `offset` must be ignored (`buffer_at_offset`), randomized offset | `cfg07_nonzero_offset` | [x] |
| 8 | `parse_number` | `length` truncates the numeric run mid-number (`length < offset + run`), e.g. content `"12345"` with `length=3` → parses `"123"` only | `cfg08_length_truncates_run` | [x] |
| 9 | `parse_number` | no NUL terminator anywhere in `content`; numeric run reaches exactly `length` (the `'\0'`-independence the comment at lib.c:30 describes) | `cfg09_no_terminator` | [x] |
| 10 | `parse_number` | single-byte windows: `length - offset == 1`, every byte value `0..=255` as the sole byte (mix of accept `"0".."9"` and reject `"+ - . e E"` and all other bytes) | `cfg10_single_byte_all_256` | [x] |
| 11 | `parse_number` | `strtod` partially consumes the scanned prefix: `"1e"`, `"1e+"`, `"1."`, `"1.e"`, `"12E-"`, `"0x"`-like `[0-9eE]` soup → `after_end - number_c_string < number_string_length`, `offset` advances only by what strtod ate | `cfg11_partial_consumption` | [x] |
| 12 | `parse_number` | multiple signs / multiple dots / multiple exponents inside the scanned run: `"1.2.3"`, `"1e2e3"`, `"1-2"`, `"+1+2"`, `"--1"`, `"1..2"` — randomized | `cfg12_multi_sign_dot_exp` | [x] |
| 13 | `parse_number` | leading zeros and `"-0"` / `"-0.0"` / `"0e0"` — sign of zero must match in `valuedouble` bits (`-0.0` vs `+0.0`) | `cfg13_zeros_and_neg_zero` | [x] |
| 14 | `parse_number` | in-range `(int)` truncation branch (lib.c:105): values in `(INT_MIN, INT_MAX)` with fractional parts, both signs → truncate toward zero | `cfg14_int_truncation` | [x] |
| 15 | `parse_number` | saturation branches (lib.c:95 / lib.c:99): magnitudes at and beyond `±INT_MAX`/`INT_MIN`, up to `±inf` via `1e999` | `cfg15_saturation` | [x] |
| 16 | `parse_number` | extreme exponents: overflow → `±inf`, underflow → `±0.0`, subnormals (`1e-320`), `DBL_MAX`/`DBL_MIN` decimal strings | `cfg16_extreme_exponents` | [x] |
| 17 | `parse_number` | very long digit strings (30–400 digits, > 17 significant digits) → exercises `strtod` correct rounding and the large `malloc` size | `cfg17_long_digit_strings` | [x] |
| 18 | `parse_number` | `depth` pass-through: `depth` ∈ {0, 1, 42, `SIZE_MAX`} must be returned unchanged on both success and failure | `cfg18_depth_passthrough` | [x] |
| 19 | `parse_number` | `item` pre-populated with garbage (`type`, `valueint`, `valuedouble` incl. NaN/inf bit patterns) — success overwrites all three, failure preserves all three | `cfg19_item_prestate` | [x] |
| 20 | `parse_number` | repeated calls on the SAME buffer, advancing `offset` across a multi-number stream (`"1,2.5,-3e2,4"`) — the composed pipeline a real cJSON consumer drives | `cfg20_sequential_stream` | [x] |
| 21 | `parse_number` | fully random byte windows (uniform `0..=255`, random length 0–40, random offset) — the unbiased fuzz row hitting arbitrary accept/reject/partial mixes | `cfg21_random_bytes_fuzz` | [x] |
| 22 | `parse_number` | random windows over the numeric alphabet only (`0-9 + - . e E`, length 0–24) — dense coverage of the scanner × strtod interaction | `cfg22_random_numeric_alphabet` | [x] |
| 23 | `parse_number` | `content` window of length 0 (`offset == length`) at several `length` values, and `length == 0` — the empty-shape boundary | `cfg23_empty_window` | [x] |
| 24 | `parse_number` | randomized *cross-product* sweep: random alphabet ∈ {digits, numeric, all-bytes} × random `length` × random `offset` ∈ {0, mid, == length, > length} × random `depth` × random `item` pre-state | `cfg24_cross_product_sweep` | [x] |

## Additional exhaustive coverage (`tests/phase_d_heavy_fuzz.rs`)

Beyond the per-row tests above, these make the coverage EXHAUSTIVE for short
inputs, which is what actually pins down `strtod`'s `endptr` placement, the
`offset` advance and the saturation/truncation boundaries:

| test | coverage | inputs |
|---|---|---|
| `fuzz_exhaustive_all_bytes_len1_len2` | every 1- and 2-byte input over the full 256-value byte space | 65,792 |
| `fuzz_exhaustive_numeric_len3_plus_any_tail` | every 3-byte input over the accepted class, plus accepted²×any-byte | 60,975 |
| `fuzz_exhaustive_numeric_len4` | every 4-byte input over the accepted class (15⁴) | 50,625 |
| `fuzz_exhaustive_reduced_len5_len6` | 8⁵ over `019+-.eE` and 5⁶ over `1.e-9` | 48,393 |
| `fuzz_numeric_windows_bulk` | randomized class windows × length × offset × depth × item pre-state | 300,000 |
| `fuzz_full_bytespace_bulk` | randomized full-byte-space windows | 200,000 |
| `fuzz_roundtrip_f64_bitpatterns` | random finite `f64` bit patterns rendered 4 ways (`{:?}`, `{:e}`, `{:.0}`, `{:.17e}`) | 480,000 |
| `fuzz_saturation_neighbourhood` | dense clusters around ±2³¹, ±2³², 2⁵³ and powers of ten | 120,000 |
| `fuzz_long_inputs` | 200 B–4 KiB inputs; plus pathological all-`'.'`, all-digit, long-fraction, long-exponent buffers | ~4,000 |
| `fuzz_single_byte_insertions` | every accepted byte inserted at every position of 6 base numbers, × every window length | ~2,000 |

Total ≈ 1.3 M differential C-vs-Rust comparisons, each checking the return value,
`item.type`/`valueint`/`valuedouble` (raw bits) and all four `parse_buffer`
fields. Seeds are fixed, so runs are reproducible.

## Negative controls

The suite is proven to actually detect divergence — three injected bugs were
each caught, then reverted:

| injected bug | caught by |
|---|---|
| `cJSON_Number` 8 → 9 | ~all Phase B rows |
| `offset += number_string_length` instead of `after_end - number_c_string` | 7 Phase B/D rows, with full diagnostics |
| dropped `b'E'` from the scanner match arms | Phase B `cfg04_scientific` |

(Two other candidate mutations — `>=` → `>` on the `INT_MAX` branch and `<=` → `<`
on the `INT_MIN` branch — are genuinely semantics-preserving: at exactly
±2³¹ the `else` branch's `(int)` cast yields the same value as the saturated
constant, so no test can or should distinguish them.)
