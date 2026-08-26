# CONFIGS.md — Configuration-surface table (Phase B gate)

The public API is a single entry point (`c_src/include/lib.h`):

```c
char *encode_base64(int size, const char *src);
```

There are no runtime option structs, no init/context objects, no `#ifdef`s and
no build-time knobs (`Cargo.toml` has **no `[features]` section**;
`c_src/CMakeLists.txt` defines no `option()`/`target_compile_definitions`).
Therefore the configuration surface is the cross-product of the axes the C code
actually branches on, which were extracted mechanically from `c_src/src/lib.c`:

## Axes derived from the C source

| axis | values | source line(s) |
|------|--------|----------------|
| A. `src` pointer | NULL / valid | `if (!src)` L33 |
| B. `size` sign & zero-ness | `0` (⇒ `strlen` path) / `> 0` / `< 0` (loop skipped) | `if (!size) size = strlen(...)` L37-39; `for (i = 0; i < size; ...)` L48 |
| C. `size % 3` | `0` / `1` / `2` ⇒ selects the padding branches | `if (i+1 < size)` L69, `if (i+2 < size)` L75 |
| D. tail-`=` count | 0 / 1 / 2 `'='` bytes | L72, L78 |
| E. `encode()` output class of each 6-bit group | `u<26`→`A-Z`, `u<52`→`a-z`, `u<62`→`0-9`, `u==62`→`+`, `u==63`→`/` | L8-21 |
| F. byte values in `src` | `0x00`, `0x01..0x7F`, `0x80..0xFF` (high bit ⇒ *signed* `char` → `unsigned char` conversion), embedded NULs | `b1 = src[i]` L51 (signed `char` ⇒ `unsigned char`) |
| G. `size` vs `strlen(src)` | `size < strlen` (truncated) / `size == strlen` / `size > strlen` (reads past the NUL) | L38 only assigns when `size==0`; the loop never re-checks |
| H. allocation-size expression sign | `size*4/3+4 >= 0` ⇒ `calloc` succeeds / `< 0` ⇒ sign-extends to huge `size_t` ⇒ `calloc` fails | `calloc(sizeof(char), size * 4 / 3 + 4)` L41, `if (!out)` L42 |
| I. `int` wrap-around in `size*4` | no wrap / wraps (`|size| >= 2^29`) | L41 (`int` arithmetic) |

## Configuration rows (each must pass with MANY randomized inputs)

| # | entry point(s) | configuration (options set + input shape) | done |
|---|----------------|-------------------------------------------|-----|
| 1 | `encode_base64` | `size > 0`, `size % 3 == 0`, random bytes over full `0x00..0xFF` — 0 `'='` padding, exercises E across all 5 classes | [x] |
| 2 | `encode_base64` | `size > 0`, `size % 3 == 1`, random bytes — 2 `'='` padding (b6 and b7 both suppressed) | [x] |
| 3 | `encode_base64` | `size > 0`, `size % 3 == 2`, random bytes — 1 `'='` padding (b7 suppressed) | [x] |
| 4 | `encode_base64` | `size == 1`, `2`, `3` (smallest non-empty; buffer-size expression `1*4/3+4=5`, `6`, `8`) | [x] |
| 5 | `encode_base64` | `size > 0`, `src` bytes all `0x00` ⇒ every 6-bit group `0` ⇒ all `'A'` | [x] |
| 6 | `encode_base64` | `size > 0`, `src` bytes all `0xFF` ⇒ groups `63` ⇒ all `'/'` (signed-`char` → `unsigned char` path, axis F) | [x] |
| 7 | `encode_base64` | `size > 0`, `src` bytes chosen so 6-bit groups hit exactly `u==61,62,63` ⇒ `'9'`, `'+'`, `'/'` boundary of `encode()` | [x] |
| 8 | `encode_base64` | `size > 0`, `src` bytes all high-bit set (`0x80..0xFF`) ⇒ negative `char` ⇒ mod-256 conversion | [x] |
| 9 | `encode_base64` | `size > 0`, `src` contains embedded `0x00` bytes at random positions (proves `size` — not NUL — terminates the loop) | [x] |
| 10 | `encode_base64` | `size == 0`, `src` = NUL-terminated non-empty C string ⇒ `strlen` path (axis B) | [x] |
| 11 | `encode_base64` | `size == 0`, `src = ""` (empty string) ⇒ `strlen==0` ⇒ `size` stays `0` ⇒ empty result, loop never runs | [x] |
| 12 | `encode_base64` | `size == 0`, `src` = string whose *first* byte is NUL but with trailing garbage after it ⇒ `strlen==0`, garbage must be ignored | [x] |
| 13 | `encode_base64` | `size > 0` and `size < strlen(src)` ⇒ truncated encode (axis G) | [x] |
| 14 | `encode_base64` | `size > 0` and `size > strlen(src)` ⇒ reads past the NUL into the rest of the buffer (axis G) | [x] |
| 15 | `encode_base64` | `size > 0`, large buffer (`size` in `[1000, 4096]`, random) ⇒ many loop iterations, all `size%3` residues | [x] |
| 16 | `encode_base64` | `size` sweep `1..=256` exhaustively with random data ⇒ every `size%3` residue and every allocation size in range | [x] |
| 17 | `encode_base64` | `size < 0`, `size in {-1,-2,-3}` ⇒ `size*4/3+4` in `{3,2,0}` ⇒ `calloc` SUCCEEDS, loop skipped ⇒ non-NULL empty string (incl. `calloc(1,0)`) | [x] |
| 18 | `encode_base64` | `size == i32::MIN` ⇒ `size*4` wraps to `0` ⇒ `nbytes==4` ⇒ non-NULL empty string, loop skipped (axis I) | [x] |
| 19 | `encode_base64` | `size` near `i32::MIN` (`-2147483647`, `-2147483646`, ...) ⇒ `size*4` wraps positive ⇒ `calloc` succeeds, loop skipped (axis I) | [x] |
| 20 | `encode_base64` | `size < 0` randomized over the whole negative `i32` range ⇒ mixes the NULL / non-NULL outcomes of axis H+I | [x] |
| 21 | `encode_base64` | returned buffer inspected over its **full** `size*4/3+4` byte extent (not just up to the first NUL) ⇒ verifies the `calloc` zero-fill tail and the exact padding bytes match | [x] |

### Deliberately excluded (undefined behaviour in the C — not a valid input)

| configuration | why excluded |
|---------------|--------------|
| `size >= 2^29` (e.g. `INT_MAX`) with a valid `src` | `size*4` wraps to a small/negative `int`, so `calloc` returns a tiny buffer while the loop still iterates `size` times ⇒ the C itself writes far out of bounds and segfaults. Both C and Rust reproduce the same wrapped `nbytes` (verified by inspection of the identical `wrapping_mul(4).wrapping_div(3).wrapping_add(4)` expression), but the call cannot be executed in-process without crashing the test harness. |
| `size > 0` with a `src` buffer shorter than `size` | the C reads `src[0..size)` unconditionally ⇒ out-of-bounds read. All rows above always supply at least `size` readable bytes. |

## Row → test mapping (all rows verified)

Every row is exercised by a differential test that loads **both** `.so` files
with `libloading` and compares NULL-ness, the whole `size*4/3+4` allocation
extent and the resulting C string.

| CONFIGS row | test in `tests/differential.rs` | randomized inputs per run |
|-------------|---------------------------------|---------------------------|
| 1  | `cfg01_size_mod3_0_random_bytes`            | 2 000 random sizes × random bytes |
| 2  | `cfg02_size_mod3_1_random_bytes`            | 2 000 |
| 3  | `cfg03_size_mod3_2_random_bytes`            | 2 000 |
| 4  | `cfg04_tiny_sizes_exhaustive`               | 256 + 65 536 exhaustive + 20 000 random |
| 5  | `cfg05_all_zero_bytes`                      | 200 sizes |
| 6  | `cfg06_all_ff_bytes`                        | 200 sizes |
| 7  | `cfg07_encode_alphabet_exhaustive`          | all 64 alphabet values + 16 boundary cases |
| 8  | `cfg08_high_bit_bytes_only`                 | 1 500 |
| 9  | `cfg09_embedded_nul_bytes`                  | 1 500 |
| 10 | `cfg10_size_zero_uses_strlen`               | 2 000 (× 2 calls) |
| 11 | `cfg11_size_zero_empty_string`              | 2 |
| 12 | `cfg12_size_zero_leading_nul_then_garbage`  | 500 |
| 13 | `cfg13_size_less_than_strlen`               | 2 000 |
| 14 | `cfg14_size_greater_than_strlen`            | 2 000 |
| 15 | `cfg15_large_buffers`                       | 120 random + 3 fixed large (up to 256 KiB) |
| 16 | `cfg16_size_sweep_1_to_256`                 | 256 sizes × 12 payloads = 3 072 |
| 17 | `cfg17_small_negative_sizes_succeed`        | 3 |
| 18 | `cfg18_size_int_min`                        | 1 |
| 19 | `cfg19_sizes_near_int_min`                  | 65 |
| 20 | `cfg20_random_negative_sizes`               | 4 000 sampled over the whole negative i32 range |
| 21 | `cfg21_full_allocation_extent_matches`      | 120 |
| extra | `wrap_positive_nbytes_from_huge_negative_sizes` | ~215 sizes where `size*4` wraps positive |
| extra | `boundary_size_domain_sweep_with_valid_src`     | every size around each i32 boundary |
| extra | `boundary_null_vs_nonnull_agreement_table`      | 12 boundary sizes + 5 accepted sizes |

All 21 rows pass, under the default configuration, `--no-default-features`,
`--all-features` and `--release`, and against the C library compiled at `-O0`
(default), `-O2` and `-O3`.
