# Mismatches found while differentially testing the translation

Method: build `c_src` with `cmake .. && cmake --build .`, build this crate with
`cargo build --release`, then run both executables with identical argv and
identical stdin and compare stdout, stderr and exit status byte for byte.
`translation/tests/differential.rs` does this from `cargo test`; the sweeps
described below were run the same way.

Two real mismatches were found and fixed, plus one divergence that cannot be
reproduced faithfully and is documented instead.

---

## 1. `%d` matching failure discarded a sign that C had already consumed

**Symptom**

| stdin | C stdout | Rust stdout (before fix) |
| --- | --- | --- |
| `-\n` | `0000000000 \x03    \n` | `0000000000     -\n` |
| `+\n` | `0000000000 \x03    \n` | `0000000000     +\n` |
| `-x\n` | `0000000000 \x03    x\n` | `0000000000     -x\n` |
| `- 5\n` | `0000000000 \x03     5\n` | `0000000000      5\n` |
| `--5\n` | `0000000000 \x03    -5\n` | `0000000000     --5\n` |

**Cause**

`scanf("%d ", &time_stamp)` on input such as `-x` reads the `-`, then finds a
non-digit and reports a matching failure. glibc pushes back only the *one*
offending character (`x`); the sign and the whitespace already skipped for the
conversion stay consumed. The Rust `scan_int_then_space` was rewinding the
stream to the position it held before the conversion started, so the `-` was
still available and was later picked up by `%80[^\n]` as part of the comment.

**Fix**

`Scanner::scan_int_then_space` no longer restores `self.pos` on a matching
failure: it returns `0` leaving the sign consumed and the offending character
unread, which is exactly one character of pushback.

---

## 2. First record copied uninitialised buffers

**Symptom**

| stdin | C stdout | Rust stdout (before fix) |
| --- | --- | --- |
| `abc\n` | `0000000000 \x03    abc\n` | `0000000000     abc\n` |
| `10 aaa111 FL01 JFK LAX x\n` | `0000000010 \x03    aaa111 FL01 JFK LAX x\n` | `0000000010     aaa111 FL01 JFK LAX x\n` |

The C output carries an extra `0x03` byte where the luggage id belongs, i.e.
one field is one byte longer than in the Rust output.

**Cause**

In `main`, `luggage_id`, `flight_id`, `departure` and `arrival` are declared
inside the `while (1)` loop and never initialised (only `comments[0]` is
zeroed). A `%[...]` conversion that fails leaves its buffer untouched, but the
record is appended anyway, so `strcpy` copies whatever was in the buffer. On
the second and later iterations that is the previous record's value — the Rust
code already reproduced this by hoisting the buffers out of the loop. On the
*first* iteration it is whatever the process start-up code left in `main`'s
stack frame.

For the binary produced by the documented build (gcc without optimisation,
x86-64, glibc; `luggage_id` at `%rbp-0x8d`) that residue is a single `0x03`
byte followed by a NUL, and every other buffer starts out zeroed. Measured as
stable across repeated runs, across `PAD` environment variables of 1–5000
bytes, across argv lengths, with `env -i`, and with stdin from a file or a
pipe — the residue sits at a fixed offset from the initial stack pointer, so
it shifts together with the rest of the frame.

**Fix**

`INITIAL_LUGGAGE_ID_RESIDUE = 0x03` is written into `luggage_id[0]` before the
loop; all other buffers start zeroed.

**Caveat.** This is undefined behaviour in the C program. The value is a
property of the compiler, the C library and the dynamic loader, not of the
source, and a different toolchain will leave a different byte there — an `-O2`
build of the same source, for instance, leaves `\xa3y\xee\xfd\x7f` in that
slot. Only inputs whose *first* record fails its `%8[A-Z0-9]` conversion can
observe it.

---

## 3. Not reproduced: C stack overflow on very large inputs

`addRoutingDirectiveToList` and `supersedes` are recursive, one frame per list
element, 48 bytes per frame. With the default 8 MiB stack the C program
segfaults at roughly 174,700 records:

| records | C | Rust |
| --- | --- | --- |
| 170,000 | exit 0, output matches | exit 0 |
| 180,000 | killed by SIGSEGV (status 139), empty stdout and stderr | exit 0, full output |

The Rust translation uses iteration, so it keeps working. This is not
reproduced deliberately: the threshold depends on `RLIMIT_STACK` at run time,
on the compiler's frame size, and on how much of the stack the environment
block already occupies, so any hard-coded limit would either fire early on
inputs the C program still handles or fire late. Inputs up to 170,000 records
were verified to match exactly.

---

## Coverage that found no further differences

Every `if`, early `return` and conversion in `luggage.c` has at least one test:

- `argc != 5` for 0, 1, 2, 3, 4, 5 and 6 arguments, with and without stdin
- each of the four `scanf` calls returning `EOF`, i.e. end of file at every
  field boundary, both on the first record and after a complete one
- `%d`: matching failure, `+`/`-` signs, leading zeros, `2147483647`,
  `2147483648`, `4294967295`, `4294967296`, `LONG_MAX`/`LONG_MIN` saturation
  and 60-digit values, all printed through `%010u`
- `%8[A-Z0-9]`, `%6[A-Z0-9]`, `%3[A-Z]`: exact width, one over, zero-length
  matching failures, rejected lowercase, digits rejected by `%[A-Z]`,
  punctuation, NUL and bytes above 0x7f
- `%80[^\n]`: absent, empty, exactly 80, 81 and 200 characters, the leading
  blank that the format never skips, tabs, `\r`, `%`-sequences
- whitespace directives crossing newlines, `\t`, `\r`, `\v`, `\f`
- `addRoutingDirectiveToList`: empty list, append, prepend, insert in the
  middle, equal time stamps (input order preserved), unsigned ordering after
  truncation
- `supersedes` / `superseded`: no later record, later record with a different
  id, later record with the same id and the same departure, later record with
  the same id and a different departure (which stops the search), chains of
  three and four, and supersession decided by sorted rather than input order
- `matches`: `-` wildcard in each of the four positions, `-` followed by more
  characters, a dash that is not the first character, empty filter strings,
  exact matches, non-matches, and filters containing non-UTF-8 bytes
- buffer carry-over: a failed conversion reusing the previous record's field
- 500-record inputs ascending and descending, and 170,000-record inputs

Beyond the suite in `tests/differential.rs`:

- exhaustive over all inputs of length ≤ 3 from `{A, 1, a, -, space, \n, ?}`
  (400 cases) and length ≤ 6 from `{A, 1, -, space, \n}` (19,775 cases)
- 32,000 randomly generated inputs (four seeds), half free-form token soup
  over an alphabet including NUL, `\xff`, `\r`, `\v`, `\f` and oversized
  fields, half structured records with random field lengths, each run with
  four randomly chosen filter arguments

All of these agree on stdout, stderr and exit status.
