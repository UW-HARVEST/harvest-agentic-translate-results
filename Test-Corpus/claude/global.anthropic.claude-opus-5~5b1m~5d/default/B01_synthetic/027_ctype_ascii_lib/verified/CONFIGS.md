# CONFIGS.md — Phase B configuration-surface table

## How this was derived

`c_src/src/driver.c` is 48 lines and contains a single public entry point:

```c
void driver(char c) {
    setlocale(LC_ALL, "C");
    printf("alphanumeric: %d\n", isalnum(c));   /* ... 11 more class macros ... */
    printf("to lower: %c\n", tolower(c));
    printf("to upper: %c\n", toupper(c));
}
```

Grepping the C for runtime options / modes / flags / `#ifdef` branches:

```
$ grep -nE '#if|#ifdef|switch|if *\(|\|\||&&|getenv|flag|mode|option' c_src/src/driver.c
(no matches)
```

So there are **no runtime options or flags** in the public API, and **no
conditional compilation**. The entire configuration surface is therefore:

* **Axis 1 — the single argument's value.** `driver` has no branches of its own,
  but the twelve `<ctype.h>` classification macros and the two conversion
  functions it calls are *table lookups whose result changes per input byte*.
  Every distinct combination of `{isalnum, isalpha, islower, isupper, isdigit,
  isxdigit, iscntrl, isgraph, isspace, isblank, isprint, ispunct}` bits, plus
  every distinct `tolower`/`toupper` mapping behaviour, is a distinct shape the
  code treats differently. Enumerating the equivalence classes of the `"C"`
  locale over the 256 `char` values gives the rows below.
* **Axis 2 — the signedness of the `char` parameter.** `char` is signed on
  x86-64 Linux, so `(int) c` is negative for bytes `0x80..0xFF`. glibc's ctype
  tables are indexed from `-128`, which is a genuinely different half of the
  table from `0..127`. This crosses axis 1 (negative × every class).
* **Axis 3 — the ambient locale at call time.** `driver` calls
  `setlocale(LC_ALL, "C")` itself, so the *documented* configuration is "C
  locale". But a real consumer's process may already be in another locale (or
  may have `LC_ALL`/`LANG` set in the environment), and `setlocale` mutates
  process-global state. Whether the C and Rust builds agree when the process
  starts in a non-"C" locale, and whether the call is idempotent across repeated
  invocations, are distinct configurations.
* **Axis 4 — call ordering / process state.** The only public entry point is
  also the lowest-level one (there is no convenience wrapper and no lower-level
  helper exported — see `SYMBOLS.md`), so "exercise the low-level entry points
  directly" == calling `driver` directly, which every row does. Ordering and
  repetition are covered as their own rows because `setlocale` is stateful.
* **No `[[bin]]` target.** `cargo metadata` reports a single `cdylib` target and
  `CMakeLists.txt` builds only `add_library(driver SHARED ...)`. There is no
  driver executable, so there is no binary-stdout comparison row; the
  library-level stdout capture in every row below is the equivalent check.

Output comparison for every row is **byte-for-byte on captured `stdout`**
(including embedded NUL bytes, which the `c == 0` case in rows 1/11/13 produces), obtained by redirecting
fd 1 to a temp file around each `dlsym`'d call. Rows marked "randomized" draw
many inputs from the row's equivalence class with a fixed seed
(`SEED = 0x5EED_1234_ABCD_EF01`, xorshift64\*) so the run is reproducible.

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | **Exhaustive sweep**: all 256 `char` values `-128..=127`, one call per process-equivalent capture, compared byte-for-byte. Covers the full cross-product of axes 1×2. | [x] |
| 2 | `driver` | Uppercase ASCII letters `A..Z` (`isupper`+`isalpha`+`isalnum`+`isgraph`+`isprint`; `toupper` identity, `tolower` shifts +32), randomized order | [x] |
| 3 | `driver` | Lowercase ASCII letters `a..z` (`islower`+…; `tolower` identity, `toupper` shifts −32), randomized order | [x] |
| 4 | `driver` | Decimal digits `0..9` (`isdigit`+`isxdigit`+`isalnum`, **not** `isalpha`), randomized order | [x] |
| 5 | `driver` | Hex-only letters `A..F` / `a..f` (`isxdigit` **and** `isalpha` — the overlap case that distinguishes `isxdigit` from `isdigit`) | [x] |
| 6 | `driver` | Non-hex letters `G..Z` / `g..z` (`isalpha` but **not** `isxdigit` — the complement of row 5) | [x] |
| 7 | `driver` | Whitespace that is also blank: `' '` (0x20, `isspace`+`isblank`+`isprint`, **not** `isgraph`) and `'\t'` (0x09, `isspace`+`isblank`+`iscntrl`) — the two cases where `isblank` is set, split across the print/cntrl boundary | [x] |
| 8 | `driver` | Whitespace that is **not** blank: `'\n'` `'\v'` `'\f'` `'\r'` (0x0A–0x0D; `isspace`+`iscntrl`, `isblank` clear) | [x] |
| 9 | `driver` | Punctuation, all 32 printable non-alphanumeric non-space bytes in the three disjoint ASCII punctuation runs `0x21–0x2F`, `0x3A–0x40`, `0x5B–0x60`, `0x7B–0x7E` (`ispunct`+`isgraph`+`isprint`) | [x] |
| 10 | `driver` | Control characters `0x01–0x08`, `0x0E–0x1F` (`iscntrl` only; no `isprint`, no `isspace`) | [x] |
| 11 | `driver` | Boundary values of every class run, one step either side: `0x1F/0x20/0x21`, `0x2F/0x30/0x39/0x3A`, `0x40/0x41/0x5A/0x5B`, `0x60/0x61/0x7A/0x7B`, `0x7E/0x7F`, `0x08/0x09/0x0D/0x0E` | [x] |
| 12 | `driver` | Negative `char` half of glibc's table: all of `-128..=-1` (bytes `0x80..0xFF`) — every class bit must be `0` and `%c` must round-trip the byte | [x] |
| 13 | `driver` | Same 256-value sweep but with the argument supplied as an **unsigned** byte `0..=255` reinterpreted into the signed `char` parameter (the wrap-around at 128 must produce output identical to row 12's negatives) | [x] |
| 14 | `driver` | Randomized property run: 4096 pseudo-random `i8` values (fixed seed), C vs Rust byte-for-byte — value-dependent path coverage beyond the hand-picked classes | [x] |
| 15 | `driver` | **Repetition / statefulness**: 512 consecutive `driver` calls on randomized values captured as one stream, compared as one blob (checks `setlocale` idempotence and that no per-call state leaks) | [x] |
| 16 | `driver` | **Interleaving**: alternating C-call / Rust-call in the same process over randomized values (Rust's in-crate `"C"` tables vs glibc's `setlocale`-managed tables must not interfere) | [x] |
| 17 | `driver` | **Ambient locale ≠ "C"**: process locale pre-set to `en_US.UTF-8` / `C.UTF-8` / `POSIX` via `setlocale` before calling, then the full 256-value sweep — `driver`'s own `setlocale(LC_ALL,"C")` must make both implementations locale-independent and identical | [x] |
| 18 | `driver` | **Ambient locale from environment**: sweep run with `LC_ALL`/`LANG` env vars set to a UTF-8 locale in the test process before any `setlocale` call | [x] |
| 19 | `driver` | Return-value / ABI shape: `driver` is `void`; verify both `.so`s' symbols are callable through the identical `extern "C" fn(c_char)` signature and neither perturbs the caller's stack/registers across 256 calls | [x] |

## Verification result

All 19 rows pass (`tests/phase_b_valid_paths.rs`, single `#[test]`
`phase_b_all_config_rows` which runs every row and reports each on stderr):

```
=== CONFIGS.md: 19 rows ===
  [ 1/19] row01_exhaustive_all_256_char_values ... ok
  [ 2/19] row02_uppercase_letters_randomized_order ... ok
  [ 3/19] row03_lowercase_letters_randomized_order ... ok
  [ 4/19] row04_decimal_digits ... ok
  [ 5/19] row05_hex_only_letters ... ok
  [ 6/19] row06_non_hex_letters ... ok
  [ 7/19] row07_blank_whitespace ... ok
  [ 8/19] row08_nonblank_whitespace ... ok
  [ 9/19] row09_punctuation_all_four_runs ... ok
  [10/19] row10_control_characters ... ok
  [11/19] row11_class_run_boundaries ... ok
  [12/19] row12_all_negative_chars ... ok
  [13/19] row13_unsigned_byte_domain_wraparound ... ok
  [14/19] row14_randomized_property_run ... ok
  [15/19] row15_repeated_calls_single_stream ... ok
  [16/19] row16_interleaved_c_and_rust_calls ... ok
  [17/19] row17_ambient_locale_not_c ... (row17 exercised 5 locales) ok
  [18/19] row18_ambient_locale_from_environment ... ok
  [19/19] row19_abi_shape_void_return_and_no_clobber ... ok
=== CONFIGS.md: all 19 rows passed ===
```

Verified under the default feature set, `--no-default-features`, and both the
release and dev cdylib profiles. Row 17 successfully set and exercised 5
distinct locales on this host (`C`, `POSIX`, `C.UTF-8`, `en_US.UTF-8`,
`de_DE.UTF-8`).

## Harness notes worth keeping

* **`cargo test` does not rebuild a `cdylib`-only crate.** Integration tests do
  not link against a cdylib, so `cargo test` will happily run the whole suite
  against a stale `target/*/libdriver.so`. This was observed live: two
  deliberately injected bugs produced a fully green run. `tests/common/mod.rs`
  now refuses to run if the `.so` is older than any `src/*.rs` or `Cargo.toml`,
  and `run_tests.sh` does the rebuild.
* **libtest writes its own progress lines to fd 1.** The harness must redirect
  fd 1 to capture `printf` output from the shared objects, and libtest's
  `test foo ... ok` lines raced into the captured bytes, producing phantom
  divergences. Each capturing test binary therefore contains exactly **one**
  `#[test]`, which iterates the rows and reports progress on stderr; this
  removes the race regardless of `--test-threads`.
* **Negative controls.** The harness was validated by injecting two bugs into
  `src/ctype.rs` (normalising `isalnum` to `0`/`1`, and dropping `~` from
  `ispunct`) and confirming each was reported with the exact offending input.
  Both were reverted.
