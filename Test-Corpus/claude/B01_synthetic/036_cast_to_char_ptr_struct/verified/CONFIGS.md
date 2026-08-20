# CONFIGS.md — configuration-surface table (Phase A / Phase B)

## Build-time configurations

| source | configuration knobs | valid combinations |
|--------|--------------------|--------------------|
| `Cargo.toml` | **no `[features]` section at all** | exactly 1: the default (empty) feature set |
| `src/**` | `grep -rn "cfg(feature" src/` → no matches | — |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `target_compile_definitions`, no build-type branches | exactly 1 |
| `c_src/src/main.c` | `grep -n "#if\|#ifdef\|#ifndef" ` → no matches | exactly 1 |

**Enumeration of every valid feature combination: `{}` (the empty set).**
Therefore `cargo check/test --no-default-features` *is* the full matrix; there
is no second code path to re-verify. `tests/phase_d_symbols.rs::d3_single_build_configuration`
asserts mechanically that `Cargo.toml` still has no `[features]` table, so this
row cannot silently go stale.

## Runtime configuration axes (derived from the C branches)

The C code contains no options, flags or modes — no globals, no `argv` parsing
(`main` takes no parameters), no environment reads. The axes it *does* branch
on are:

| axis | where the C branches on it | distinct values the code treats differently |
|------|---------------------------|---------------------------------------------|
| A. entry point | exported `driver` vs exported `main` | `driver(int)` (low level, no I/O parsing); `main()` (composed: `scanf` → `driver` → `print_hex`) |
| B. byte pattern of `floors` | `print_hex`'s `printf("%02x", p[i])` per byte | byte `0x00`; bytes `0x01`–`0x0f` (require the `0` pad of `%02x`); bytes `0x10`–`0x7f`; bytes `0x80`–`0xff` (`unsigned char` promotion — no sign extension); byte = `0x0a`/`0x0d`/`0x09`/`0x20` |
| C. `floors` sign / extremes | none (no branch), but the object representation differs | `0`, `1`, `-1`, `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, arbitrary |
| D. `print_hex` loop trip count | `i < len` with `len = sizeof(house_t)` = 16 | fixed 16 → every one of the 16 struct offsets, incl. the 8 `double` bytes and the 4 `bedrooms` bytes |
| E. leading whitespace in stdin | `scanf` `%d`'s implicit whitespace skip (`isspace`) | none; each of `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'`; a long mixed run |
| F. sign character | `strtol` sign handling inside `%d` | absent; `'+'`; `'-'` |
| G. digit-string shape | `%d`'s digit accumulation | 1 digit; many digits; leading zeros; 4096-digit string |
| H. magnitude class | `strtol` range clamping then `(int)` cast | fits `int`; `> INT_MAX` within `long`; `< INT_MIN` within `long`; `> LONG_MAX`; `< LONG_MIN`; exact `LONG_MAX`/`LONG_MIN`; multiples of 2^32 |
| I. conversion terminator | `%d` stops at the first non-digit | EOF; whitespace; non-digit; start of a second number |
| J. invocation mode | the two build products | `dlopen` + `dlsym` of the `.so` (both `driver` and `main`); the linked executable (`add_executable` / `[[bin]]`) |
| K. invocation context | `scanf`/`printf` act on the process-wide libc `stdin`/`stdout`, which a host shares | standalone program; `dlopen`ed by a host that reads with C stdio **before** the call; **after** the call; a host that writes to stdout around the call; a host that leaves through `_exit`; stdin seekable vs a pipe |

## Rows — the pruned cross-product actually distinguished by the C

Each row is run against **both** `.so`s (and rows C22–C23 against both
executables) and asserted byte-identical. Rows marked *(randomized)* use a
fixed-seed xorshift PRNG with many iterations per row.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` via `dlsym` | `floors = 0` — all 16 bytes zero except `bedrooms`/`bathrooms` (axis B: `0x00`) | `c1_floors_zero` | [x] |
| C2 | `driver` via `dlsym` | `floors` whose every byte is in `0x01..=0x0f` — exercises `%02x` zero padding (`0x01020304`, `0x0f0f0f0f`, `0x01010101`, …) | `c2_low_nibble_bytes_zero_padded` | [x] |
| C3 | `driver` via `dlsym` | `floors` positive with bytes ≥ `0x10`, no high bit (`0x12345678`, `0x7f7f7f7f`, …) | `c3_positive_high_bytes` | [x] |
| C4 | `driver` via `dlsym` | `floors` negative / high bit set (`0x80000000`, `0xdeadbeef`, `0xffffffff`, …) — `unsigned char` promotion must not sign-extend | `c4_negative_high_bit_bytes` | [x] |
| C5 | `driver` via `dlsym` | `floors` at the `int` extremes: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` | `c5_int_boundaries` | [x] |
| C6 | `driver` via `dlsym` | `floors` with embedded zero bytes (`0x00ff00ff`, `0xff0000ff`, `0x00000001`, `0x01000000`) | `c6_embedded_zero_bytes` | [x] |
| C7 | `driver` via `dlsym` | `floors` whose bytes are newline/tab/space codes (`0x0a0d0920`, `0x0a0a0a0a`) — must be hex, never literal control characters | `c7_control_character_bytes` | [x] |
| C8 | `driver` via `dlsym` | `floors` uniform over the full 32-bit range *(randomized, 4096 values)* | `c8_random_full_range` | [x] |
| C9 | `driver` via `dlsym` | many calls in one loaded instance — the struct must be re-zeroed per call, output must not depend on call order *(randomized, interleaved repeats)* | `c9_repeated_calls_no_state_leak` | [x] |
| C10 | `main` via `dlsym` | plain decimal, no whitespace, no sign, EOF terminator (axes E=none, F=none, I=EOF) | `c10_plain_decimal` | [x] |
| C11 | `main` via `dlsym` | each single leading whitespace character `' '`, `'\t'`, `'\n'`, `'\v'`, `'\f'`, `'\r'` before the number (axis E) | `c11_each_whitespace_prefix` | [x] |
| C12 | `main` via `dlsym` | long mixed whitespace run before the number (axis E) | `c12_mixed_whitespace_run` | [x] |
| C13 | `main` via `dlsym` | explicit `'+'` and `'-'` sign with in-range magnitudes (axis F × H=fits int) | `c13_explicit_signs` | [x] |
| C14 | `main` via `dlsym` | leading zeros: `"0"`, `"0000000005"`, `"-000012"`, 100 zeros then a digit (axis G) | `c14_leading_zeros` | [x] |
| C15 | `main` via `dlsym` | magnitude `> INT_MAX` but within `long` → `(int)` truncation (`2147483648`, `4294967296`, `4294967297`, `2147483648*3`) (axis H) | `c15_above_int_max_truncates` | [x] |
| C16 | `main` via `dlsym` | magnitude `> LONG_MAX` → `strtol` saturation to `LONG_MAX` → `-1` (axis H) | `c16_above_long_max_saturates` | [x] |
| C17 | `main` via `dlsym` | negative below `INT_MIN` within `long`, and below `LONG_MIN` → `LONG_MIN` → `0` (axis H) | `c17_below_int_min_and_long_min` | [x] |
| C18 | `main` via `dlsym` | terminator variants after a valid number: EOF, `' '`, `'\n'`, non-digit letter, a second number (axis I) | `c18_terminator_variants` | [x] |
| C19 | `main` via `dlsym` | very long digit strings: 4096 digits, 4096 leading zeros then `7` (axis G × H) | `c19_very_long_digit_strings` | [x] |
| C20 | `main` via `dlsym` | decimal strings sampled across all magnitude classes *(randomized, 512 inputs)* (axis H) | `c20_random_decimal_magnitudes` | [x] |
| C21 | `main` via `dlsym` | full random cross-product of axes E×F×G×H×I: random whitespace prefix + random sign + random digit string + random terminator *(randomized, 512 inputs)* | `c21_random_axis_crossproduct` | [x] |
| C22 | linked executables (`add_executable` vs `[[bin]]`) | the same randomized stdin corpus as C20/C21 driven end to end through the real programs; stdout **and** exit status compared *(randomized, 256 inputs)* | `c22_executables_end_to_end` | [x] |
| C23 | linked executables | with and without a trailing newline on stdin; stdin from a pipe (fully buffered stdout) — buffered output must still be flushed identically | `c23_trailing_newline_and_pipe_buffering` | [x] |
| C24 | `main` via `dlsym` | the FFI return value of `main` for every input class above (must be `0`) | `c24_main_return_value` | [x] |

### Rows for the remaining axes: `argv`, repeated invocation, chunked input

An exported `main` may be called more than once by a consumer of the shared
library, and the axes below are the ones the C distinguishes in that case.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| C25 | linked executables | extra command-line arguments (`main` is declared without parameters and ignores `argv`): none, one, several, an empty argument | `c25_extra_argv_ignored` | [x] |
| C26 | `main` via `dlsym`, **called twice** | the second conversion continues on the same stream, so the row covers where the first one left it: digit terminator (`"12x34"`), whitespace terminator (`"12 34"`), a `'-'` terminator reused as a sign (`"5-6"`), a mismatching character after a sign (`"- 5"`, `"--3"`), EOF (`"12"`, `""`) | `c26_two_calls_stream_position` | [x] |
| C27 | `main` via `dlsym`, **called 4×** | whole token lists: space/newline/mixed separated, signed, overflowing, exhausted early, blocked by a non-digit | `c27_many_calls_consume_a_list` | [x] |
| C28 | `main` via `dlsym`, called 1–5× | randomized token streams mixing whitespace runs, signs, all magnitude classes and non-matching tokens *(randomized, 192 streams)* | `c28_random_token_streams` | [x] |
| C29 | `main` via `dlsym` | stdin delivered in **several chunks** with pauses, so one conversion spans multiple `read` calls: split digits, split sign, late number, early mismatch, split overflow, one byte at a time | `c29_stdin_arrives_in_chunks` | [x] |

Row C26–C28 expectations were measured from glibc with successive
`fscanf("%d")` calls on one stream; they are the rows that caught the pushback
and sticky-EOF divergences recorded in ERRORS.md.

### Rows for axis K — the invocation context (shared libc streams)

These are valid-path configurations, not error paths: the same call, made from a
different kind of host. They are covered by `tests/phase_c_stdio.rs` and are
listed as rows E20–E24 in ERRORS.md as well, since they are the rows that caught
the stdio-state divergences.

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|-------------------------------------------|------|-----|
| C30 | `main` via `dlsym`, host uses C stdio | the host performs `scanf("%d")` **before** calling `main`, so the library must continue where the host stopped (`"5 7"` → host `5`, library prints `7`); 9 inputs including mismatches and overflow | `e21_stdin_shared_with_the_host` | [x] |
| C31 | `main` via `dlsym`, host uses C stdio | the host performs `scanf("%d")` **after** the call, so it must see what the library left (`"5 7"` → library prints `5`, host reads `7`) | `e21_stdin_shared_with_the_host` | [x] |
| C32 | `main` via `dlsym` | the host writes markers to stdout with `printf` around the call (ordering), and the host leaves through `_exit` (durability of the buffered line) | `e22_stdout_shared_with_the_host` | [x] |
| C33 | executables and `main` via `dlsym` | stdin is a **seekable file** that is only partially consumed: the descriptor position left behind, and what a following reader sees (11 measured offsets + 4 remainders) | `e20_stdin_offset_restored_at_exit`, `e24_next_reader_sees_the_remainder` | [x] |

Coverage of the low-level surface: rows C1–C9 drive the *lowest-level* exported
entry point `driver` directly (not through `main`), and rows C10–C21 drive the
composed `scanf` → `driver` → `print_hex` pipeline through the exported `main`,
so both the primitive and the composed paths are exercised. `print_hex` is
`static` in C and is covered transitively by every row (16 bytes × all byte
classes in axis B).

### Where each row lives

| test file | rows |
|-----------|------|
| `tests/phase_b_driver.rs` | C1–C9 |
| `tests/phase_b_main.rs` | C10–C24, C29 |
| `tests/phase_b_repeated.rs` | C26–C28 |
| `tests/phase_c_process.rs` | C25 |
| `tests/phase_c_stdio.rs` | C30–C33 |
| `tests/coexistence.rs` | both libraries loaded in one process at once |

Every row above passes across its randomized inputs against both shared
libraries.
