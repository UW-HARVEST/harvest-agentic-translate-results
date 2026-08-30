# Differential verification log

C reference: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust program: `translation/src/main.rs`, built to `translation/target/release/driver`.
Tests: `translation/tests/differential.rs` — runs both binaries as subprocesses,
compares stdout, stderr and exit status byte for byte.

## Result

**No behavioural mismatch was found.** Roughly 3,000 input pairs were executed
against both binaries — the enumerated branch inputs, a power-of-two boundary
sweep, long digit runs, bulk input, and two seeded fuzzers — and every one
produced identical stdout, identical stderr (always empty) and identical exit
status (always 0).

The sections below record what was checked, because the value of this exercise
is the list of behaviours that were confirmed rather than assumed.

## Program behaviour, and why the output looks wrong but is right

`main` performs one `scanf("%d", &x)` with `x` initialised to 0, ignores the
return value, and branches on `if (x)`. There are exactly two reachable outputs:

| branch | stdout |
|---|---|
| `x == 0` → `bad()` | `fffffffe\n` |
| `x != 0` → `good()` | `04\ndata value is too large to perform arithmetic safely.\n` |

`bad()` sets `data = CHAR_MAX` (127). `data * 2` is evaluated at `int` width as
254, then narrowed to `char`, wrapping to -2. `printHexCharLine` takes a `char`,
which the variadic `printf` call promotes back to `int`, and `%02x`
reinterprets those 32 bits as `unsigned int` — so it prints `fffffffe`, not
`fe`, and the `02` minimum width has no effect. This is the original defect and
is reproduced deliberately.

The translation encodes this as `char_hex as i32` then `as u32` for formatting.
Printing the byte instead (`char_hex as u8`) yields `fe` and was confirmed to be
caught by the suite.

## Dead branches in the C (no input can reach them)

These were identified by reading the source; they are unreachable rather than
untested, and are pinned by `golden_outputs_match_c`:

- `printLine`'s `line != NULL` guard — the only call site passes a string
  literal, so the NULL path never runs.
- `if (data > 0)` in `bad()`, `goodG2B()` and `goodB2G()` — `data` is a
  compile-time constant (127, 2, 127) in each, so the guard is always true.
- The `if (data < (CHAR_MAX/2))` *then* branch in `goodB2G()` — `data` is 127
  and `CHAR_MAX/2` is 63 (integer division), so only the `else` ever runs.
  `goodB2G` therefore never prints a hex value at all.
- `goodB2G`'s `data = ' '` is dead: it is overwritten by `data = CHAR_MAX` on
  the next line. The Rust keeps the dead store behind `#[allow(unused_assignments)]`.

## The only input-dependent behaviour: `scanf("%d")`

Because every other branch is constant-folded, the entire input space collapses
onto how `%d` converts. That is where a translation can realistically diverge,
so it was tested exhaustively. Confirmed identical:

- **No conversion leaves `x` untouched.** Empty input, whitespace-only input, a
  lone `-` or `+`, `- 5`, `--5`, `+-5`, `.5`, letters, a leading NUL, a leading
  `0xff`, and a UTF-8 BOM all fail to convert, so `x` keeps its initialiser of 0
  and the program takes `bad()`. The ignored return value is what makes this
  observable.
- **Whitespace skipping** covers all six C-locale space characters, including
  vertical tab (`0x0b`) and form feed (`0x0c`), and reads across newlines.
- **Conversion stops at the first non-digit**, so `0x10` converts as just `0`
  (→ `bad()`) while `010` converts as decimal 10 (→ `good()`).
- **Saturate at `long`, then truncate to `int`.** The C library converts at
  `long` width, clamps to `LONG_MAX`/`LONG_MIN` on overflow, and the store into
  `int` truncates. This produces branch flips that a naive translation gets
  wrong:

  | input | long value | truncated `int` | branch |
  |---|---|---|---|
  | `4294967295` | 4294967295 | -1 | `good()` |
  | `4294967296` | 4294967296 | **0** | `bad()` |
  | `9223372036854775807` | `LONG_MAX` | -1 | `good()` |
  | `-9223372036854775808` | `LONG_MIN` | **0** | `bad()` |
  | `99999999999999999999` | saturates to `LONG_MAX` | -1 | `good()` |
  | `-99999999999999999999` | saturates to `LONG_MIN` | **0** | `bad()` |

  A translation that saturated into `int` instead of truncating would send
  `4294967296` to `good()`; that defect was confirmed to be caught.
- **Digit runs beyond any integer width** — up to 10,000 nines, in both signs,
  plus long leading-zero runs — saturate identically.

## Process error found during verification

One genuine error was in the throwaway shell harness used for the first
exploratory sweep, not in either program, and it is recorded because it produced
false passes:

- Cases with a leading `-` were generated with `printf "$input"`, and `printf`
  rejected values like `-2147483648` as command-line options. The input file was
  left holding the **previous** case's bytes, so seven negative-number cases
  reported `ok` while actually re-testing a stale input. Re-running them through
  a harness that passes input as data (not as a format argument) showed they do
  in fact match. The permanent suite in `tests/differential.rs` passes inputs as
  byte slices and cannot hit this class of bug.

## Suite validity

The assertions were checked against deliberately injected defects rather than
trusted. Killed: no sign extension in `%02x`; `if (x > 0)` in place of
`if (x != 0)`; the `goodB2G` comparison replaced with `data <= CHAR_MAX`;
saturating rather than truncating the store into `int`.

One injected change survived and should: removing the one-byte pushback after a
matching failure in the `%d` scanner. Nothing reads stdin after the single
`scanf`, so the position of the stream at exit is not observable through stdout,
stderr or exit status. It is an equivalent mutant, not a coverage gap.

No test is `#[ignore]`d, skipped or otherwise disabled, and nothing in `c_src/`
was modified.
