# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/include/driver.h`, `c_src/src/driver.c` and
`c_src/CMakeLists.txt`.

## Axes the C code actually branches on

* **Runtime options / modes / flags:** none. There is no init function, no
  context struct, no global state, no setter, and no environment variable read.
  `grep -nE 'if|switch|#if|#ifdef' c_src/src/driver.c` yields only the loop
  bound `i < len`, and the single `#include`/include guard.
* **Public entry points (full set, incl. the lowest level):** exactly one —
  `void driver(int x)`. The lower-level helper `static void print_hex(unsigned
  char *, int)` has internal linkage and is *not* reachable across the `.so`
  boundary (confirmed by `nm -D`), so `driver` **is** the lowest-level entry
  point available to any external consumer. There is no convenience-wrapper /
  low-level split to under-test here.
* **Input shapes the code special-cases:** the sole input is one `int`
  (4 bytes, native little-endian on x86-64). The observable output depends on
  the *value* of each of its 4 bytes, so the meaningful shape axis is the byte
  pattern: zero bytes, bytes < 0x10 (which exercise the `0` zero-padding in
  `%02x`), bytes >= 0x80 (which exercise `unsigned char` promotion), byte
  position (endianness ordering), and the sign of the whole value.
* **Length axis:** `len` is always `sizeof(int)` == 4; the loop therefore always
  runs exactly 4 iterations, and "empty / one / many" is not caller-selectable.
  Output is always exactly 9 bytes: 8 hex digits + `\n`.
* **Build/feature axes:** `translation/Cargo.toml` has no `[features]` section,
  so the only feature combination is the default (empty) set. `CMakeLists.txt`
  has no options or `#ifdef`-guarded variants.

## Configuration table

Every row is exercised over many randomized inputs with a fixed seed
(`differential.rs`, seed `0x5EED_1234_5678_9ABC`, splitmix64 generator), calling
BOTH `.so` files through `libloading` and comparing captured stdout
byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` | value `0` — all four bytes `0x00`, minimal zero-padding case | `cfg_zero` | [x] |
| C2 | `driver` | small positive values `1..=255` (only byte 0 non-zero; exercises `%02x` padding for every single-byte value) | `cfg_low_byte_sweep` | [x] |
| C3 | `driver` | byte 0 == `0x00`, byte 1 non-zero (`x = n << 8`, all 255 n) — leading zero group | `cfg_byte1_sweep` | [x] |
| C4 | `driver` | byte 2 non-zero only (`x = n << 16`, all 255 n) | `cfg_byte2_sweep` | [x] |
| C5 | `driver` | byte 3 non-zero only (`x = n << 24`, all 255 n) — includes sign-bit values | `cfg_byte3_sweep` | [x] |
| C6 | `driver` | all four bytes equal and >= `0x80` (`0x80808080` … `0xffffffff`) — worst case for signed-char promotion | `cfg_all_bytes_high` | [x] |
| C7 | `driver` | negative values, randomized over `INT_MIN..0` | `cfg_negative_random` | [x] |
| C8 | `driver` | positive values, randomized over `0..INT_MAX` | `cfg_positive_random` | [x] |
| C9 | `driver` | full-range randomized `u32` bit patterns reinterpreted as `int` (2048 values) | `cfg_full_range_random` | [x] |
| C10 | `driver` | powers of two and their negations / off-by-ones (`±(1<<k)`, `(1<<k)-1` for k in 0..32) — byte-boundary carries | `cfg_powers_of_two` | [x] |
| C11 | `driver` | endianness witnesses: values whose 4 bytes are all distinct (`0x01020304`, `0x04030201`, `0xdeadbeef`, `0xefbeadde`) — pins byte order of the memory dump | `cfg_endianness_witnesses` | [x] |
| C12 | `driver` | bytes containing `0x0a` (newline) and `0x00` (NUL) in every position — output framing must come only from the trailing `\n` | `cfg_nul_and_newline_bytes` | [x] |
| C13 | `driver` | repeated invocation: 1000 randomized calls against the *same* loaded handle, output accumulated — checks no per-call state drift and exactly 9 bytes per call | `cfg_repeated_calls_same_handle` | [x] |
| C14 | `driver` | interleaved C/Rust calls through one shared `FILE *stdout` (C,R,C,R… 1000x) — checks buffering/flush behaviour composes identically | `cfg_interleaved_c_and_rust` | [x] |
| C15 | `driver` | stdout redirected to a **regular file** (fully buffered mode) | `cfg_stdout_fully_buffered_file` | [x] |
| C16 | `driver` | stdout redirected to a **pipe** (also fully buffered, but distinct `fstat` mode; drained on a helper thread) | `cfg_stdout_pipe` | [x] |
| C17 | `driver` | output length / shape invariant: every call emits exactly 8 lowercase hex digits then `\n`, and the digits equal the little-endian bytes of `x` | `cfg_output_shape_invariant` | [x] |
| C18 | `driver` | default (and only) feature combination — no `[features]` in `Cargo.toml`; verified by `check_features.sh` | `run_all` | [x] |
