# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## Axes, derived from the branches the C actually takes

`c_src/src/lib.c` has **no runtime options, no flags, no modes, no globals and
no `#ifdef`s** (the only macros are `TRUE`/`FALSE`). `c_src/CMakeLists.txt` has
no build options, and `Cargo.toml` has **no `[features]` section** — so there is
exactly one build configuration (`--no-default-features`, the empty feature
set). The whole configuration surface is therefore the *shape of the input
string*, which the code branches on as follows:

| axis | source of the branch | values the C distinguishes |
|------|----------------------|----------------------------|
| P — entry point | `include/lib.h` | `decode_base64` (only public symbol). Internal, reachable only through it: `is_base64` (`lib.c:29`, 6 accepting branches + reject), `decode` (`lib.c:10`, 5 branches) |
| S1 — filtered length `l` mod 4 | `k+1<l`, `k+2<l`, `k+3<l` (`lib.c:79,83,87`) — decides whether `c2`/`c3`/`c4` keep the default `'A'` | 0, 1, 2, 3 |
| S2 — group count `ceil(l/4)` | loop `for (k=0; k<l; k+=4)` (`lib.c:73`) | 0 (loop skipped), 1, 2, many |
| S3 — padding position | `if (c3 != '=')` (`lib.c:98`), `if (c4 != '=')` (`lib.c:102`) | none; `'='` at c4 only; `'='` at c3 (suppresses both writes); `'='` at c1/c2 (no suppression, `decode('=')==63`); `'='` mid-stream; all `'='` |
| S4 — alphabet class per position | `decode` (`lib.c:12,15,18,21,25`) | `A-Z` → 0..25, `a-z` → 26..51, `0-9` → 52..61, `'+'` → 62, `'/'`/`'='`/anything → 63 |
| S5 — non-alphabet bytes | `if (is_base64(src[k]))` (`lib.c:68`) | none; interleaved; *only* non-alphabet; low bytes `0x01..0x2A`; high bytes `0x80..0xFF` (negative `char`); whitespace / `\n` |
| S6 — input length | `int l = strlen(src)+1` (`lib.c:49`), buffer sizes `l+13` / `l` | 1, 2, 3, 4, 5, ≤200 (all mod-4 and capacity edges), 4 KiB, 64 KiB, 1 MiB |
| S7 — decoded payload bytes | `*p++` writes (`lib.c:96,99,103`) | payload with `0x00` bytes (buffer content past the C-string terminator), payload with `0xFF` bytes, full 0..255 range |
| S8 — source buffer vs. string | `strlen` / `src[k]` | interior NUL (bytes after the first NUL must be ignored) |

Every row below is compared **byte-for-byte over the whole `l + 13`-byte
destination buffer** (both implementations `calloc` the same size, so the
trailing zero fill is part of the comparison — this catches
off-by-one/short-write bugs that a `strcmp` would hide). All rows use many
randomized inputs from a fixed-seed PRNG (`seed = 0x2545F4914F6CDD1D`) unless
marked *exhaustive*.

## Rows

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `decode_base64` | S6=1 byte, S5=any: **exhaustive** all 255 single-byte inputs `0x01..0xFF` (covers every `is_base64` and `decode` branch in group position 0) | `b01_exhaustive_single_byte` | [x] |
| 2 | `decode_base64` | S6=2, alphabet only: **exhaustive** all 65×65 pairs of `[A-Za-z0-9+/=]` (S1=2, S3 at c1/c2) | `b02_exhaustive_alphabet_pairs` | [x] |
| 3 | `decode_base64` | S6=3, alphabet only: **exhaustive** all 65³ = 274 625 triples (S1=3, `'='` in every position, all `decode` classes × 3 positions) | `b03_exhaustive_alphabet_triples` | [x] |
| 4 | `decode_base64` | S6=4, alphabet only, randomized 4-char groups (S1=0, S2=1, `'='` at c3/c4 combinations) | `b04_random_alphabet_quads` | [x] |
| 5 | `decode_base64` | S1=0, S2=many, alphabet only, no `'='` (pure 4-aligned stream) | `b05_aligned_no_padding` | [x] |
| 6 | `decode_base64` | S1=1, S2=many, alphabet only (last group: c2/c3/c4 all default `'A'`) | `b06_len_mod4_eq_1` | [x] |
| 7 | `decode_base64` | S1=2, S2=many, alphabet only (last group: c3/c4 default `'A'`) | `b07_len_mod4_eq_2` | [x] |
| 8 | `decode_base64` | S1=3, S2=many, alphabet only (last group: c4 defaults to `'A'`) | `b08_len_mod4_eq_3` | [x] |
| 9 | `decode_base64` | canonical RFC-4648 encodings of random binary data, S3 = 0 / 1 / 2 trailing `'='`, S7 includes `0x00` and `0xFF` payload bytes | `b09_canonical_roundtrip` | [x] |
| 10 | `decode_base64` | S3=`'='` at c3 of a *middle* group (suppresses 2nd+3rd write mid-stream, decoding continues) | `b10_padding_mid_stream` | [x] |
| 11 | `decode_base64` | S3=all `'='` (`l` = 1..64 of `'='` only; `decode('=')==63`, c3/c4 suppression on every group) | `b11_all_equals` | [x] |
| 12 | `decode_base64` | S3=random `'='` sprinkled anywhere (incl. > 2, incl. position c1) | `b12_random_equals_anywhere` | [x] |
| 13 | `decode_base64` | S5=only non-alphabet bytes ⇒ S2=0 groups: returns non-NULL empty buffer | `b13_no_alphabet_chars_at_all` | [x] |
| 14 | `decode_base64` | S5=interleaved non-alphabet bytes (`0x01..0x2A`, `0x3A..0x40`, `0x5B..0x60`, `0x7B..0x7F`) with alphabet, all S1 values | `b14_interleaved_invalid_low_bytes` | [x] |
| 15 | `decode_base64` | S5=high bytes `0x80..0xFF` (negative `char` in `is_base64`/`decode`) interleaved with alphabet | `b15_interleaved_high_bytes` | [x] |
| 16 | `decode_base64` | S5=whitespace/newline wrapped, PEM-style 64-column base64 (`\r\n`, spaces, tabs) | `b16_pem_style_wrapping` | [x] |
| 17 | `decode_base64` | fully random bytes `0x01..0xFF`, lengths 1..64 — the general mixed path (S1,S3,S4,S5 all random) | `b17_random_bytes_small` | [x] |
| 18 | `decode_base64` | fully random bytes, lengths sweeping 1..=200 (every S1 and every destination-capacity relationship `3*ceil(l/4)` vs `l+13`) | `b18_length_sweep_1_to_200` | [x] |
| 19 | `decode_base64` | alphabet-only, length sweep 1..=200 (worst case: no byte is filtered, output = 3·⌈l/4⌉ against a `l+14` buffer) | `b19_alphabet_length_sweep` | [x] |
| 20 | `decode_base64` | S6=4 KiB / 64 KiB / 1 MiB, alphabet-only and mixed (long-loop behaviour, `k` sequencing) | `b20_large_inputs` | [x] |
| 21 | `decode_base64` | S4 coverage matrix: every alphabet char in every group position `k%4 ∈ {0,1,2,3}` with the other three positions randomized | `b21_alphabet_position_matrix` | [x] |
| 22 | `decode_base64` | `'+'` and `'/'` heavy inputs (`decode` → 62 / 63, high-bit-pattern outputs) | `b22_plus_slash_heavy` | [x] |
| 23 | `decode_base64` | S8=interior NUL: buffer holds trailing garbage after a NUL; result must equal the truncated string's result | `b23_interior_nul_buffer` | [x] |
| 24 | `decode_base64` | S7: payload chosen so the decoded buffer contains NUL bytes at the start/middle/end (verifies the compare past the C-string terminator, i.e. the `calloc` zero fill and the exact write count) | `b24_nul_bytes_in_payload` | [x] |
| 25 | `decode_base64` | repeated calls in sequence (no shared/global state between calls; interleaved C/Rust invocations) | `b25_no_cross_call_state` | [x] |

All rows live in `tests/valid_paths.rs` and all of them pass (verified in both
the `dev` and `release` profiles, and against the C compiled at `-O0`, `-O1`,
`-O2` and `-O3`).

Row 26 (`b26_known_answers`) is an extra *absolute* check: it pins the C's actual
output for a handful of well-known vectors, so a regression that happened to
affect both implementations identically would still be caught.

## Build/feature configuration matrix

`Cargo.toml` has **no `[features]` table**, so the cross-product of features is a
single element: the empty set. `./verify.sh` enumerates it mechanically from
`Cargo.toml` (rather than hard-coding it) and runs `cargo check --all-targets`,
`cargo build` and the whole test suite for each element it finds.

| # | cargo features | C build config | status |
|---|----------------|----------------|--------|
| 1 | `--no-default-features` (empty set — the only valid combination) | `add_library(driver SHARED src/lib.c)`, no CMake options | [x] `cargo check --all-targets` clean, Phases B+C+D all pass |

Additional configurations exercised beyond the required matrix (all pass):

| axis | values covered |
|------|----------------|
| Rust profile | `dev` (unoptimized, debug assertions + integer-overflow checks on) and `release` (`opt-level=3`) |
| C optimization | `-O0` (the CMake default), `-O1`, `-O2`, `-O3` — signed-overflow UB in `l + 13` is the one place gcc could legally diverge, so every level was compared (`C_DRIVER_SO=<path> cargo test`) |
