# CONFIGS.md — configuration / valid-input surface

## Build-time configurations

| axis | values | source |
|------|--------|--------|
| Cargo features | **none** — `[features]` is empty; the only valid combination is the empty set (`--no-default-features`) | `Cargo.toml` |
| CMake options | **none** — `CMakeLists.txt` has no `option()`, no `target_compile_definitions`, no build-type branches | `c_src/CMakeLists.txt` |
| C preprocessor | **none** — `grep -nE '#if|#ifdef|#define' c_src/src/main.c` → no matches | `c_src/src/main.c` |

So there is exactly **one** build configuration to verify:
`cargo check/test --no-default-features` (identical to the default build).

## Runtime axes the C code actually branches on

There are no flags, modes, env vars or CLI arguments (`main()` takes no `argc`/
`argv`, no `getenv`). The behaviour is a pure function of

* **entry point**: `driver(const char*, const char*)` — the low-level FFI entry —
  and `main(void)` — the whole-program entry that reads stdin;
* **input shape**, i.e. the data-dependent branches inside the libc calls the C
  code performs:
  * `fgets`: newline seen / 99-byte cap reached / EOF-with-bytes / EOF-without-bytes;
  * `strlen`: position of the first NUL (0, interior, none within the buffer);
  * `strcspn`: reject set empty / match at first / middle / last byte / no match,
    byte values `0x01..0x7F` vs `0x80..0xFF` (signed-`char` hazard), duplicates;
  * the `s[strlen(s)-1]` chop: chops `\n` / chops real data / underflows to `-1`;
  * stdin kind: regular file (seekable) vs pipe (non-seekable).

Every row below is a combination the C code treats differently. Each is tested
with **many randomized inputs (fixed seed)** through both `.so` files, not a
single hand-picked value.

## Configuration table

### A. Low-level entry point `driver(s1, s2)` — called directly through the FFI

| #   | entry point | configuration (input shape) | test | [x] |
|-----|-------------|------------------------------|------|-----|
| C1  | `driver` | `s1` empty, `s2` empty | `cfg_c1_both_empty` | [x] |
| C2  | `driver` | `s1` non-empty, `s2` empty (empty reject set → result = `strlen(s1)`) | `cfg_c2_empty_reject` | [x] |
| C3  | `driver` | `s1` empty, `s2` non-empty | `cfg_c3_empty_s1` | [x] |
| C4  | `driver` | 1-byte `s1` × 1-byte `s2`, matching and non-matching (all 255 × a sample of byte values) | `cfg_c4_single_bytes` | [x] |
| C5  | `driver` | match at the **first** byte of `s1` (result 0) | `cfg_c5_match_first` | [x] |
| C6  | `driver` | match in the **middle** of `s1` | `cfg_c6_match_middle` | [x] |
| C7  | `driver` | match at the **last** byte of `s1` (result = `strlen(s1)-1`) | `cfg_c7_match_last` | [x] |
| C8  | `driver` | **no** match (disjoint alphabets → result = `strlen(s1)`) | `cfg_c8_no_match` | [x] |
| C9  | `driver` | `s2` with **duplicate** bytes / `s2` longer than `s1` | `cfg_c9_dup_reject_long_s2` | [x] |
| C10 | `driver` | printable-ASCII random strings, random lengths 0…120 (both operands) | `cfg_c10_random_ascii` | [x] |
| C11 | `driver` | full-byte-range random strings `0x01..0xFF` incl. `>= 0x80` (signed-char hazard) | `cfg_c11_random_full_bytes` | [x] |
| C12 | `driver` | small alphabet (`ab`) → high match probability, random lengths | `cfg_c12_small_alphabet` | [x] |
| C13 | `driver` | `s1` containing every byte `0x01..0xFF` exactly once; `s2` = one byte, sweeping the match index over the whole range | `cfg_c13_index_sweep` | [x] |
| C14 | `driver` | oversized operands (1 KiB, 4 KiB, 64 KiB) — past the 100-byte buffers of `main` | `cfg_c14_oversized` | [x] |
| C15 | `driver` | `s1`/`s2` with an **interior NUL** (valid C string that ends early) | `cfg_c15_interior_nul` | [x] |
| C16 | `driver` | many calls in one process (output ordering / buffering of repeated `printf` vs Rust `stdout`): 1000 calls | `cfg_c16_repeated_calls` | [x] |

### B. Whole-program entry point `main()` — stdin-driven, called through the FFI

| #   | entry point | configuration (stdin shape) | test | [x] |
|-----|-------------|------------------------------|------|-----|
| C17 | `main` | 0 lines (empty stdin) | `cfg_c17_zero_lines` | [x] |
| C18 | `main` | 1 line, with `\n` | `cfg_c18_one_line_nl` | [x] |
| C19 | `main` | 1 line, **without** `\n` (EOF-with-bytes) | `cfg_c19_one_line_no_nl` | [x] |
| C20 | `main` | 2 lines, both with `\n` (the nominal case) — randomized | `cfg_c20_two_lines_random` | [x] |
| C21 | `main` | 2 lines, 2nd without `\n` | `cfg_c21_second_no_nl` | [x] |
| C22 | `main` | `> 2` lines (surplus ignored) | `cfg_c22_surplus_lines` | [x] |
| C23 | `main` | first line empty (`"\n"`), second non-empty | `cfg_c23_empty_first_line` | [x] |
| C24 | `main` | both lines empty (`"\n\n"`) | `cfg_c24_both_lines_empty` | [x] |
| C25 | `main` | line 1 length sweep around the `fgets` cap: 97, 98, 99, 100, 101, 102 bytes (with/without `\n`) | `cfg_c25_cap_sweep` | [x] |
| C26 | `main` | line 1 far longer than the cap so its tail becomes line 2 (150 B, 199 B, 10 KiB) | `cfg_c26_spill_into_s2` | [x] |
| C27 | `main` | embedded NUL bytes at various positions in line 1 / line 2 (incl. first byte) | `cfg_c27_nul_positions` | [x] |
| C28 | `main` | CRLF line endings (`\r\n`) — the `\r` survives the chop | `cfg_c28_crlf` | [x] |
| C29 | `main` | non-UTF-8 / high-byte content (`0x80..0xFF`) on both lines | `cfg_c29_high_bytes` | [x] |
| C30 | `main` | stdin is a **pipe** (non-seekable) instead of a regular file, same payloads | `cfg_c30_pipe_stdin` | [x] |
| C31 | `main` | randomized fuzz over the whole stdin shape space (lengths 0…260, random bytes incl. `\n`, `\0`, high bytes, random trailing-newline presence) — 400 cases, fixed seed | `cfg_c31_fuzz_stdin` | [x] |
| C32 | `main` | `\n` as the very first byte of stdin (1-byte input) and single-byte inputs | `cfg_c32_tiny_inputs` | [x] |
| C35 | `main` | the exported `main()` invoked **repeatedly (1/2/3/5×) in one process** — C's `FILE*` stdin keeps its buffered remainder between calls, so the translation must not discard read-ahead data (file *and* pipe stdin, payloads that cross the 4 KiB stdio buffer) | `cfg_c35_repeated_main_calls` | [x] |
| C34 | `main` | stdin is a pipe delivered in 1/3/7-byte chunks with pauses → `read(2)` returns **short reads** in the middle of a line (the `fgets` refill path) | `cfg_c34_chunked_pipe_stdin` | [x] |

Hostile **output** configurations (broken stdout pipe, `/dev/full`, closed fd 1)
are failure modes and therefore listed in `ERRORS.md` (rows E21–E23).

### C. Executable parity (CMake target, not the `.so`)

| #   | entry point | configuration | test | [x] |
|-----|-------------|---------------|------|-----|
| C33 | `c_src/build/driver` (CMake exe) vs `target/debug/driver` (Rust bin) | the same randomized stdin corpus as C31, end to end | `cfg_c33_executables` | [x] |
