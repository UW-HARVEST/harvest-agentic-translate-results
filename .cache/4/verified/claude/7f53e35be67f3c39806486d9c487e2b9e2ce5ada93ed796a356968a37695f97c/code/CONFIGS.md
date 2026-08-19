# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## Axes the C code actually branches on

Derived from `c_src/src/main.c` + `c_src/CMakeLists.txt`:

* **Build-time options:** none. No `#ifdef`/`#if` in the C source, no `option()`
  or compile definitions in `CMakeLists.txt`, no `[features]` in `Cargo.toml`.
  → a single configuration (see `SYMBOLS.md`).
* **Runtime options/flags:** none. The program takes no argv, no environment
  variable, and no setter; the public surface is two functions:
  * `void driver(int x)` — the lowest-level entry point (called directly through
    the `.so` export, not only through the `main` wrapper);
  * `int main()` — the composed pipeline `scanf("%d") → driver → printf`.
* **Value-shape axes for `driver(int x)`** (the branch-free arithmetic
  `y = 2*x; y += 300` still has value-dependent wrap-around behaviour):
  sign, magnitude, and which of the two additions overflows —
  `2*x` overflow at `|x| ≥ 2^30`, `y += 300` overflow for
  `x ∈ [1073741674, 1073741823]`.
* **Input-shape axes for `main`** (each is a real branch inside the `%d`
  conversion the translation reimplements): leading-whitespace kind and length,
  optional sign, digit count, leading zeros, first-byte class, terminator
  (EOF vs newline vs other), magnitude vs `INT_MAX`/`LONG_MAX`, stdin length vs
  the 4096-byte read chunk, and stdin/stdout being a file, a pipe delivered in
  chunks, or closed.
* Note: `auto int y` is the C89 *storage-class* specifier (gcc 11 default
  `-std=gnu17`), i.e. a plain automatic `int`; it has no behavioural effect.

## Rows (each verified with many randomized inputs, fixed seed)

Both `.so`s are loaded with `libloading` and the exported symbols are compared
byte-for-byte on stdout (plus exit status where a process is involved).

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|--------------------------------------------|-----|
| 1  | `driver` (FFI) | `x = 0` | [x] |
| 2  | `driver` (FFI) | `x` random in `1..=1000` (small positive, no overflow) | [x] |
| 3  | `driver` (FFI) | `x` random in `-1000..=-1` (small negative, no overflow) | [x] |
| 4  | `driver` (FFI) | boundary set `{1, -1, 2, -2, 149, 150, 151, -150, -151}` (`y` around 0) | [x] |
| 5  | `driver` (FFI) | `x = INT_MAX`, `INT_MAX-1`, `INT_MAX/2`, `INT_MAX/2+1` | [x] |
| 6  | `driver` (FFI) | `x = INT_MIN`, `INT_MIN+1`, `INT_MIN/2`, `INT_MIN/2-1` | [x] |
| 7  | `driver` (FFI) | `x ∈ [1073741674, 1073741823]` — `2*x` fits, `+300` overflows (all 150 values) | [x] |
| 8  | `driver` (FFI) | `x` random in `1073741824..=INT_MAX` — `2*x` overflows | [x] |
| 9  | `driver` (FFI) | `x` random in `INT_MIN..=-1073741824` — `2*x` underflows | [x] |
| 10 | `driver` (FFI) | `x = ±2^k` for `k = 0..=31` (powers of two, incl. sign bit) | [x] |
| 11 | `driver` (FFI) | `x` uniform random over the whole `i32` range (2000 values) | [x] |
| 12 | `driver` (FFI) | `x` = high-bit / all-ones bit patterns (`0x8000_0000`, `0xFFFF_FFFF`, `0x7FFF_FFFF`, `0xAAAA_AAAA`, `0x5555_5555`) | [x] |
| 13 | `main` (FFI, fresh child per case) | plain decimal digits, no sign, EOF-terminated, random `0..=10^9`, 200 values | [x] |
| 14 | `main` (FFI) | same value, newline-terminated / CRLF-terminated | [x] |
| 15 | `main` (FFI) | explicit `+` sign / explicit `-` sign, random magnitudes | [x] |
| 16 | `main` (FFI) | leading whitespace: each of `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'` singly and mixed, before a random value | [x] |
| 17 | `main` (FFI) | leading whitespace run of 4095/4096/4097/10000 bytes (crosses the read chunk) then a value | [x] |
| 18 | `main` (FFI) | leading zeros: `"0"`, `"-0"`, `"+0"`, `"0000000042"`, `"-0000042"` | [x] |
| 19 | `main` (FFI) | digit-count sweep 1…19 digits (random per length) — `int` vs `long` ranges | [x] |
| 20 | `main` (FFI) | magnitude classes: `INT_MAX`, `INT_MAX+1`, `INT_MIN`, `INT_MIN-1`, `2^32`, `2^32+1`, `LONG_MAX`, `LONG_MAX+1`, `LONG_MIN`, `LONG_MIN-1` as text | [x] |
| 21 | `main` (FFI) | digit run ≥ one read chunk: 4095/4096/4097/10000 `9`s | [x] |
| 22 | `main` (FFI) | valid value + trailing garbage: letters, `" 2"` second number, punctuation, `"\n7"` | [x] |
| 23 | `main` (FFI) | uniform random `i32` rendered as decimal text, 500 values (property loop) | [x] |
| 24 | `main` (FFI) | random `i64` (out of `int` range) rendered as decimal text, 500 values | [x] |
| 30 | `driver` (FFI) | strided sweep of 65536 values across the whole `int` domain (`INT_MIN + k*65537`) | [x] |
| 25 | `driver` bin vs C bin (process) | stdin from a regular file; stdout to a regular file | [x] |
| 26 | `driver` bin vs C bin (process) | stdin from a pipe written in small chunks; stdout to a pipe | [x] |
| 27 | `driver` bin vs C bin (process) | randomized corpus (all shapes of rows 13–24) end-to-end, comparing stdout bytes **and** exit status | [x] |
| 28 | `main` (FFI) | byte-level fuzz: 400 random strings over `[0-9+- \t\n\r\v\f a b x e E . , NUL 0xFF /]`, length 0–24 | [x] |
| 29 | `main` (FFI) | same fuzz prefixed with a 4090–4100 byte run so the token straddles the read chunk | [x] |

## Verification status

Every row above is checked off: its test calls **both** `.so`s through their
exported symbols (`driver` / `main`, resolved with `libloading`) in that
configuration and compares stdout byte-for-byte (rows 25-27 additionally compare
stderr and the exact termination status of the two real executables).

| rows | test file / names | status |
|------|-------------------|--------|
| 1–12, 30 | `tests/valid_paths.rs::row01…row12`, `row30` (`driver` via the `.so`, ~70000 values total) | [x] PASS |
| 13–24, 28–29 | `tests/valid_paths.rs::row13…row29` (`main` via the `.so`, one forked child per input, ~2400 inputs) | [x] PASS |
| 25–27 | `tests/binary_diff.rs::row25…row27` (cmake-built C binary vs the Rust `driver` binary, 528-input corpus each) | [x] PASS |

All randomized rows use `Rng` (SplitMix64) with a hard-coded per-row seed, so a
failure is exactly reproducible.

Command: `cargo test` (or `./run_all_configs.sh` for every feature combination).
