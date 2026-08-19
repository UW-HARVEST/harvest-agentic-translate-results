# CONFIGS.md — configuration surface table (Phase A, tested in Phase B)

## Axes the C actually branches on

Derived from `c_src/src/main.c` and `c_src/CMakeLists.txt`:

* **Runtime options / modes / flags:** none. There is no init function, no
  context struct, no setter, no global, no environment variable and no
  `#ifdef`. `CMakeLists.txt` declares no `option()` and no compile definitions.
  The only *build*-time axis is the optimisation level, which changes whether
  gcc vectorises the `fma_array` loop — so both a `-O0` and an `-O2` C `.so`
  are built and every row is run against **both**.
* **Entry points (all three, lowest level first):**
  1. `fma_array(out, mul1, mul2, add, len)` — the lowest-level primitive.
  2. `call_fma(data, len)` — composes scratch buffers + `fma_array`.
  3. `main()` — the top-level driver: `scanf("%d")` loop + `call_fma` + `printf`.
     Exercised (a) by `dlopen`-ing the `.so` and calling its exported `main`
     with a piped stdin, and (b) end-to-end against the CMake-built `driver`
     executable.
* **Input shapes the code distinguishes:**
  * `len` sign/size: `len == 0` (explicit branch), `len > 0`, `len < 0`;
    boundary lengths 1, 2, 3, 4, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65 (SIMD
    vector-width boundaries for the `-O2` build), 100 (the `main` cap), 1 000,
    65 536, 200 000.
  * element **values**: zeros, ones, `INT_MAX`, `INT_MIN`, `-1`, values whose
    product overflows `int`, values whose product-plus-add overflows `int`,
    full-range random.
  * **buffer aliasing among the read-only pointers** (legal: only `out` is
    `restrict`): `mul1 == mul2` (square), `mul1 == add`, all three identical.
  * `out` buffer **padded** past `len` (checks nothing outside `[0, len)` is
    written).
  * stdin **token count**: 0, 1, 2, 99, 100, 101, 250.
  * stdin **separators**: single space, tab, `\n`, `\r`, `\v`, `\f`, runs and
    mixtures; leading and trailing whitespace; no trailing newline.
  * stdin **number syntax**: bare digits, `+n`, `-n`, `-0`, leading zeros, long
    zero runs, digit prefix followed by a non-digit.
  * stdin **magnitudes**: 0, ±1, `INT_MAX`, `INT_MIN`, `INT_MAX+1`,
    `UINT32_MAX`, `LONG_MAX`, `LONG_MAX+1`, `LONG_MIN`, `LONG_MIN-1`, 29-digit
    and 400-digit runs (glibc saturation path).
  * stdin **chunking**: written to the pipe in one `write` vs. byte-at-a-time
    (the Rust reimplements `scanf`'s buffering, so partial reads matter).

Rows below are the pruned cross-product of those axes: one row per combination
the C treats differently. Every row is asserted byte-identical between the C
`.so` and the Rust `.so` (both `dlopen`-ed; the Rust is never called directly),
with **many randomized inputs per row** driven by a fixed-seed xorshift PRNG so
runs are reproducible.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `fma_array` | `len == 0`, distinct non-null buffers, canary-filled `out` — asserts zero bytes written | `cfg_c1_fma_array_len_zero` | [x] |
| C2 | `fma_array` | `len == 1`, random values ×2000 | `cfg_c2_fma_array_len_one_random` | [x] |
| C3 | `fma_array` | `len ∈ {2,3,4,5,6,7,8}` (scalar / partial-vector), random values ×500 per len | `cfg_c3_fma_array_small_lens_random` | [x] |
| C4 | `fma_array` | `len ∈ {15,16,17,31,32,33,63,64,65}` (SIMD width boundaries for `-O2`), random values ×200 per len | `cfg_c4_fma_array_simd_boundary_lens` | [x] |
| C5 | `fma_array` | `len == 100` (the `main` cap), random values ×300 | `cfg_c5_fma_array_len_100_random` | [x] |
| C6 | `fma_array` | `len ∈ {1000, 65536}`, random values ×20 | `cfg_c6_fma_array_large_lens` | [x] |
| C7 | `fma_array` | all-zero inputs, `len` 1…64 | `cfg_c7_fma_array_all_zeros` | [x] |
| C8 | `fma_array` | all-ones inputs (`mul1=mul2=add=1`), `len` 1…64 | `cfg_c8_fma_array_all_ones` | [x] |
| C9 | `fma_array` | extreme values only (`INT_MIN`, `INT_MAX`, `-1`, `0`, `1`) drawn randomly ×2000 — exercises signed multiply **and** add wraparound | `cfg_c9_fma_array_extreme_values` | [x] |
| C10 | `fma_array` | products chosen to overflow `int` (`mul1`,`mul2` random 16-bit-plus magnitudes ×2000) | `cfg_c10_fma_array_multiply_overflow` | [x] |
| C11 | `fma_array` | non-overflowing product plus `add == INT_MAX`/`INT_MIN` (add-side wraparound) ×2000 | `cfg_c11_fma_array_add_overflow` | [x] |
| C12 | `fma_array` | aliasing among the read-only pointers: `mul1 == mul2` (square), random ×1000 | `cfg_c12_fma_array_alias_mul1_mul2` | [x] |
| C13 | `fma_array` | aliasing: `mul2 == add`, random ×1000 | `cfg_c13_fma_array_alias_mul2_add` | [x] |
| C14 | `fma_array` | aliasing: `mul1 == mul2 == add` (all three read-only pointers identical), random ×1000 | `cfg_c14_fma_array_alias_all_inputs` | [x] |
| C15 | `fma_array` | `out` padded with a canary tail; `len` < buffer length, random ×1000 — asserts nothing past `len-1` is touched | `cfg_c15_fma_array_out_padding_untouched` | [x] |
| C16 | `fma_array` | `len < 0` (`-1`, `-7`, `INT_MIN`) with valid canary buffers — asserts no writes | `cfg_c16_fma_array_negative_len_no_writes` | [x] |
| C17 | `fma_array` | repeated calls into the same `out` buffer with increasing `len` (stateless-ness check), random ×500 | `cfg_c17_fma_array_repeated_calls` | [x] |
| C18 | `call_fma` | `len == 0`, non-null `data` | `cfg_c18_call_fma_len_zero` | [x] |
| C19 | `call_fma` | `len == 1`, random `data[0]` full range ×2000 | `cfg_c19_call_fma_len_one_random` | [x] |
| C20 | `call_fma` | `len ∈ 2…8`, random data ×500 per len | `cfg_c20_call_fma_small_lens_random` | [x] |
| C21 | `call_fma` | `len ∈ {15,16,17,31,32,33,63,64,65,100}`, random data ×200 per len | `cfg_c21_call_fma_boundary_lens` | [x] |
| C22 | `call_fma` | `len ∈ {1000, 65536, 200000}`, random data ×10 — run on a 512 MiB thread (`with_big_stack`) because the C puts 12·len bytes of VLAs on the *caller's* stack and libtest threads get only 2 MiB; E6 pushes the same path to `len = 8 000 000` | `cfg_c22_call_fma_large_lens` | [x] |
| C23 | `call_fma` | `data` buffer strictly longer than `len` (only the first `len` elements may be read) ×1000 | `cfg_c23_call_fma_data_longer_than_len` | [x] |
| C24 | `call_fma` | extreme-value data (`INT_MIN`/`INT_MAX`/`-1`/`0`/`1`) ×2000 | `cfg_c24_call_fma_extreme_values` | [x] |
| C25 | `call_fma` | same buffer reused across many calls with varying `len` (stateless-ness) ×500 | `cfg_c25_call_fma_repeated_calls` | [x] |
| C26 | `main` (via `.so`) | empty stdin | `cfg_c26_main_empty` | [x] |
| C27 | `main` (via `.so`) | exactly one integer, random full-range value ×300, no trailing newline | `cfg_c27_main_single_random` | [x] |
| C28 | `main` (via `.so`) | exactly one integer, random full-range value ×300, with trailing newline | `cfg_c28_main_single_random_trailing_nl` | [x] |
| C29 | `main` (via `.so`) | 2…10 integers, space separated, random ×200 | `cfg_c29_main_multi_space` | [x] |
| C30 | `main` (via `.so`) | 2…10 integers separated by randomly chosen whitespace runs drawn from `{' ', '\t', '\n', '\r', '\v', '\f'}` ×200 | `cfg_c30_main_multi_random_whitespace` | [x] |
| C31 | `main` (via `.so`) | leading whitespace before the first integer ×200 | `cfg_c31_main_leading_whitespace` | [x] |
| C32 | `main` (via `.so`) | 99 / 100 / 101 / 250 integers (the `i < 100` cap boundary) ×40 each | `cfg_c32_main_count_boundaries` | [x] |
| C33 | `main` (via `.so`) | explicit `+` sign, `-` sign, `-0`, and leading-zero forms mixed randomly ×300 | `cfg_c33_main_sign_and_leading_zeros` | [x] |
| C34 | `main` (via `.so`) | magnitude boundaries: `0`, `±1`, `INT_MAX`, `INT_MIN`, `INT_MAX+1`, `UINT32_MAX`, `LONG_MAX`, `LONG_MAX±1`, `LONG_MIN`, `LONG_MIN-1` in random positions ×300 | `cfg_c34_main_magnitude_boundaries` | [x] |
| C35 | `main` (via `.so`) | very long digit runs (29 and 400 digits, signed and unsigned, plus long leading-zero runs) ×200 | `cfg_c35_main_long_digit_runs` | [x] |
| C36 | `main` (via `.so`) | fully random byte soup restricted to `[0-9+- \t\n]` ×600 — exercises arbitrary interleavings of valid tokens, signs and separators | `cfg_c36_main_random_token_soup` | [x] |
| C37 | `main` (via `.so`) | stdin delivered byte-at-a-time (one `write` per byte, so every `read` returns 1 byte) ×100 | `cfg_c37_main_byte_at_a_time_stdin` | [x] |
| C38 | `main` (via `.so`) | stdin larger than the 4096-byte read buffer (≫ one `read`), ×40 | `cfg_c38_main_large_stdin` | [x] |
| C39 | `driver` executable (CMake-built C `driver` vs `cargo`-built Rust `driver`) | the same randomized stdin corpus as C26–C38, end to end: stdout bytes **and** exit status | `cfg_c39_driver_executables_end_to_end` | [x] |
| C40 | `fma_array` then `call_fma` in one loaded library instance | interleaved calls to both exports on the same handle ×500 (composed-pipeline check: no shared state, no ordering effect) | `cfg_c40_interleaved_exports` | [x] |
| C41 | all exports | every row above re-run against the `-O2`-compiled C `.so` as well as the `-O0` one | every test is parameterised over both `.so`s | [x] |
| C42 | all exports | every row above re-run under both feature configurations (`cargo test` and `cargo test --no-default-features`; the crate declares no features, so these are the only two invocations) | `scripts/verify_all.sh` | [x] |
| C43 | `main` (via `.so`) | **exhaustive**: each of the 256 possible byte values used (a) as the leading byte, (b) as the separator between two integers, (c) as the trailing byte, (d) immediately after a sign — pins glibc's C-locale `isspace` set instead of sampling it | `cfg_c43_main_every_byte_as_separator` | [x] |
| C44 | `main` (via `.so`) | unrestricted random byte fuzz over all 256 values (biased towards digits/signs/whitespace so tokens still form) ×400 | `cfg_c44_main_full_byte_fuzz` | [x] |
| C45 | `main` (via `.so`) + both driver executables | stdin kinds other than a pipe: `/dev/null`, a **seekable regular file** with content (glibc buffers those differently from pipes), an empty regular file, and a **write-only descriptor on fd 0** so every `read` fails with `EBADF` | `cfg_c45_main_unusual_stdin_kinds` | [x] |
| C46 | all exports | every row above re-run in the **release** profile (optimisations on, debug assertions off, `panic = "abort"` for the cdylib) as well as `dev` | `scripts/verify_all.sh` | [x] |

## Harness self-check (mutation testing)

Passing rows only mean something if the harness can actually see a divergence,
so the following deliberate mutations were injected into `src/fma.rs`, rebuilt,
and confirmed to make specific rows fail (all were then reverted):

| mutation | rows that caught it |
|----------|---------------------|
| drop `0x0b` (vertical tab) from `is_c_space` | C30, C31, C32, C33, C34, C35, C37, C39 |
| replace glibc's `LONG_MAX`/`LONG_MIN` saturation with wrapping | C34, C35, C36, C37, C39 + E16, E19 |
| `fma_array` loop bound `i < len` → `i + 1 < len` | 36 of 42 rows |
| `fma_array` result `+ 1` | 36 of 42 rows |
| `call_fma` fills `ones[i] = 2` instead of `1` | 21 rows (every `call_fma` and `main` row) |
| `call_fma`'s `if (len == 0) return 0` → `if (len <= 0) return 1` | E1, E2, E4, E23 |

One mutation was deliberately *not* expected to fail: rewriting the kernel as
`(a as i64 * b as i64 + c as i64) as i32`, which is bit-for-bit equivalent to
the wrapping `i32` form, correctly produced no failures.
