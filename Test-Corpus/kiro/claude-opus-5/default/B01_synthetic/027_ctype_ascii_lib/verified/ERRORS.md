# ERRORS.md — Error-surface table (Phase C gate)

## How this table was derived

The whole library is one 48-line translation unit. Mechanically grepping
`c_src/src/driver.c` for *every* rejection construct:

```sh
grep -n 'return\|RETURN_ERROR\|assert\|NULL\|errno\|exit\|abort\|if *(\|switch\|#if\|else' \
    c_src/src/driver.c
# -> no matches (outside of the copyright comment block)
```

Results, per construct:

| construct searched for | occurrences in `driver.c` |
|------------------------|---------------------------|
| `return` (any)         | 0 (`driver` is `void`)    |
| `return -1` / sentinel | 0                         |
| `return NULL`          | 0                         |
| `RETURN_ERROR`-style macro | 0                      |
| `assert` / `static_assert` | 0                      |
| explicit range check (`if`, `<`, `>`, `?:`) | 0 |
| null-pointer check     | 0 (no pointer parameters) |
| `switch` / `#ifdef` branch | 0 (only the header guard `DRIVER_H_`) |
| error enum / status type | 0                       |
| min/max constant       | 0                         |
| `errno` read or write  | 0                         |
| `exit` / `abort`       | 0                         |

So the library has **no explicit error surface**: `void driver(char c)` accepts
every one of the 256 `char` bit patterns, has no failure mode, no return value,
and no out-parameter. Its *entire* observable behaviour is the 14 lines it
`printf`s to `stdout`.

That does not end the phase. The rows below are the rejection/edge conditions
that *do* exist for this API — the implicit ones at the language and FFI
boundary, i.e. every input that a real caller can construct and for which the
two implementations could disagree. "Expected C result" is the ground truth the
Rust must reproduce byte-for-byte; where the C does not reject at all, the
required behaviour is *not rejecting in exactly the same way*.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` | `c == 0` (NUL, the C-string terminator / "empty" sentinel) | No rejection. 14 lines printed; classifiers report the raw `_IScntrl` bit, so `control: 2`, all other classes `0`; `to lower:`/`to upper:` emit a literal NUL byte (`%c` of `0`) |
| 2 | `driver` | `c == -1` i.e. `(char)0xFF`. Sign-extends to `-1`, which is the value of `EOF`; the ctype tables have a dedicated `EOF` slot at index `-1` | No rejection. All 12 classifiers `0`; `tolower(-1) == -1` and `toupper(-1) == -1`, and `printf("%c", -1)` writes the single byte `0xFF` |
| 3 | `driver` | `c == -128` i.e. `(char)0x80` — the most-negative `char`, the lowest legal index into glibc's ctype table | No rejection. Classifiers `0`; `tolower`/`toupper` return `128`, and `%c` of `128` writes the byte `0x80` |
| 4 | `driver` | any `c` in `-128..=-2` (high-bit set; `char` is **signed** on x86-64 Linux, so these index the ctype tables at a *negative* offset — out of the `0..=255` range the C standard defines for `isXXX`) | No rejection, no crash: glibc's tables are legally addressable from `-128`. Classifiers `0` for all of `-128..=-1`; `tolower`/`toupper` are the identity on `128..=254` |
| 5 | `driver` | `c == 127` (`DEL`) — one step past the last *printable* ASCII value and the largest positive `char` | No rejection. `control: 2`, everything else `0`; `%c` writes byte `0x7F` |
| 6 | `driver` | `c == 128` written by the caller as a wider integer (`128` is *not* representable in a signed `char`; implementation-defined narrowing) | No rejection. Narrows to `-128`, so output is identical to row 3 |
| 7 | `driver` | `c == 256`/`c == 0x100` passed as a wider integer — one step past the end of the `unsigned char` range | No rejection. Only the low 8 bits are consumed, so output is identical to `c == 0` (row 1) |
| 8 | `driver` | caller passes a full-width `int` with garbage in bits 8..31 (e.g. `0xDEADBE41`), the FFI analogue of an **out-of-range enum value**: the ABI passes sub-`int` args in a 32-bit register slot and the callee may not rely on the upper bits | No rejection. Callee sign-extends the low byte only (`movsbl`), so `0xDEADBE41` behaves exactly like `c == 'A'` (`0x41`) |
| 9 | `driver` | caller passes a negative wide `int` whose low byte is positive (e.g. `-65280 == 0xFFFF0100`) | No rejection. Low byte `0x00`, so identical to row 1 |
| 10 | `driver` | `c` is one of the values glibc classifies with *multiple* bits at once (`'0'..'9'` → `_ISalnum|_ISdigit|_ISxdigit|_ISgraph|_ISprint`) — the case where a normalising `0`/`1` translation would silently diverge | No rejection, and the returned `int` is the **raw masked bit**, not `1`: e.g. for `'0'`, `alphanumeric: 8`, `digit: 2048`, `hexadecimal: 4096`, `graphical: 32768`, `printing: 16384` |
| 11 | `driver` | `c == ' '` (0x20) — the one value that is `_ISblank|_ISspace|_ISprint` but **not** `_ISgraph` | No rejection. `space: 8192`, `blank: 1`, `printing: 16384`, `graphical: 0` |
| 12 | `driver` | `c == '\t'` (0x09) — `_ISblank` **and** `_IScntrl` and `_ISspace` simultaneously | No rejection. `control: 2`, `space: 8192`, `blank: 1`; `printing: 0` |
| 13 | `driver` | repeated invocation / locale already changed by an earlier caller (`driver` unconditionally calls `setlocale(LC_ALL, "C")` with no error check on its return value) | No rejection, and the return value of `setlocale` is discarded. Output is idempotent: the N-th call for a given `c` is byte-identical to the first |
| 14 | `driver` | `stdout` redirected to a non-tty (fully buffered) and never flushed by the callee — `driver` calls no `fflush` | No rejection. The 14 lines sit in the `stdout` FILE buffer; both implementations must share the *same* libc `stdout` buffer, so a caller's `fflush(NULL)` drains both identically |

No row is a real *rejection*, because the C rejects nothing — recording that
truthfully is the point. Every row is exercised by a differential test in
`tests/errors.rs` that asserts the two `.so`s produce byte-identical output
(and, for the wide-integer rows, identical narrowing).

## Status

All rows have a passing differential test in `tests/errors.rs`.

| # | test | result |
|---|------|--------|
| 1 | `err_01_nul_byte` | [x] pass |
| 2 | `err_02_minus_one_eof_slot` | [x] pass |
| 3 | `err_03_most_negative_char` | [x] pass |
| 4 | `err_04_all_negative_chars` | [x] pass |
| 5 | `err_05_del_127` | [x] pass |
| 6 | `err_06_128_narrowing` | [x] pass |
| 7 | `err_07_256_one_past_uchar` | [x] pass |
| 8 | `err_08_garbage_high_bits` | [x] pass |
| 9 | `err_09_negative_wide_int` | [x] pass |
| 10 | `err_10_multi_bit_classes_raw_mask` | [x] pass |
| 11 | `err_11_space_is_blank_not_graph` | [x] pass |
| 12 | `err_12_tab_is_blank_and_cntrl` | [x] pass |
| 13 | `err_13_repeated_calls_idempotent` | [x] pass |
| 14 | `err_14_shared_stdout_buffer` | [x] pass |

Plus the generic boundaries the task calls out even though they are not table
rows:

| boundary | test | result |
|---|---|---|
| null-pointer / length parameters | `generic_boundaries_no_pointer_or_length_params` — asserts mechanically, from `driver.h`, that the API has neither, then checks totality over the whole parameter domain | [x] pass |
| one step past every documented range | `generic_boundaries_one_step_past_ranges` | [x] pass |
| out-of-range value crossing FFI (the enum analogue) | `err_08_garbage_high_bits` — 512 seeded-random 32-bit arguments plus adversarial patterns | [x] pass |

### Bug this phase found

Row 8 was not a formality. Passing `0xDEAD_BE41` through the `driver` symbol
**segfaulted the Rust** while the C printed the results for `'A'`:
`extern "C" fn driver(c: c_char)` makes rustc mark the parameter `signext` and
emit `movslq %edi`, so the *whole* 32-bit register became the ctype-table index.
GCC's code keeps only `%al` (`mov %al,-0x4(%rbp)`) and re-reads it with `movsbq`.
Fixed by taking the argument as `c_int` and narrowing explicitly. See
`CONFIGS.md` for the full list of the three bugs verification uncovered.
