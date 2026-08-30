# Differential findings: `c_src/src/main.c` vs `translation/src/main.rs`

The C program is the ground truth. Everything below was found by building both
programs and comparing stdout, stderr and exit status for the same stdin.
Test suite: `translation/tests/differential.rs` (`cargo test`).

## Input space

`main` reads exactly one byte with `getchar()` and truncates it to `char`
(signed on x86-64 Linux), then `driver` prints 14 lines. The complete input
space that changes behaviour is therefore:

* EOF — no byte available (`c == -1`)
* each of the 256 possible byte values (`0x00..=0x7f` → non-negative `char`,
  `0x80..=0xff` → negative `char`)

All 257 classes are covered exhaustively by `every_single_byte` and
`empty_input_eof`; the remaining tests cover surrounding behaviour (extra input
bytes, unreadable stdin, closed stdout, argv, ambient locale).

## Mismatch 1 — exit status on a closed stdout (found, fixed)

**Symptom**

```
$ printf 'A' | { sleep 0.2; ./c_src/build/driver; echo "c=$?" >&2; } | true
c=141
$ printf 'A' | { sleep 0.2; ./translation/target/release/driver; echo "r=$?" >&2; } | true
r=0
```

stdout and stderr agreed (both empty), but the exit status did not: the C
program was killed by `SIGPIPE` (signal 13, shell status 141) while the Rust
program exited 0.

**Cause**

The Rust standard library installs `SIG_IGN` for `SIGPIPE` before `main` runs.
A C program keeps the default disposition, so its `write` from the stdio flush
at exit kills the process. In Rust the same `write` merely returned `EPIPE`,
which `main` discarded, so the process exited normally.

**Fix**

`translation/src/main.rs` now restores the default `SIGPIPE` disposition as the
first thing `main` does (`restore_default_sigpipe`, a direct `signal(13, SIG_DFL)`
call). Covered by `closed_stdout_dies_the_same_way`, which closes the read end
of the child's stdout pipe while the child is still blocked in `getchar()`, so
the race is deterministic.

## Verified, no mismatch

These are the places where the C program's behaviour is surprising enough that a
naive translation would diverge. The existing Rust code already reproduced them,
and the tests pin them down:

* **Classifiers return raw glibc class bits, not 0/1.** glibc's `<ctype.h>`
  macros expand to `(*__ctype_b_loc ())[(int) (c)] & _ISxxx`, so `printf("%d")`
  prints the bit itself: `isalpha('A')` prints `1024`, `isalnum('A')` prints `8`,
  `isupper('A')` prints `256`. A translation returning booleans would print `1`
  everywhere.
* **Bytes ≥ 0x80 become negative `char` values and classify as nothing.** glibc's
  table is defined for indices `-128..=255`; the negative half carries no class
  bits in the `C` locale, and `tolower`/`toupper` there are the identity on the
  raw byte. So `0xe9` prints `0` for all twelve classifiers and `0xe9` for both
  case conversions.
* **`0xff` is indistinguishable from EOF.** `EOF` is `-1`, and the byte `0xff`
  truncated to `signed char` is also `-1`, so empty input and input `"\xff"`
  produce byte-identical output (both print `0xff` for "to lower"/"to upper").
* **`printf("%c", tolower(c))` writes one raw byte,** not a UTF-8 encoding of a
  code point. For `0x80..=0xff` the output is invalid UTF-8, so the Rust side
  must write bytes (`Vec<u8>` / `write_all`), never a `String`.
* **Only the first byte is consumed.** `getchar()` reads one byte and the process
  exits; the rest of stdin is discarded, and a large unread input gives the
  *writer* a broken pipe, never the program under test.
* **Unreadable stdin behaves like EOF.** With stdin closed, `/dev/null`, or a
  directory (`read` → `EISDIR`), `getchar()` returns `EOF`, nothing is written to
  stderr, and the exit status is 0 in both programs.
* **`setlocale(LC_ALL, "C")` makes the ambient locale irrelevant.** Output is
  identical under `LC_ALL`/`LANG`/`LC_CTYPE` set to `C`, `C.UTF-8`, `POSIX`,
  `en_US.UTF-8`, `tr_TR.UTF-8`, `de_DE.UTF-8`, an invalid name, or empty.
* **Exit status is 0.** `main` has no `return`; C99 implicitly returns 0, which
  matches Rust's `fn main() -> ()`.
* **argv is ignored.** `int main()` declares no parameters.
* **No output goes to stderr** on any input.
