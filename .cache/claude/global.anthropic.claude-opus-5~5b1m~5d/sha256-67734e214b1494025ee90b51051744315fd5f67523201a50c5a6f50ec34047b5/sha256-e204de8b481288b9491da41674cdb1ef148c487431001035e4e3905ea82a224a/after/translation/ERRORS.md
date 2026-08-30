# Differential verification log — `c_src` (C, ground truth) vs `translation` (Rust)

## How the two programs are run

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |

Both read a single line from **stdin**, take no meaningful arguments, and always
exit `0`. The test suite (`translation/tests/differential.rs`) spawns each as a
subprocess, pipes identical stdin, and compares **stdout, stderr and exit
status** byte for byte. The Rust code is never loaded as a library.

## Outcome

**No behavioral mismatches were found.** Phase A found no compile errors
(`cargo build --release` succeeded unmodified), and every input class enumerated
in Phases B and C produced byte-identical stdout, stderr and exit status.

To confirm that this is genuine agreement and not an insensitive test suite, the
suite was validated by *mutation testing* (see below) and by an out-of-band fuzz
of **2000** pseudo-random inputs against the release binary — all identical.

`c_src/` was not modified. The only addition under `c_src/` is the generated
`build/` directory produced by the prescribed CMake commands.

## Input classes enumerated from the C source

`main()` has exactly one branch — `if (parse_val(in, &x))` — but the behavior is
driven by `fgets` and `strtol` semantics, which fan out into these classes:

| Class | Example input | Reaches |
|---|---|---|
| Immediate EOF (`fgets` returns NULL, `in` stays `""`) | *(empty)* | error path |
| Whitespace only (`strtol` skips it, finds no digits) | `"\n"`, `" \t\v\f\r"` | error path |
| Leading garbage → `endp == str` | `"abc"`, `".5"`, `"--5"`, `"- 5"`, `"-"`, `"+"` | error path |
| Trailing garbage → accepted, `endp != str` | `"12abc"`, `"1.9"`, `"0x10"`, `"7,8,9"` | success path |
| Valid in `int` range | `0`, `1`, `-7`, `+7`, `2147483647`, `-2147483648` | success path |
| In `long` range but outside `int` range | `2147483648`, `-2147483649`, `LONG_MAX` | error path (range check) |
| `strtol` sets `ERANGE` | `9223372036854775808`, `-…809`, 30-digit numbers | error path |
| Signed `int` overflow in `bedrooms += x` | `2147483647`, `-2147483648` | success path, wraps |
| `fgets` 99-byte truncation | 99/100/200 digits; 97–99 spaces then a number | both paths |
| Embedded `NUL` (buffer is zero-initialized) | `"\0 5"`, `"5\0abc"`, `"12\0 34"` | both paths |
| `fgets` does **not** read past the first newline | `"5\n9\n"`, `"abc\n9\n"` | both paths |
| Non-ASCII / high bytes | `"héllo"`, `"\xff\xfe"`, `"9\xff"` | both paths |

## Subtleties that had to match, and do

1. **`fgets`, not `scanf`.** `fgets(in, 100, stdin)` stops *after* the first
   newline and reads at most 99 bytes. A second line is never read. The Rust
   `fgets_line(100)` reproduces both limits (`while buf.len() + 1 < size`, break
   on `\n`). Confirmed by `fgets_stops_at_first_newline` and
   `fgets_buffer_boundary`.
2. **Truncation changes the parse result.** With 99 leading spaces the digits
   fall outside the buffer, so a "valid" number becomes the error path. With 98
   spaces exactly one digit survives. Both replicated.
3. **`strtol` accepts trailing garbage.** The check is only `endp != str`, so
   `"1.9"` yields `1` and succeeds, while `".5"` fails. Replicated exactly —
   notably this is why Rust's `str::parse` cannot be used here (see mutant #9).
4. **`long` is 64-bit** on this target, so the `INT_MIN`/`INT_MAX` bounds check
   is a real, separate filter from `ERANGE`. `2147483648` is rejected by the
   bounds check, not by `ERANGE`.
5. **Signed overflow.** `bedrooms += extra_bedrooms` overflows for
   `x = INT_MAX`. The C is built with no optimization flags (`CMakeLists.txt`
   sets none, and `CMAKE_BUILD_TYPE` is empty), so it wraps two's-complement;
   the Rust uses `wrapping_add` to match. Without this, Rust debug builds would
   panic and release builds could diverge.
6. **`%.1f` formatting.** Only `2.5 / 3.5 / 4.5` ever print — exactly
   representable, so `{:.1}` and `%.1f` agree with no rounding-mode risk.
7. **`run()` is called twice** and mutates `the_house` in place, so the second
   call continues from the first call's final state (floors 4, bathrooms 4.5).
8. **Exit status is always 0**, including on the error path. A translation that
   used `exit(1)` for "An error occurred" would pass a stdout-only test but is
   caught here (mutant #4).

## Suite sensitivity (mutation testing)

Eleven deliberate bugs were injected into `translation/src/main.rs`; the source
was then restored and re-verified identical to the original.

| # | Injected bug | Caught? |
|---|---|---|
| 1 | `saturating_add` instead of `wrapping_add` for bedrooms | ✅ 3 tests |
| 2 | `fgets_line(101)` — buffer off-by-one | ✅ `fgets_buffer_boundary` |
| 3 | drop the `!erange` check | ❌ **equivalent** (see below) |
| 4 | `exit(1)` on the error path | ✅ 3 tests |
| 5 | drop `buf.truncate(end)` (no NUL handling) | ❌ **equivalent** (see below) |
| 6 | `{:.2}` instead of `{:.1}` | ✅ 3 tests |
| 7 | don't stop at newline (scanf-like) | ✅ 3 tests |
| 8 | drop the `INT_MIN`/`INT_MAX` range check | ✅ 4 tests |
| 9 | naive `str::parse::<i64>()` instead of strtol semantics | ✅ 6 tests |
| 10 | no leading-whitespace skip | ✅ 6 tests |
| 11 | `floors: 1`; and `run()` called once | ✅ 15 tests |

### The two survivors are semantically equivalent, not coverage gaps

- **#3 (`!erange`)** — when `strtol` sets `ERANGE` it returns `LONG_MAX` or
  `LONG_MIN`, which are *always* outside `[INT_MIN, INT_MAX]`. The range check
  therefore already rejects every `ERANGE` input, making the `errno` test
  redundant *for the observable outcome*. No input can distinguish the two, so
  no test can. The check is kept because the C keeps it.
- **#5 (NUL truncation)** — `strtol` only ever inspects leading whitespace, an
  optional sign, and digits. `NUL` is none of those, so parsing halts at a
  `NUL` byte whether or not the buffer was truncated there. Again no
  distinguishing input exists. The truncation is kept because it makes the
  C-string semantics explicit.

Both are documented rather than removed: they are correct code that happens to
be unobservable, which is different from untested code.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test
```

21 tests, all passing, none `#[ignore]`d, skipped or disabled.
