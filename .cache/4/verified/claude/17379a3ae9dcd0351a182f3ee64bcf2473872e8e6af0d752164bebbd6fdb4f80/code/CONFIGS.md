# CONFIGS.md — Phase B: configuration-surface table (valid inputs)

## Axes derived from the C source (not guessed)

The library has **no** runtime options, no init/teardown, no global state, no
modes and no `#ifdef`/CMake configuration axes (see `ERRORS.md` for the grep
that proves the only preprocessor construct is the header guard). The CMake
build offers no options either. So the configuration surface is the
cross-product of the **entry points** with the **input shapes the code
branches on**:

* **Entry points (complete public surface, lowest level first)**
  * `int foo(const char *in, char c)` — the low-level primitive. Exported
    (non-`static`) but *absent from the public header*; a per-wrapper test suite
    that only drove `driver` would never call it directly, so it is driven
    directly here.
  * `void driver(const char *in)` — the one-shot convenience wrapper; composes
    `foo` twice and formats via `printf`.
* **Shapes `foo` distinguishes** (from `for (s = in; s = strchr(s, c); s++)`)
  * needle class: `'A'` / `'x'` (the two values `driver` hard-codes), other
    ASCII, high-bit byte (negative `char`), byte absent from haystack;
  * haystack length: 0, 1, small, large, 1 MiB;
  * match density: none, one, all, adjacent runs (exercises the `s++` skip),
    match at first index, match at last index (adjacent to the terminator);
  * byte-range: printable-only vs full `0x01..=0xFF` (signed-`char` compares);
  * argument-passing shape: `c` delivered as a real `c_char` vs. as a `c_int`
    whose upper 24 bits are garbage (ABI truncation).
* **Shapes `driver` distinguishes**
  * the `('A' count, 'x' count)` pair: (0,0), (n,0), (0,n), (n,m) — governs the
    two printed lines;
  * decimal width of each count (1, 2, 3, 4, 5+ digits) — governs `%d` output
    length;
  * case-variants `'a'`/`'X'` present (must *not* be counted);
  * `printf`-hostile bytes (`%`, `%n`, `\n`, backslash) in the *input* — the
    format string is a fixed literal, so they must not change behaviour.

## Configuration table

Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, xorshift64\*, so runs are reproducible), and both the C
`.so` and the Rust `.so` are called through their exported symbols via
`libloading`; outputs are compared byte-for-byte (`foo`: the returned `int`;
`driver`: the exact bytes written to fd 1, captured with `dup2`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `foo` | `c='A'`; empty haystack `""` (len 0) | [x] |
| 2 | `foo` | `c='A'`; len-1 haystack that **is** the match (`"A"`) | [x] |
| 3 | `foo` | `c='A'`; len-1 haystack that is **not** the match (random non-`A` byte) | [x] |
| 4 | `foo` | `c='A'`; len 2..64 drawn from the 2-letter alphabet `{'A','B'}` — maximal density variety, every match/miss pattern | [x] |
| 5 | `foo` | `c='A'`; exactly one match, pinned at **index 0**, rest non-matching | [x] |
| 6 | `foo` | `c='A'`; exactly one match, pinned at the **last** index (adjacent to the NUL) | [x] |
| 7 | `foo` | `c='A'`; **adjacent runs** `"…AAAA…"` of random run-length — exercises `s++` after a hit | [x] |
| 8 | `foo` | `c='A'`; **zero occurrences**, len 1..64 over an alphabet excluding `'A'` | [x] |
| 9 | `foo` | `c='x'` (the other value `driver` uses); random printable haystack, len 0..256 | [x] |
| 10 | `foo` | random `c` in `0x01..=0x7F`; random haystack over `0x01..=0x7F`, len 0..256 | [x] |
| 11 | `foo` | random `c` in `0x01..=0xFF` (incl. high-bit ⇒ **negative `char`**); random haystack over full `0x01..=0xFF`, len 0..256 | [x] |
| 12 | `foo` | `c` = high-bit byte guaranteed **present** in a haystack of other high-bit bytes (signed-compare trap) | [x] |
| 13 | `foo` | `c='A'`; **large** haystack 4 KiB..8 KiB, ≈50 % density (count ≫ 1000, 4-digit) | [x] |
| 14 | `foo` | `c='A'`; **1 MiB** haystack, ≈50 % density (count ≈ 500 000, 6-digit) | [x] |
| 15 | `foo` | `c='A'`; haystack containing an **embedded NUL** with matches on both sides — count must stop at the NUL | [x] |
| 16 | `foo` | `c` passed as `c_int` with **garbage upper bits** (`0x141`, `0x1_0041`, `0x7FFFFF41`, `0xFFFFFF41`, `-1`, `i32::MAX`, `i32::MIN+1`, plus 2000 random `high\|low` values) over random haystacks — ABI/truncation parity, cross-checked against the plain `c_char` prototype. Values whose **low byte is `0x00`** (`0x100`, `i32::MIN`) are fatal in both implementations and are therefore verified in Phase C (`phase_c_row_9`) instead. | [x] |
| 17 | `foo` | `in` = interior pointer (`in + k`) into a longer buffer — caller-offset shape | [x] |
| 18 | `driver` | empty input `""` ⇒ `(0,0)`, both lines single-digit `0` | [x] |
| 19 | `driver` | input of **only `'A'`s**, random len 1..300 ⇒ `(n,0)`, 1..3-digit | [x] |
| 20 | `driver` | input of **only `'x'`s**, random len 1..300 ⇒ `(0,n)` | [x] |
| 21 | `driver` | random mix of `'A'`,`'x'`,`'a'`,`'X'`,`'B'` — both counts non-zero, **case-variants must be ignored** | [x] |
| 22 | `driver` | random full-byte-range input `0x01..=0xFF`, len 0..512 (both counts usually small, sometimes 0) | [x] |
| 23 | `driver` | `printf`-hostile input bytes `% s n \ "` and `\n` mixed with matches — fixed format literal must be unaffected | [x] |
| 24 | `driver` | **wide counts**: input engineered so the `'A'` count has 5–6 digits and the `'x'` count a different digit width (asymmetric line lengths) | [x] |
| 25 | `driver` | input with an **embedded NUL** — both printed counts must stop at the NUL | [x] |
| 26 | `driver` + `foo` | **composed pipeline** consistency: the two integers parsed out of `driver`'s captured stdout must equal `foo(in,'A')` and `foo(in,'x')` from the *same* `.so`, and cross-`.so` | [x] |
| 27 | `foo`, `driver` | **repeat/statelessness**: the same input driven 3× in a row through both `.so`s, interleaved C→Rust→C, gives identical results (no hidden global state, no order dependence) | [x] |

## Feature / build configurations

`Cargo.toml` declares **no `[features]` table**, and `c_src/CMakeLists.txt`
declares no build options and no `-D` flags, so the configuration matrix has
exactly one member: the empty feature set (`--no-default-features` ==
default). `run_tests.sh` derives the power set of the declared features
mechanically from `Cargo.toml`, so the matrix grows automatically if a feature
is ever added, and runs the whole suite for each combination in **both** the
`dev` and `release` profiles (`release` matters here because it sets
`panic = "abort"` and enables optimizations, which can change how the
out-of-bounds and NULL-pointer rows behave).

| # | features | profile | result |
|---|----------|---------|--------|
| 1 | `--no-default-features` (the only combination) | dev     | 34/34 pass |
| 2 | `--no-default-features` (the only combination) | release | 34/34 pass |

## Suite validation (mutation testing)

To prove the rows are not vacuous, the Rust source was deliberately broken and
the suite re-run; each mutation must be caught:

| mutation | caught? |
|----------|---------|
| `s++` -> `s += 2` (skip a byte after each hit) | yes |
| `res++` -> `res += 2` (off-by-factor count) | yes |
| `driver` counts `'a'` instead of `'A'` | yes |
| format literal `"A: %d\n"` -> `"X: %d\n"` | yes |
| hand-rolled `strchr` instead of libc's (the real bug) | yes — `SIGABRT` vs `SIGSEGV` |
| `c as c_int` -> `c as u8 as c_int` | no — and correctly so: `strchr` compares `(unsigned char)c`, so sign- vs zero-extending the low byte is unobservable. Rows 11/12 and `phase_c_row_10` exercise all 255 non-zero needle values, so it would be caught if it were observable. |
