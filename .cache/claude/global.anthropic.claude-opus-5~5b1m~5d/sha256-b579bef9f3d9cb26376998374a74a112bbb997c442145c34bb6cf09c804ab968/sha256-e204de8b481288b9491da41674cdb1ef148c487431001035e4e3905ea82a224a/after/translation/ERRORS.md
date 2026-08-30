# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

## How the two programs are run

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |

Both read **one byte** from stdin and write 14 lines to stdout. The complete
input space of the program is therefore `{EOF} ∪ {0x00..=0xFF}` for the first
byte; everything after byte 0 is unread. The suite in
`tests/differential.rs` runs both binaries as subprocesses and compares
**stdout, stderr and exit status** for all 257 of those classes plus the I/O
edge cases below.

Platform of record: x86-64 Linux / glibc, where `char` is **signed**.

## Result

**No output mismatch was found.** All 257 first-byte classes and every I/O edge
case produce byte-identical stdout, byte-identical stderr and identical exit
status (`0`). `cargo test` passes 16/16 in both the debug and release profiles.

The sections below record the traps that make this program much harder to
translate than it looks. Each was verified empirically against the compiled C —
not reasoned about — and each is covered by a test.

---

## Trap 1 — `is*()` return the glibc **bitmask**, not `1`

`printf("%d", isalpha(c))` does not print `0`/`1`. glibc implements the
classifiers as a table lookup masked with an `_ISbit` value:

```c
#define __isctype(c, type) ((*__ctype_b_loc ())[(int) (c)] & (unsigned short) type)
```

so the *nonzero* result is the bit itself. Confirmed with input `A`:

```
alphanumeric: 8        alphabetic: 1024      uppercase: 256
hexadecimal: 4096      graphical: 32768      printing: 16384
```

A translation that returned `1` from these would be "logically correct" and
still wrong on every printed line. The Rust models the actual `_ISbit` values
(`IS_UPPER = 1<<8`, `IS_ALNUM = (1<<11)>>8 = 8`, …).

*Harness validation:* I temporarily changed `isalnum` to return `1` instead of
the mask `8`. **9 of the 16 tests failed**, which is the evidence that this
suite detects real divergence rather than merely passing. The mutation was
reverted; `isalnum` is back to `ctype_b(c) & IS_ALNUM` and the suite is green.

## Trap 2 — `char c = getchar()` truncates, and the byte can go **negative**

`getchar()` returns `int`, but it is stored in a `char`. For input bytes
`0x80..=0xFF` the value becomes negative (`0x80` → `-128`, `0xE9` → `-23`), and
`isalpha(c)` then indexes glibc's ctype table at a **negative** index. This is
legal in glibc (the table covers `-128..=255`), and in the `"C"` locale the
negative half is all zeroes — so every classification reports `0` for high
bytes. Verified for all of `0x80..=0xFF`.

`tolower`/`toupper` on those negative indices return the value **unchanged**,
so `%c` reprints the original byte:

```
$ printf '\xe9' | driver     # …to lower: \xe9   to upper: \xe9
```

## Trap 3 — EOF and byte `0xFF` are **indistinguishable**

`getchar()` returns `EOF == -1`; `(char)(-1) == -1`, which is exactly what
input byte `0xFF` truncates to. Empty input and `printf '\xff'` therefore
produce **identical** output — a genuine information loss in the C that the
translation must reproduce rather than repair. Asserted explicitly in
`negative_char_range`.

## Trap 4 — `%c` with a negative argument

`printf("to lower: %c\n", tolower(c))` converts the `int` to `unsigned char`.
For EOF/`0xFF` the argument is `-1`, and `%c` emits byte `0xFF`, not an error
and not a truncated multibyte sequence. The Rust does `(value as u32 & 0xff) as u8`.

## Trap 5 — only the first byte is read

`getchar()` reads exactly one byte. It is not `scanf` (which would skip leading
whitespace and read across newlines) and not `fgets` (which would stop at a
newline). So a leading `'\n'`, a leading `'\0'`, `"12 34"`, and a 300 KB payload
all report on their **first** byte only, and the unread remainder is discarded
at exit. Covered by `only_first_byte_is_consumed`, `large_input` and
`all_256_first_bytes_with_trailer` (all 256 bytes each followed by a trailer).

## Trap 6 — the locale is pinned by the program, not the environment

`driver()` calls `setlocale(LC_ALL, "C")` itself, so `LC_ALL=en_US.UTF-8` in the
environment must **not** change the classifications. Verified in
`locale_env_does_not_matter`.

## Trap 7 — I/O failures are silent, and the exit status is always 0

`main()` has no `return` statement, so it falls off the end and exits `0`. There
is no error path anywhere in this program: no validation, no `exit(1)`, no
`fprintf(stderr, …)`. Consequently *stderr must always be empty and the status
must always be 0*, including when:

- stdin is closed / `/dev/null` → `getchar()` yields EOF, the EOF report is printed
- **stdin is a directory** → `read(2)` fails with `EISDIR`, `getchar()` still yields EOF
- stdout is closed or `/dev/null` → `printf`'s failure is ignored

This is the one place a naive Rust port diverges loudly: `println!` **panics**
on a broken pipe ("failed printing to stdout"), which would write to stderr and
exit `101` where the C exits `0`. The translation avoids this by writing to a
locked `stdout` with `write_all` and discarding the `Result` via `let _ =`.
Covered by `stdin_closed`, `stdin_is_a_directory` and `stdout_to_devnull`.

---

## Change made to the Rust during verification

One change, in `getchar()`: the read loop now retries on
`ErrorKind::Interrupted` instead of treating `EINTR` as EOF. C's `getchar()` is
restarted after a signal, so the previous catch-all `_ => -1` could have
reported a spurious EOF under signal delivery. This was a latent fidelity gap
reasoned from the C semantics, **not** an observed output mismatch — no test
input triggered it. All other error kinds still return `-1`, matching `getchar()`
reporting EOF on a read error (Trap 7).

## Test inventory (16 tests, none `#[ignore]`d)

| test | input class |
|---|---|
| `both_binaries_run` | Phase A: both binaries built and runnable |
| `output_shape_is_pinned` | exact 14-line byte layout for `A` |
| `empty_input_eof` | empty input → EOF |
| `single_byte_inputs` | one item: representative single bytes |
| `all_256_first_bytes` | **exhaustive** `0x00..=0xFF` |
| `control_characters` | `NUL`, `0x07..=0x0e`, `0x1f`, `0x7f` |
| `class_boundaries` | every ASCII class transition edge |
| `negative_char_range` | high half + the EOF/`0xFF` collision |
| `only_first_byte_is_consumed` | trailing bytes ignored; leading `\n`, `\0`, blanks |
| `large_input` | 300 KB and 100 KB binary (max the code "handles") |
| `all_256_first_bytes_with_trailer` | exhaustive sweep with trailing data |
| `stdin_closed` | `/dev/null` stdin |
| `stdin_is_a_directory` | `read()` fails with `EISDIR` |
| `stdout_to_devnull` | write-failure path stays silent, status 0 |
| `extra_argv_is_ignored` | `main()` takes no parameters |
| `locale_env_does_not_matter` | `LC_ALL=en_US.UTF-8` |

Nothing in `c_src/` was modified; the only addition there is the untracked
`c_src/build/` directory produced by CMake.
