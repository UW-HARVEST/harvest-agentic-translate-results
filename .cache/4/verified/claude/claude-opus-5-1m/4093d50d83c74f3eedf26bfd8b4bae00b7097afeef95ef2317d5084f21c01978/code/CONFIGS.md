# CONFIGS.md — Phase A configuration-surface table

## Mechanical derivation of the axes

`c_src/src/main.c` is the whole library. The axes it actually branches on:

* **Public entry points** (all three exported symbols, lowest level first):
  * `int foo(const char *in, char c)` — the primitive: counts `c` in a C string;
  * `void driver(const char *in)` — calls `foo` twice (`'A'`, then `'x'`) and `printf`s both;
  * `int main()` — reads ≤ 1000 bytes from stdin into a zero-filled `char in[1000]`, then `driver`.
* **Runtime options/flags:** none. There is no option struct, no mode, no environment
  variable, no `#ifdef` (`grep -rn '#ifdef\|#ifndef\|#if ' c_src/src/` → no matches) and no
  CMake `option()`. The only "mode" selector in the API is `foo`'s second parameter `c`, i.e.
  *which byte* is being counted — `driver` hard-codes the two values `'A'` and `'x'`.
* **Input shapes the code distinguishes:**
  * the needle byte: `'A'`, `'x'`, some other ASCII value, a value with the high bit set
    (negative `char`), and the special value `0` (→ `ERRORS.md` row 3);
  * occurrence count: none / one / two / many / every byte;
  * occurrence position: first byte, last byte (the `s++` step lands exactly on the NUL),
    consecutive runs, alternating;
  * string length: 0, 1, 2, 3, and the `sizeof(in)` boundaries 998 / 999 / 1000 / 1001 / 4096;
  * byte alphabet: ASCII only vs. the full 1…255 range (and NUL for `main`, which terminates
    the string early);
  * for `main` only: total stdin length vs. the 1000-byte buffer, position of the first NUL,
    and the stdin *kind* (empty, `/dev/null`, regular file, pipe delivering the data in
    several chunks so `fread`'s refill loop runs more than once).
* **Build-time configurations:** `Cargo.toml` has no `[features]`, so the cross-product of
  feature combinations is the single empty set; both `cargo test` and
  `cargo test --no-default-features` are run, in the `dev` **and** `release` profiles
  (`run_all_checks.sh`).

## Table (cross-product, pruned to combinations the C treats differently)

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `foo` via `.so` | needle `'A'` (the value `driver` uses), randomized dense strings over `{A,x}` | `b01_foo_ascii_needles_random` | [x] |
| 2 | `foo` via `.so` | needle `'x'`, randomized strings over `{A,a,x,X}` (case-sensitivity) | `b01_foo_ascii_needles_random` | [x] |
| 3 | `foo` via `.so` | arbitrary printable-ASCII needle, wide-alphabet strings (sparse hits) | `b01_foo_ascii_needles_random` | [x] |
| 4 | `foo` via `.so` | full byte alphabet 1…255, needle drawn *from the string* (guaranteed present) | `b02_foo_full_byte_alphabet_random`, `b02b_foo_all_needles_exhaustive` | [x] |
| 5 | `foo` via `.so` | full byte alphabet 1…255, random needle (usually absent) | `b02_foo_full_byte_alphabet_random`, `b02b_foo_all_needles_exhaustive` | [x] |
| 6 | `foo` via `.so` | needle with the high bit set (`0x80`, `0xff` → negative `char`) on binary data; **exhaustive** over all 255 legal needles | `b02_foo_full_byte_alphabet_random`, `b02b_foo_all_needles_exhaustive`, `b03_foo_boundary_shapes` | [x] |
| 7 | `foo` via `.so` | empty string (`""`) with every needle | `b03_foo_boundary_shapes`, `c12_empty_string_inputs` | [x] |
| 8 | `foo` via `.so` | length 1 / 2 / 3, hit and no-hit variants | `b03_foo_boundary_shapes` | [x] |
| 9 | `foo` via `.so` | single hit at the *first* byte / at the *last* byte (`s++` lands on the NUL) | `b03_foo_boundary_shapes` | [x] |
| 10 | `foo` via `.so` | every byte is a hit (runs of 255 / 256 / 511 / 512 / 4096) | `b03_foo_boundary_shapes` | [x] |
| 11 | `foo` via `.so` | lengths at `main`'s buffer boundaries: 998, 999, 1000, 1001, 4095, 4096, and a 100 kB string (~50 000 hits) | `b03_foo_boundary_shapes`, `b02b_foo_all_needles_exhaustive` | [x] |
| 12 | `driver` via `.so` | randomized inputs (2000 of them), byte-exact stdout comparison, batched in a fresh child process | `b04_driver_random` | [x] |
| 13 | `driver` via `.so` | same, but called in-process through `dlsym` with fd 1 redirected (live-process path) | `b04b_driver_random_in_process` | [x] |
| 14 | `driver` via `.so` | only `'A'` / only `'x'` / both / neither / wrong case (`'a'`, `'X'`) | `b05_driver_shapes` | [x] |
| 15 | `driver` via `.so` | non-ASCII + whitespace payloads, and lengths 1 / 999 / 1000 / 1001 / 2000 | `b05_driver_shapes` | [x] |
| 16 | `main` via `.so` | stdin empty (length 0) | `b06_main_via_so_lengths`, `b10_main_stdin_sources` | [x] |
| 17 | `main` via `.so` | stdin length 1 / 2 / 3 / 17 / 500 / 997 / 998 / 999 (buffer NUL-terminated) | `b06_main_via_so_lengths` | [x] |
| 18 | `main` via `.so` | stdin longer than the buffer (1000 / 1001 / 2000 / 5000) **with** a NUL inside the first 1000 bytes | `b06_main_via_so_lengths` | [x] |
| 19 | `main` via `.so` | embedded NUL at offset 0 / middle / 999, and two NULs | `b07_main_via_so_embedded_nul` | [x] |
| 20 | `main` via the executables | randomized binary inputs (bytes 0…255, incl. NUL) and text inputs, lengths 0…1500 | `b08_executables_random` | [x] |
| 20b | `main` via the executables | six distinct input-shape families: tiny, buffer-boundary (990…1010), uniform-random binary, exact boundary lengths (0/1/999/1000/1001/4096), NUL-sprinkled text, oversized (1400…3000) | `b08b_executables_shape_fuzz` | [x] |
| 21 | `main` via the executables | stdin delivered as 5 separate pipe writes with pauses (forces `fread` to loop) | `b09_executables_chunked_pipe_stdin` | [x] |
| 22 | `main` via `.so` + executables | stdin = `/dev/null` | `b10_main_stdin_sources` | [x] |
| 23 | `main` via `.so` | stdin = regular file (seekable, exact size) | `b10_main_stdin_sources` | [x] |
| 24 | `foo` via `.so` | needle passed as `int` across the FFI boundary (ABI-level `char`/`int` mismatch) over 6 string shapes × 17 out-of-range values | `c04_foo_needle_out_of_char_range` | [x] |
| 25 | all three symbols | resolvable through `dlsym` in both objects and callable through the C ABI | `d02_all_symbols_callable_via_dlsym` | [x] |
| 26 | whole crate | build-time combination: default features / `--no-default-features`, `dev` and `release` profiles (the complete set — no `[features]` exist) | `run_all_checks.sh` | [x] |

Randomization: every row marked "randomized" uses the xorshift64\* PRNG in
`tests/differential.rs` with a hard-coded seed per test, so runs are reproducible
(4000 + 4000 + 2000 + 240 + 150 + 40 randomized cases in total, plus the enumerated shapes and the
exhaustive 255-needle sweep over 11 strings = 2805 further comparisons).

## Deliberately excluded (undefined behavior, see ERRORS.md)

* `main` with ≥ 1000 bytes and no NUL among them (`in` unterminated) — row 10 of `ERRORS.md`;
  tested with the weaker invariant that the C can only see *more* occurrences than the buffer
  holds while the Rust deterministically reports exactly the buffer contents.
* `foo`/`driver` with a non-terminated buffer or `c == 0` — rows 3 and 10 of `ERRORS.md`.
