# Differential verification log

The Rust crate in this directory is a port of `../c_src` (`src/main.c` +
`src/lib.c`, built by `c_src/CMakeLists.txt` as the executable `driver`). The C
program is the ground truth; this file records every divergence found while
comparing the two **executables** and what caused it.

## How the comparison was done

Both programs were built and driven as subprocesses, with the same bytes on
stdin, comparing stdout, stderr and the exit status (including the terminating
signal):

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # ./driver
cd translation && cargo build --release                                # target/release/driver
cd translation && cargo test                                           # runs the diff suite
```

`translation/tests/differential.rs` holds 164 enumerated cases; the harness is
`translation/tests/harness/mod.rs`. Beyond that suite the two binaries were
compared over roughly 45 000 additional generated inputs: an exhaustive
op × flags × string-pool matrix, a length sweep from 0 to the 1024-byte maximum,
a random structured fuzz over buffer contents, a fuzz over the numeric token
stream (signs, leading zeros, overflow at every integer boundary), and a raw
random-byte fuzz. No divergence remains on any input for which the C program is
itself reproducible.

## Mismatch 1 — `%zu` / `%u` overflow with a leading minus sign

**Symptom.** stderr differed for inputs whose length field overflowed
`unsigned long`:

```
stdin:  0 0 -99999999999999999999999999
C   :   Error: input length 18446744073709551615 exceeds maximum 1024   (exit 1)
Rust:   Error reading input byte 0                                      (exit 1)
```

The Rust program produced a completely different error message — and for other
seeds a different one again — because the bogus length landed on a small value
instead of `ULONG_MAX`, so it passed the `> MAX_BUFFER_SIZE` check and the
program went on to read buffer bytes.

**Cause.** `Scanner::scan_ulong` modelled `strtoul` as "clamp the magnitude,
then apply the sign". glibc does the opposite: as soon as the digit string
overflows it sets `ERANGE` and returns `ULONG_MAX` *without* applying the
leading `-`. Only values that actually fit get negated. So `-1` → `ULONG_MAX`
(negation of 1) but `-99999999999999999999999999` → `ULONG_MAX` as well
(saturation, no negation), whereas the old code computed
`0 - ULONG_MAX == 1`.

**Fix.** `translation/src/main.rs`, `Scanner::scan_ulong`: return `u64::MAX`
immediately when the digit accumulation overflowed, and only wrap the sign in
for magnitudes that fit. `%d` was already correct — glibc's signed path
saturates to `LONG_MIN` / `LONG_MAX` and *is* sign-aware, and the port then
truncates to `int`, which is why `99999999999999999999` reads back as `-1`.

**Regression cover.** `main_scanf_error_paths::err_input_len_overflow_ulong`,
`err_ref_len_overflow_ulong`, `main_scalar_edge_cases::scan_flags_overflow_ulong`,
`scan_op_neg_long_saturates`, plus `scanf_integer_semantics` in `src/main.rs`.

## Mismatch 2 — stdin was consumed eagerly instead of on demand

**Symptom.** Not visible in stdout/stderr/exit status for a finite pipe, but the
Rust program blocked where the C program did not: `scanf` stops reading the
moment its conversions are satisfied, so the C `driver` answers immediately even
if the writer keeps the pipe open, while the Rust `driver` sat in
`read_to_end` waiting for EOF.

**Cause.** `Scanner::new` called `stdin().read_to_end()` up front.

**Fix.** `Scanner` now buffers stdin incrementally (`Scanner::fill`, 4 KiB at a
time) and only pulls more bytes when the next byte of lookahead is not already
buffered. Verified by holding the pipe open after a complete input: both
binaries print the result and exit at the same point.

## Inputs where the C program is not reproducible (excluded, by design)

`main()` declares `char input_buffer[1024]` and `char ref_buffer[1024]` and
fills only the first `input_len` / `ref_len` bytes. `lib.c` then calls
`strcmp`, `strlen` and `strncmp` on them — the comments in the C source flag
these as `VULNERABLE`. When the declared length contains **no NUL byte**, those
calls read pre-`main` stack residue left by the dynamic loader and libc
start-up. Part of that residue is saved pointers, whose bytes change with ASLR,
so **the C binary returns different answers on different runs of the same
input**. Measured on the reference build:

| stdin | C results over 200 runs |
|---|---|
| `0 0 1 0 0` (op 0, `input=""`, `ref_len=0`) | `0` ×183, `1` ×17 |
| `4 0 6 80 65 85 83 69 0 0` (op 4, `"PAUSE"`, `ref_len=0`) | `0` ×191, `5` ×9 |
| `4 3 64 <63×'A'> 0 0` | `0` ×190, `10` ×10 |
| `4 2 0 0` (op 4, both lengths 0) | `0` ×178, `10` ×18, SIGSEGV ×4 |

No translation can be byte-identical to a program that is not byte-identical to
itself, so these inputs are deliberately **not** asserted on. The rule used to
classify a case is mechanical: an input is reproducible iff every buffer the
operation dereferences as a string contains a NUL byte within its declared
length. All 164 cases in `tests/differential.rs` satisfy that rule and were each
confirmed stable over 30 consecutive runs of the C binary before being admitted.

`src/stack_residue.rs` models the residue so that the port still behaves
plausibly on these inputs — it reproduces the modal C answer in the cases
measured above — but its ASLR-dependent bytes cannot be matched exactly, and
nothing in the test suite depends on them.

Two `parse_command` cases (`op1_stop_unterminated`, `op1_resume_unterminated`)
do read residue and were nonetheless stable over 80 runs, because the bytes they
touch fall in a zero-filled part of the start-up frame rather than on a saved
pointer. They are kept as in-crate model tests in `src/main.rs`, not as
differential assertions, so that an environment with a different start-up layout
cannot turn the differential suite flaky.

## Behaviour deliberately preserved (checked, no change needed)

* **`size_t` underflow in `match_pattern`.** With `flags & 0x02` set and a
  pattern longer than the text, `for (size_t i = 0; i <= text_len - pattern_len; i++)`
  underflows to a huge bound and `strncmp` walks off the end of the buffer. Both
  binaries die with SIGSEGV (exit status 139 through a shell). Verified stable
  over 110 text/pattern length combinations —
  `op4_match_pattern::op4_cs_pattern_longer_underflow*`.
* **`strncpy` + `strncat` truncation in `compare_prefix`.** For a prefix of 63
  bytes or more, `strncpy(expected, prefix, 63)` leaves no room, `strlen(expected)`
  is 63, and `strncat(..., 63 - 63)` appends nothing — so none of the `_v1`,
  `_v2`, `_old`, `_new`, `_tmp` variations can ever match. Covered at prefix
  lengths 60/61/62/63/70 and at the 1024-byte maximum.
* **`snprintf` truncation to 63 bytes** for the three wildcard patterns in
  `match_pattern` — `op4_cs_long_pattern_wildcard_trunc`,
  `op4_cs_pattern_62_wild_both`.
* **Flag masking.** Only bit 0 selects exact matching (op 2) and only bit 1
  selects case sensitivity (op 4); every other bit, including
  `flags == 0xFFFFFFFF`, is ignored. Covered per operation.
* **`(char)byte` truncation** of each buffer byte read with `%u`: `321` stores
  `'A'`, `256` stores `0`, `-191` stores `'A'`. `operation` is likewise a
  `strtol` result truncated to `int`, and `flags` a `strtoul` result truncated to
  `unsigned int`.
* **`find_delimiter` with a NUL delimiter** (`ref[0] == 0`): the `data[i] == delim`
  test runs before the `data[i] == '\0'` break, so the function returns the
  index of the first NUL rather than `-1`.
* **`find_delimiter` scans `input_len` bytes, not the string.** A NUL before the
  delimiter breaks the loop, and the `"NONE"` / `"EMPTY"` special cases are only
  reachable for delimiters `|` and `:` respectively.
* **Order of the two length checks.** `input_len` is validated (and reported)
  before `ref_len` is read at all, so a payload with both lengths out of range
  reports only the input one.
* **`process_strings`' `-1` and `-2` returns are unreachable** from this
  `main()`: `input_buffer` and `ref_buffer` are arrays, never `NULL`. The port
  keeps the branches (modelled as `Option::None`) so the translation stays
  faithful, but no stdin can reach them. The `-2` seen in test output comes from
  `find_delimiter`'s `"NONE"` special case, not from the NULL check.
