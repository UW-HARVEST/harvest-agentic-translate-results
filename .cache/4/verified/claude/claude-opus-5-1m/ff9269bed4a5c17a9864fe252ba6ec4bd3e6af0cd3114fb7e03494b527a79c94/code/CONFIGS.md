# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE

Derived **mechanically** from the C source, the public header and the build
files. This is the mirror of `ERRORS.md`: it enumerates the *valid* input space
and every axis the C actually branches on.

## Axis derivation (from the source, not from assumptions)

### Public entry points (`c_src/include/driver.h`)

```sh
grep -n '^[a-z].*(' c_src/include/driver.h
# 27: void driver(int x);
```

| entry point | linkage | level |
|-------------|---------|-------|
| `void driver(int x)` | exported (`T driver`) | the **only** public entry point — it is simultaneously the highest and the lowest level of the API |
| `static void print_hex(unsigned char *p, int len)` | internal | not reachable by an external consumer (`static`); exercised *through* `driver`, which always passes `len == sizeof(int) == 4` |

There is no convenience/one-shot wrapper vs. low-level split to worry about:
`driver` **is** the lowest-level exported entry point, and it is driven directly
through `dlsym` in every test.

### Runtime options / modes / flags

```sh
grep -nE '^[a-zA-Z_].*=|static [a-z].*;|extern|volatile|struct|typedef' c_src/src/driver.c
# -> no file-scope variables, no setters, no context struct, no flags
```

| axis | values | source of truth |
|------|--------|-----------------|
| runtime options | **none** — the library has 0 globals, 0 setters, 0 context objects, 0 modes | grep above |
| compile-time options (`#ifdef` / cmake `option()`) | **none** — `CMakeLists.txt` sets no `target_compile_definitions`; `driver.c` contains no `#ifdef` besides the header include guard | `c_src/CMakeLists.txt`, `driver.c` |
| cargo features | **none** — `Cargo.toml` has no `[features]` table → one combination: the empty set | `Cargo.toml` |

### Input shapes the code distinguishes

The code's only control flow is `for (int i = 0; i < len; i++) printf("%02x", p[i])`
over the 4 object-representation bytes of `x`. The behaviour therefore varies
along:

| axis | distinct classes | why the C distinguishes them |
|------|------------------|------------------------------|
| **A. per-byte value class** | `0x00`; `0x01–0x09`; `0x0a–0x0f`; `0x10–0x7f`; `0x80–0xff` | `%02x` takes a zero-padding path for values `< 0x10`, emits letter digits for nibbles `0xa–0xf`, and the `unsigned char` → `int` promotion must **not** sign-extend for `>= 0x80` |
| **B. byte position** | `0` (LSB) … `3` (MSB) | `(unsigned char *)&x` exposes the little-endian object representation; each position is a separate loop iteration |
| **C. sign of `x`** | `> 0`, `== 0`, `< 0` | sign lives in the MSB of the representation; a signed→unsigned mistake shows up only for `< 0` |
| **D. interior `0x00` bytes** | present / absent | a string-oriented mistranslation would truncate at a NUL; the C loop is length-driven |
| **E. calls per output stream** | `0`, `1`, `2`, `3`, `17`, `256` | output accumulates in the shared `stdout` `FILE` buffer; ordering/flushing must match |
| **F. stream interleaving** | C-then-Rust, Rust-then-C, alternating | proves neither library owns private buffer state |
| **G. `stdout` buffering mode** | `_IOFBF` (file, default), `_IOLBF`, `_IONBF`, pipe | the trailing `"\n"` interacts with line buffering; C emits it via `putchar`, Rust via `printf` |
| **H. calling thread** | main thread, spawned thread | proves there is no TLS/global state (C has none) |
| **I. locale** | `"C"` (default), `LC_ALL` changed | `%02x` must stay locale-independent in both |
| **J. FFI argument marshalling** | value passed in the low 32 bits with clean vs. dirty upper 32 register bits | the SysV ABI passes `int` in a 64-bit register; both sides must ignore the upper half |

## CONFIGURATION-SURFACE TABLE

Every row is exercised through the `.so` exports of **both** libraries and
compared byte-for-byte. Rows marked *randomized* use a fixed-seed SplitMix64
PRNG (seed `0x243F6A8885A308D3`) so failures reproduce.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | no options (none exist) + `x == 0` — axis A class `0x00` in **all four** byte positions (pure zero-padding path) | [x] |
| C2 | `driver` | `x` exhaustive over `1..=9` — A class `0x01–0x09` in position 0, `0x00` elsewhere (padded decimal digit) | [x] |
| C3 | `driver` | `x` exhaustive over `0x0a..=0x0f` — A class `0x0a–0x0f` in position 0 (padded **letter** digit) | [x] |
| C4 | `driver` | `x` exhaustive over `0x10..=0x7f` — A class `0x10–0x7f` in position 0 (two digits, high bit clear) | [x] |
| C5 | `driver` | `x` exhaustive over `0x80..=0xff` — A class `0x80–0xff` in position 0 (**high bit set**: no sign extension) | [x] |
| C6 | `driver` | position sweep, byte 1: `x = b << 8`, all `b in 0..=255` (axes A×B) | [x] |
| C7 | `driver` | position sweep, byte 2: `x = b << 16`, all `b in 0..=255` (axes A×B) | [x] |
| C8 | `driver` | position sweep, byte 3 (MSB): `x = (b << 24) as i32`, all `b in 0..=255` — crosses axis C (b ≥ 0x80 ⇒ `x < 0`) | [x] |
| C9 | `driver` | all four bytes distinct, non-zero, letter-digit heavy: `0xABCDEF12`, `0xDEADBEEF`, `0xFEEDFACE`, `0x0F1E2D3C`, … (axes A×B×C) | [x] |
| C10 | `driver` | interior/leading/trailing NUL bytes: `0x00FF00FF`, `0xFF0000FF`, `0x0000FF00`, `0xFF00FF00`, `0x00000001`, `0x01000000` (axis D) | [x] |
| C11 | `driver` | `x == -1` (`0xFFFFFFFF`) — axis C negative, all bytes `0xff` | [x] |
| C12 | `driver` | `x == i32::MIN` (`0x80000000`) and `x == i32::MAX` (`0x7FFFFFFF`) — signed extremes | [x] |
| C13 | `driver` | **exhaustive** `x in 0..=0xFFFF` — every low-16-bit pattern (65 536 calls) | [x] |
| C14 | `driver` | **exhaustive** `x in -65536..=-1` (`0xFFFF0000..=0xFFFFFFFF`) — every low-16-bit pattern with a negative high half | [x] |
| C15 | `driver` | **randomized** full 32-bit domain, 20 000 fixed-seed samples (axes A×B×C×D jointly) | [x] |
| C16 | `driver` | zero calls into a capture — empty-stream baseline (axis E = 0) | [x] |
| C17 | `driver` | `N` sequential calls in **one** stdout capture for `N in {1, 2, 3, 17, 256}`, randomized inputs — accumulation & ordering (axis E) | [x] |
| C18 | `driver` | interleaved C/Rust invocations into the **same** buffered `stdout`: `[C,R,C,R]` vs `[R,C,R,C]` over randomized inputs (axis F) | [x] |
| C19 | `driver` | `stdout` explicitly `setvbuf(_IOFBF, 4096)` (fully buffered) + randomized inputs (axis G) | [x] |
| C20 | `driver` | `stdout` explicitly `setvbuf(_IOLBF, 4096)` (line buffered — the trailing `"\n"` forces a flush per call) + randomized inputs (axis G) | [x] |
| C21 | `driver` | `stdout` explicitly `setvbuf(_IONBF, 0)` (unbuffered — one `write(2)` per conversion) + randomized inputs (axis G) | [x] |
| C22 | `driver` | `stdout` redirected to a **pipe** instead of a regular file + randomized inputs (axis G) | [x] |
| C23 | `driver` | invoked from a **spawned thread** (not the main thread) + randomized inputs (axis H) | [x] |
| C24 | `driver` | invoked with `LC_ALL` / `LC_NUMERIC` switched away from `"C"` (via `setlocale`) + randomized inputs (axis I) | [x] |
| C25 | `driver` | argument marshalled with **dirty upper 32 register bits** (called through an `extern "C" fn(i64)` signature) + randomized 64-bit values (axis J) | [x] |
| C26 | `driver` | same input repeated 64× (idempotence: no residual state between calls) | [x] |
| C27 | `driver` | one nibble set at a time: `x = 0xF << (4*k)` and `x = 1 << k` for all `k` — single-bit / single-nibble walk over all 32 bit positions (axes A×B×C) | [x] |

All rows are implemented in `tests/valid_paths.rs`.

## Phase B result

```
running 27 tests
test c1_all_zero_bytes ... ok                     test c15_randomized_full_domain ... ok
test c2_low_byte_decimal_digits ... ok            test c16_zero_calls_empty_stream ... ok
test c3_low_byte_letter_digits ... ok             test c17_call_count_axis ... ok
test c4_low_byte_two_digits_positive ... ok       test c18_interleaved_same_stream ... ok
test c5_low_byte_high_bit_set ... ok              test c19_fully_buffered ... ok
test c6_byte1_sweep ... ok                        test c20_line_buffered ... ok
test c7_byte2_sweep ... ok                        test c21_unbuffered ... ok
test c8_byte3_msb_sweep ... ok                    test c22_stdout_is_a_pipe ... ok
test c9_distinct_letter_heavy_bytes ... ok        test c23_called_from_spawned_thread ... ok
test c10_embedded_nul_bytes ... ok                test c24_non_c_locale ... ok
test c11_minus_one ... ok                         test c25_dirty_upper_register_bits ... ok
test c12_signed_extremes ... ok                   test c26_repeated_identical_input ... ok
test c13_exhaustive_low_16_bits ... ok            test c27_bit_and_nibble_walk ... ok
test c14_exhaustive_negative_low_16_bits ... ok

test result: ok. 27 passed; 0 failed
```

Total distinct `driver` invocations compared byte-for-byte per run:
≈ 175 000 per library (including the two exhaustive 65 536-value sweeps), all
records identical.

Every comparison additionally cross-checks the **C** output against an
independent oracle (`expected_record`) so that a broken harness cannot make a
row pass vacuously.

## Anti-vacuity evidence (mutation testing)

Each mutation was applied to `src/lib.rs`, the `cdylib` rebuilt, and the full
suite re-run (`--no-fail-fast`). Restored to the original afterwards.

| mutation | tests that failed |
|----------|-------------------|
| `%02x` → `%02X` (hex case) | 30 (24 of 27 Phase B rows + 6 Phase C rows) |
| reversed byte order (`p[len-1-i]`) | 30 |
| sign-extended byte (`byte as i8 as c_int`) | 28 |
| wrong length (`sizeof(int) - 1`) | 32 |
| `len = 0` (forces the `i < len` false branch) | 32 |
| export renamed (`no_mangle` → `export_name`) | 42 — the whole suite, incl. the Phase D symbol-parity test |
| *(unmutated)* | **0** |

The rows that legitimately survive the hex-case mutation are exactly the ones
whose expected output contains no letter digit (`x == 0`, `x in 0x01..=0x09`),
the zero-call baseline, the symbol-parity tests, and the error-state tests that
compare `(ferror, errno)` rather than bytes.
