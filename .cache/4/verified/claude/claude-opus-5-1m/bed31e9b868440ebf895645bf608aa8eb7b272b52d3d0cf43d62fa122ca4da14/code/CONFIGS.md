# CONFIGS.md — configuration-surface table (Phase A / Phase B)

Mechanically derived from the C source + header. The library has **one** public
entry point, so the configuration surface is the cross-product of its
runtime-option arguments and the input shapes the C code branches on.

## Public entry points (`c_src/include/lib.h`)

| entry point | signature |
|---|---|
| `hex2bin` | `int hex2bin(uint8_t *bin, size_t bin_maxlen, const char *hex, size_t hex_len, const char *ignore, const char **hex_end_p)` |

There is no init/context/one-shot split — this *is* the lowest-level entry
point, and there are no convenience wrappers to shadow it.

## Axes the C actually branches on

| axis | values the C distinguishes | source line |
|---|---|---|
| `ignore` pointer | `NULL` / non-`NULL` (`ignore != NULL`) | 23 |
| ignore set contents | matching char / non-matching char / `""` (matches only NUL) / set containing hex digits (never consulted for them) / set containing high-bit bytes | 24 (`strchr`) |
| `state` (parser phase) | `0` (byte-aligned) / `0xFF` (mid-byte) — gates ignore-skipping *and* the final odd-digit error | 23, 35, 40, 43 |
| `hex_end_p` | `NULL` (strict: unconsumed input becomes an error) / non-`NULL` (lenient: reports stop position) | 50, 52 |
| `bin_maxlen` | `0` / `< hex_len/2` / `== hex_len/2` (exact) / `> hex_len/2` (slack) / huge (`usize::MAX`) | 31 |
| `hex_len` | `0` / `1` / `2` / even / odd / large (4 KiB) / shorter than the backing buffer (partial view) | 16 |
| char class of `hex[i]` | `'0'..'9'` (via `c_num0`) / `'A'..'F'` (via `c_alpha0`, bit 5 cleared) / `'a'..'f'` / everything else (rejected) | 18–22, 30 |
| char case | lowercase / uppercase / mixed within one byte (`"aB"`) | 20 (`c & ~32U`) |
| boundary bytes | `0x00`, `0x2F`, `0x3A`, `0x40`, `0x47`, `0x60`, `0x67`, `0x7F`, `0x80`, `0xFF` | 18–22 |
| separator placement | none / leading / between bytes (`state==0`) / mid-byte (`state!=0`) / trailing / runs of several | 23–27 |
| `bin`/`hex` aliasing | disjoint buffers / in-place (`bin == (uint8_t*)hex`) — legal because writes trail reads | 38 |

## Configuration rows

Each row is exercised with **many randomized inputs** (fixed seed, see
`tests/common/mod.rs::Rng`), comparing C and Rust `.so` exports on: return
value, the **entire** `bin` buffer (including slack bytes past `bin_maxlen`),
the `*hex_end_p` offset, and the `hex` buffer contents afterwards.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `hex2bin` | `ignore=NULL`, `hex_end_p=NULL`, even random digit stream, `bin_maxlen` exact → full consume, success | `cfg_01_strict_exact` | [x] |
| 2 | `hex2bin` | `ignore=NULL`, `hex_end_p=Some`, even random digits, `bin_maxlen` exact | `cfg_02_lenient_exact` | [x] |
| 3 | `hex2bin` | `ignore=NULL`, `hex_end_p=Some`, even digits, `bin_maxlen` with slack (`+1..+16`) | `cfg_03_slack_bin` | [x] |
| 4 | `hex2bin` | `ignore=NULL`, `hex_end_p=Some`, even digits, `bin_maxlen` short (`0..needed-1`) — partial write then `-1` | `cfg_04_short_bin` | [x] |
| 5 | `hex2bin` | `hex_len == 0` × {`bin_maxlen` 0 / >0} × {`hex_end_p` NULL / Some} × {`ignore` NULL / set} | `cfg_05_empty_input` | [x] |
| 6 | `hex2bin` | `hex_len == 1` (single digit) and `hex_len == 2` (single byte), both cases and all 16 digit values | `cfg_06_one_and_two_digits` | [x] |
| 7 | `hex2bin` | odd digit count (3,5,…,33) — `state != 0` end path, both `hex_end_p` variants | `cfg_07_odd_lengths` | [x] |
| 8 | `hex2bin` | all-lowercase digit stream (`0-9a-f`) | `cfg_08_lowercase_only` | [x] |
| 9 | `hex2bin` | all-uppercase digit stream (`0-9A-F`) | `cfg_09_uppercase_only` | [x] |
| 10 | `hex2bin` | mixed case inside single bytes (`"aB"`, `"Fc"`, …) — exercises `c & ~32U` on both nibbles | `cfg_10_mixed_case` | [x] |
| 11 | `hex2bin` | digits-only stream (`'0'..'9'`, i.e. only the `c_num0` path) and letters-only stream (only the `c_alpha0` path) | `cfg_11_single_class_streams` | [x] |
| 12 | `hex2bin` | `ignore=""` (empty set) with a random stream containing non-hex bytes | `cfg_12_empty_ignore` | [x] |
| 13 | `hex2bin` | `ignore=":"`, single separator between every byte (`state==0` positions only) → full decode | `cfg_13_colon_separated` | [x] |
| 14 | `hex2bin` | `ignore=" \t\r\n:-"`, random runs (1..4) of random separators at byte-aligned positions | `cfg_14_multi_ignore_runs` | [x] |
| 15 | `hex2bin` | ignore set + **leading** separators (before any digit) | `cfg_15_leading_separators` | [x] |
| 16 | `hex2bin` | ignore set + **trailing** separators (consumed, `hex_pos == hex_len`, so success even with `hex_end_p=NULL`) | `cfg_16_trailing_separators` | [x] |
| 17 | `hex2bin` | ignore set + separator at a **mid-byte** (`state!=0`) position → stop + odd-digit error | `cfg_17_midbyte_separator` | [x] |
| 18 | `hex2bin` | ignore set that also contains hex digits (`"0aA:"`) — must not change digit handling | `cfg_18_ignore_contains_hex_digits` | [x] |
| 19 | `hex2bin` | ignore set of high-bit bytes (`"\x80\xff"`) with high-bit bytes in the input | `cfg_19_high_bit_ignore` | [x] |
| 20 | `hex2bin` | every one of the 256 byte values as the char at a random position, × `ignore` ∈ {NULL, `""`, that byte, other byte} × `hex_end_p` ∈ {NULL, Some} | `cfg_20_all_bytes_matrix` | [x] |
| 21 | `hex2bin` | boundary bytes `0x00 0x2F 0x3A 0x40 0x47 0x60 0x67 0x7F 0x80 0xFF` embedded at every position of a short stream | `cfg_21_boundary_bytes` | [x] |
| 22 | `hex2bin` | embedded NUL byte(s) + non-NULL `ignore` (the `strchr` terminator quirk), at aligned and mid-byte positions | `cfg_22_embedded_nul_quirk` | [x] |
| 23 | `hex2bin` | `hex_len` shorter than the backing buffer (partial view; bytes past `hex_len` must not be read/reported) | `cfg_23_partial_view` | [x] |
| 24 | `hex2bin` | in-place decode: `bin == (uint8_t*)hex`, with and without ignore chars | `cfg_24_in_place` | [x] |
| 25 | `hex2bin` | large input: 4096–8192 digits, random separators, random `bin_maxlen` | `cfg_25_large_input` | [x] |
| 26 | `hex2bin` | `bin_maxlen = usize::MAX` and `usize::MAX/2` with a small stream | `cfg_26_huge_bin_maxlen` | [x] |
| 27 | `hex2bin` | `ignore` = 1-byte set whose byte is `0x00`-terminated only (`"\x01"`), stream contains `0x01` | `cfg_27_control_char_ignore` | [x] |
| 28 | `hex2bin` | round-trip property: for uniformly random `bin` bytes rendered as hex (upper and lower), both impls return the original bytes | `cfg_28_round_trip` | [x] |
| 29 | `hex2bin` | full-fuzz cross-product: random `hex` bytes (any of 256), random `hex_len`, random `bin_maxlen` (0..len+2, plus huge), random `ignore` (NULL / random set), random `hex_end_p` presence — 20 000 cases | `cfg_29_fuzz_all_axes` | [x] |
| 30 | `hex2bin` | fuzz restricted to *mostly valid* streams (90 % hex digits, 10 % separators from the ignore set) so deep decode paths dominate — 20 000 cases | `cfg_30_fuzz_mostly_valid` | [x] |
| 31 | `hex2bin` | **exhaustive**: every input of length 0, 1 and 2 (all 65 536 byte pairs) × {`ignore` NULL/`""`/`":"`/`" \t\r\n:-"`} × {`hex_end_p` NULL/Some} × {`bin_maxlen` 0/1/2/`usize::MAX`} | `cfg_31_exhaustive_len_0_1_2` | [x] |
| 32 | `hex2bin` | **exhaustive** over an 18-byte representative alphabet (range ends, the bytes just outside them, NUL, 0x80, 0xFF, separators) for lengths 3 and 4 (18³ + 18⁴ inputs) × option matrix | `cfg_32_exhaustive_repr_len_3_4` | [x] |
| 33 | `hex2bin` | **exhaustive**: all 16 777 216 three-byte inputs × 2 option settings (33.5 M differential comparisons) | `cfg_33_exhaustive_all_triples` | [x] |

## How the rows are exercised

`tests/common/mod.rs` loads **both** shared objects with `libloading` and calls
only the exported `hex2bin` symbol (so the Rust `#[no_mangle]` wrapper is under
test too). For every case it compares:

* the `int` return value,
* the **entire** `bin` allocation, including slack bytes past `bin_maxlen`
  (catches over-writes and rolled-back/partial writes),
* the `*hex_end_p` offset relative to `hex` (and that the pointer is *not*
  written when `hex_end_p == NULL`),
* the `hex` buffer afterwards (catches stray writes, and validates the in-place
  configuration of row 24).

Test files: `tests/valid_paths.rs` (rows 1–30), `tests/exhaustive.rs`
(rows 31–33), `tests/error_paths.rs` (`ERRORS.md`), `tests/harness.rs`
(self-checks + symbol parity).

## Build caveat discovered while verifying

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library target, so
a bare `cargo test` can silently load a **stale** `.so` and report a false PASS.
Two safeguards are in place:

1. `scripts/run_diff_tests.sh` runs `cargo build` before `cargo test` for every
   feature combination.
2. The harness itself refuses to run when a `.so` is older than its sources
   (`STALE SHARED OBJECT` panic), so the failure mode cannot recur silently.
