# CONFIGS.md — configuration / valid-input surface (Phase B)

## Build-time configurations

| axis | values | why |
|------|--------|-----|
| C preprocessor options | **none** | `c_src/src/main.c` contains no `#if`/`#ifdef`/`#define`; `CMakeLists.txt` declares no `option()`/`target_compile_definitions` |
| C artifact kind | executable (per `CMakeLists.txt`) **and** `-shared -fPIC` shared object (same TU, used for the FFI diff) | both are exercised |
| Cargo features | `default = []` — no other feature exists | ⇒ the only two invocations possible are `--features default` and `--no-default-features`, which select identical code |
| Cargo profile | `dev`, `release` (the latter also sets `panic = "abort"` and optimises) | the executable under test is a genuinely different binary |
| Rust artifact kind | `driver` executable + `libcdylib.so` (`[[example]] crate-type = ["cdylib"]`) | both are exercised, and row C26 asserts they agree |

So there is exactly **one** *code* configuration, and
`./run_all_configs.sh` runs `cargo check --all-targets` + the full test-suite
for all **four** build configurations:

| # | invocation |
|---|------------|
| 1 | `cargo test --no-default-features` |
| 2 | `cargo test --no-default-features --release` |
| 3 | `cargo test --features default` |
| 4 | `cargo test --features default --release` |

## Public entry points (all of them, lowest level first)

| entry point | signature | reached from |
|-------------|-----------|--------------|
| `print_foo` | `void print_foo(const foo_t *)` | lowest level; called by `driver` |
| `driver`    | `void driver(unsigned int, unsigned int, bool, int)` | called by `main` |
| `main`      | `int main(void)` | process entry point; also an exported `.so` symbol |

## Runtime axes the C actually branches on

* `print_foo`: the three bit-field reads (`bits & 3`, `bits>>2 & 7`,
  `bits>>5 & 1`) and `z` — i.e. all 256 values of the storage byte × `z`
  patterns × padding-byte patterns.
* `driver`: the three bit-field *stores* (`x & 3`, `y & 7`, `b & 1`) and `z`.
* `main`: glibc `scanf` conversion state machine for `%u %u %d %d`
  — token count (0..4, >4), leading-white-space kind/amount, sign
  (none/`+`/`-`), digit-count and magnitude class (small, `>INT_MAX`,
  `>UINT_MAX`, `>LONG_MAX`, `>ULONG_MAX`), leading zeros, terminating byte
  (white space / EOF / other), separators, stdin kind (pipe / file /
  `/dev/null` / closed / directory), and the reader's 4096-byte buffer
  boundary.

## Configuration rows

Every row is checked with **many randomized inputs** (fixed seed `0x5EED_1234`,
so runs are reproducible) unless it is an exhaustive row.

Rows C1..C12 and C27 live in `tests/ffi_inproc.rs` (in-process `libloading`
calls into both `.so`s, fd 1 captured); rows C13..C26 live in
`tests/phase_b_configs.rs`.

| #   | entry point(s) | configuration (options set + input shape) | test | [x] |
|-----|----------------|-------------------------------------------|------|-----|
| C1  | `print_foo` | **exhaustive** over all 256 values of the bit-field byte, `z = 0` | `cfg_c01_print_foo_all_bits` | [x] |
| C2  | `print_foo` | exhaustive bit-field byte × `z ∈ {0,1,-1,INT_MIN,INT_MAX,0x7f7f7f7f}` | `cfg_c02_print_foo_bits_x_z` | [x] |
| C3  | `print_foo` | random bit-field byte × random `z` (5 000 draws) | `cfg_c03_print_foo_random` | [x] |
| C4  | `print_foo` | bit-field byte with padding bits 6..7 set × random `z` | `cfg_c04_print_foo_padding_bits_set` | [x] |
| C5  | `print_foo` | padding bytes 1..3 = `0x00`/`0xAA`/`0xFF`/random × random `bits`,`z` | `cfg_c05_print_foo_padding_bytes` | [x] |
| C6  | `print_foo` | `z` sweeping all powers of two and their negations, `bits` random | `cfg_c06_print_foo_z_powers` | [x] |
| C7  | `driver` | exhaustive `x ∈ 0..=8`, `y ∈ 0..=8`, `b ∈ {0,1}`, `z ∈ {0,±1}` (small in-range grid) | `cfg_c07_driver_small_grid` | [x] |
| C8  | `driver` | exhaustive `b ∈ 0..=255` (all `_Bool` byte patterns) × random `x`,`y`,`z` | `cfg_c08_driver_all_bool_bytes` | [x] |
| C9  | `driver` | `x`,`y` = every residue class boundary (`0..16`, `2^32-1`, `2^31`, random) | `cfg_c09_driver_xy_boundaries` | [x] |
| C10 | `driver` | `z ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` × random `x`,`y`,`b` | `cfg_c10_driver_z_boundaries` | [x] |
| C11 | `driver` | fully random `x`,`y`,`b`,`z` (5 000 draws) | `cfg_c11_driver_random` | [x] |
| C12 | `driver` | repeated calls in one process (state-freeness / no buffering artefacts) | `cfg_c12_driver_repeated_calls` | [x] |
| C13 | `main` (`.so`, dlopen) | 4 well-formed tokens separated by single spaces, trailing `\n` | `cfg_c13_so_main_simple` | [x] |
| C14 | `main` (`.so`, dlopen) | 4 tokens, randomized magnitudes/signs, randomized separators | `cfg_c14_so_main_random` | [x] |
| C15 | `main` (exe) | 0,1,2,3,4,5,8 tokens (`>4` ⇒ surplus ignored) × trailing newline present/absent | `cfg_c15_exe_token_counts` | [x] |
| C16 | `main` (exe) | separators: every C white-space byte, singly and in runs, incl. leading run | `cfg_c16_exe_separators` | [x] |
| C17 | `main` (exe) | sign forms per token: none / `+` / `-` (3⁴ combinations) | `cfg_c17_exe_sign_forms` | [x] |
| C18 | `main` (exe) | magnitude classes per token: `<2^31`, `2^31..2^32`, `2^32..2^63`, `2^63..2^64`, `>2^64` (cross product, randomized within class) | `cfg_c18_exe_magnitude_classes` | [x] |
| C19 | `main` (exe) | leading zeros (1..30 of them) before a small value, per token | `cfg_c19_exe_leading_zeros` | [x] |
| C20 | `main` (exe) | value-dependent bit-field masking: `x,y` drawn so all 4×8 `(x&3,y&7)` pairs occur, `b` drawn so both `!!b` outcomes occur | `cfg_c20_exe_bitfield_matrix` | [x] |
| C21 | `main` (exe) | token straddling the 4096/8192-byte buffer boundary (leading white space run, long digit run, both) | `cfg_c21_exe_buffer_boundary` | [x] |
| C22 | `main` (exe) | stdin kind: pipe, regular file, empty file, `/dev/null`, directory (`EISDIR`); closed fd in `err_e21_unreadable_stdin` | `cfg_c22_exe_stdin_kinds` | [x] |
| C23 | `main` (exe) | partial/slow reads (data delivered in several `write`s on a pipe) | `cfg_c23_exe_partial_reads` | [x] |
| C24 | `main` (exe) | fully random byte soup from the alphabet `[0-9+- \t\n\v\f\r.xeaA]`, 0..40 bytes (4 000 draws) | `cfg_c24_exe_random_soup` | [x] |
| C25 | `main` (exe) | fully random *arbitrary* bytes, 0..64 long (2 000 draws, incl. NUL/high bytes) | `cfg_c25_exe_random_bytes` | [x] |
| C26 | `main` (exe vs `.so`) | the same inputs routed through the executable and through `dlopen`+`main` | `cfg_c26_exe_matches_so` | [x] |
| C27 | `driver`/`print_foo` composition | `driver(x,y,b,z)` output == `print_foo` on the byte the C bit-field store produces (pipeline consistency) | `cfg_c27_driver_print_foo_pipeline` | [x] |
| C28 | all | all four build configurations (2 feature invocations × 2 profiles) | `run_all_configs.sh` | [x] |
