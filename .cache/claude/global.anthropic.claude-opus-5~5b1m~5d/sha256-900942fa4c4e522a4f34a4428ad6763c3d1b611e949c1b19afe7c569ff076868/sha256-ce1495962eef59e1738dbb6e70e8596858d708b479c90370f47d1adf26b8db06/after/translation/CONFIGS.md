# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the
branches the C code actually takes, not from what looks important.

## Public entry points (complete)

`c_src/include/lib.h` declares exactly one function, and it is also the only
symbol in the C `.so` dynamic table, so the *full* public API surface is:

| entry point | signature | level |
|-------------|-----------|-------|
| `encode_base64` | `char *encode_base64(int size, const char *src)` | the only, and therefore also the lowest-level, entry point |

`encode` (`lib.c:6`) is `static` — not an entry point — but it is reached
*through* `encode_base64` and its five branches are an axis below. There are no
convenience wrappers, no init/teardown, no options struct, and no global state,
so "driving the library like a real consumer" is a single call whose result the
caller `free()`s.

## Axes the C code actually branches on

* **A1 — `size` mode** (`lib.c:37` `if (!size)`):
  * `size == 0` → length is measured with `strlen(src)` (**strlen mode**)
  * `size != 0` → the caller's `size` is used verbatim (**explicit mode**)
* **A2 — sign/overflow class of `size`**, because `n = size*4/3+4` is computed
  in `int` (`lib.c:41`) and then sign-extended into `calloc`'s `size_t`:
  positive-normal / negative-with-`n>0` / negative-with-`n<=0` /
  positive-with-`int`-overflow. (The classes where `calloc` fails are error
  rows E5/E6 in `ERRORS.md`.)
* **A3 — `len % 3`** (the padding branches `lib.c:69` `if (i+1 < size)` and
  `lib.c:75` `if (i+2 < size)`): `0` → `XXXX`, `1` → `XX==`, `2` → `XXX=`.
  Note the two inner *read* guards (`lib.c:53`, `lib.c:57`) and the two *emit*
  guards are separate branch pairs, so all four combinations matter.
* **A4 — byte values**, which select among `encode()`'s five branches:
  `u < 26` → `A-Z`, `u < 52` → `a-z`, `u < 62` → `0-9`, `u == 62` → `+`,
  `u >= 63` → `/`. Reaching sextet 62 and 63 requires specific high byte
  patterns (e.g. `0xFB..`, `0xFF..`), so byte content is a real axis.
* **A5 — signedness of `char`**: `src[i]` is a *signed* `char` on x86-64 and is
  assigned to `unsigned char b1` (`lib.c:51`), so bytes `0x80..0xFF` round-trip
  through a negative `char`. Content with the high bit set is its own axis.
* **A6 — embedded NUL bytes**: in explicit mode a `\0` inside the data is
  ordinary data; in strlen mode it terminates measurement. Same buffer, two
  different results — a real interaction.
* **A7 — length magnitude**: `0`, `1`, `2` (loop body runs once, partially),
  `3` (exactly one full group), `4`/`5` (two groups, second partial), `6`,
  many groups, and large buffers.

## Configuration table (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, a xorshift64* PRNG reimplemented identically in the test)
rather than one hand-picked value. Both `.so`s are called through `libloading`
and the **entire allocated buffer** (`n = size*4/3+4` bytes, i.e. the emitted
bytes *and* the `calloc` zero padding) is compared byte-for-byte, as is the
NULL-ness of the returned pointer.

| # | entry point(s) | configuration (options set + input shape) | test | status |
|---|----------------|--------------------------------------------|------|--------|
| C1 | `encode_base64` | explicit mode, `len % 3 == 0`, random bytes `0x00..0xFF`, lengths 3/6/9/…/96 | `c1_len_mod3_eq0` | [x] |
| C2 | `encode_base64` | explicit mode, `len % 3 == 1`, random bytes (drives `XX==` double padding), lengths 1/4/7/…/97 | `c2_len_mod3_eq1` | [x] |
| C3 | `encode_base64` | explicit mode, `len % 3 == 2`, random bytes (drives `XXX=` single padding), lengths 2/5/8/…/98 | `c3_len_mod3_eq2` | [x] |
| C4 | `encode_base64` | explicit mode, every length `1..=200`, random bytes — exhaustive length sweep over all three padding classes at once | `c4_length_sweep_1_to_200` | [x] |
| C5 | `encode_base64` | explicit mode, `size == 1` (minimum non-empty; both read guards false), all 256 possible single byte values | `c5_size_one_all_bytes` | [x] |
| C6 | `encode_base64` | explicit mode, `size == 2` (first read guard true, second false), all 65536 two-byte pairs | `c6_size_two_all_pairs` | [x] |
| C7 | `encode_base64` | explicit mode, `size == 3` (exactly one complete group, both guards true), randomized triples + all-zero + all-`0xFF` | `c7_size_three_full_group` | [x] |
| C8 | `encode_base64` | explicit mode, content = ASCII printable only (`0x20..0x7E`), random lengths — the "normal" text case, `encode()` stays mostly in `A-Z`/`a-z` | `c8_printable_ascii` | [x] |
| C9 | `encode_base64` | explicit mode, content = high-bit bytes only (`0x80..0xFF`) — A5, signed-`char`-to-`unsigned char` conversion | `c9_high_bit_bytes` | [x] |
| C10 | `encode_base64` | explicit mode, content = all zero bytes — sextets all 0, output all `A` | `c10_all_zero_bytes` | [x] |
| C11 | `encode_base64` | explicit mode, content = all `0xFF` — sextets all 63, drives `encode()`'s `return '/'` catch-all | `c11_all_ff_bytes` | [x] |
| C12 | `encode_base64` | explicit mode, content crafted so sextets hit **62** (`+`) and **63** (`/`) specifically, plus a byte pattern sweeping all 64 sextet values through every one of the 4 output positions | `c12_all_sextets_in_all_positions` | [x] |
| C13 | `encode_base64` | explicit mode, content contains **embedded NUL bytes** at random positions (A6) — NULs are data here | `c13_embedded_nuls_explicit_mode` | [x] |
| C14 | `encode_base64` | strlen mode (`size == 0`), NUL-terminated random ASCII payloads of random length (A1) | `c14_strlen_mode_random_ascii` | [x] |
| C15 | `encode_base64` | strlen mode (`size == 0`), NUL-terminated payload with high-bit bytes (A1 × A5) | `c15_strlen_mode_high_bit` | [x] |
| C16 | `encode_base64` | strlen mode (`size == 0`) on a buffer that *also* has bytes after the NUL (A1 × A6) — measurement must stop at the NUL and the trailing bytes must not leak in | `c16_strlen_mode_data_after_nul` | [x] |
| C17 | `encode_base64` | strlen mode where measured length `% 3` is forced to 0, 1 and 2 in turn (A1 × A3) | `c17_strlen_mode_each_padding_class` | [x] |
| C18 | `encode_base64` | explicit mode with `size` **smaller** than the real buffer length (truncating prefix encode), random `size` and random content | `c18_size_smaller_than_buffer` | [x] |
| C19 | `encode_base64` | explicit mode, large buffers (4 KiB, 64 KiB, 1 MiB) of random bytes — many loop iterations, `n` well past any small-buffer luck | `c19_large_buffers` | [x] |
| C20 | `encode_base64` | explicit mode, large buffer whose length is `% 3 == 1` and `% 3 == 2` (padding at a large offset) | `c20_large_buffers_with_padding` | [x] |
| C21 | `encode_base64` | negative `size` where `n > 0` (`-1`, `-2`, `-3`) — valid, well-defined: loop never runs, so the result is an all-zero `calloc` buffer of `n` bytes (A2) | `c21_small_negative_sizes` | [x] |
| C22 | `encode_base64` | negative `size` where `size*4` wraps to a **small positive** `int` (`-(2^30)+k`: `-1073741823`, `-1073741822`, `-1073741821`, `-1073741820`, `-1073741700`) → `n` = 5, 6, 8, 9, 169 zero bytes (A2, `int`-overflow interaction) | `c22_negative_size_wrapping_to_small_positive_n` | [x] |
| C23 | `encode_base64` | negative `size` = `INT_MIN` and `-(2^30)`, where `size*4` wraps to exactly `0` → `n == 4` (A2 boundary) | `c23_negative_size_wrapping_to_zero` | [x] |
| C24 | `encode_base64` | negative `size` whose wrap yields a **large** `n` (`-1072991824` → `n == 1000004`) — a ~1 MB all-zero allocation must match byte-for-byte (A2) | `c24_negative_size_wrapping_to_large_n` | [x] |
| C25 | `encode_base64` | randomized fuzz sweep: random `size` from the *whole* well-defined space (strlen mode, explicit-in-range, truncating, negative-with-`n>0`, negative-with-`n<=0`) × random content × random buffer length, 4000 iterations | `c25_randomized_fuzz_sweep` | [x] |

Rows whose `size` makes `calloc` fail on purpose are error rows and live in
`ERRORS.md` (E5, E6); rows C21–C24 are their *valid* counterparts, chosen so
that `n > 0` and the read loop provably never executes.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and `src/lib.rs`
contains **no `#[cfg(feature = ...)]`**, so the crate has exactly one feature
combination: the default (empty) one. Symmetrically, the C sources contain no
`#if`/`#ifdef`/`#ifndef`, so the C library has exactly one build configuration.
`check_features.sh` enumerates the combinations from `Cargo.toml` and runs the
full suite for each; it therefore runs the default combination, plus
`--no-default-features` and `--all-features` as equivalent aliases of it.

## Completion gate item

- [x] Phase B: EVERY row above passes across randomized inputs.
