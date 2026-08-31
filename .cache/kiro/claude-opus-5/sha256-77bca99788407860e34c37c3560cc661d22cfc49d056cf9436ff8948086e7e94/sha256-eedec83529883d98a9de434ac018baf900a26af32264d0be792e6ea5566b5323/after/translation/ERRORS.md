# Differential verification log

Reference: `c_src/src/main.c` (built with CMake as `c_src/build/driver`).
Under test: `translation/src/main.rs` (built as `translation/target/release/driver`).

Method: both programs are spawned as subprocesses with byte-identical stdin;
stdout, stderr and exit status are compared byte for byte. See
`tests/differential.rs`. The Rust code is never loaded as a library.

## Mismatches found

**None.** Every enumerated input class, plus 6000 randomized inputs, produced
identical stdout, identical (empty) stderr and exit status 0 from both programs.

## What the C program branches on

The program has exactly one input-dependent decision:

```c
int x = 0;
scanf("%d", &x);   // return value ignored
driver(x);
return 0;
```

* On a successful conversion, `x` becomes the converted value truncated to `int`.
* On a **matching failure** or **EOF**, `scanf` does not store anything, so `x`
  keeps its initializer `0`.

`driver()` is unconditional and has no early returns: it zero-initializes
`house_t`, sets `floors`/`bedrooms`/`bathrooms`, and `print_hex` walks
`sizeof(house)` == 16 bytes emitting `%02x` each, then one `"\n"`. There is no
error path, no `stderr` output and no non-zero exit anywhere in the program, so
stderr is always empty and the status is always 0.

## Behaviors that were verified rather than assumed

These are the places where a naive translation would diverge. Each was checked
against the C binary; the Rust code already handled all of them.

| Behavior | Observed C result | Notes |
|---|---|---|
| Struct object representation | `floors`@0, `bedrooms`@4, `bathrooms`@8, size 16, **no padding holes** | e.g. input `1` → `01000000030000000000000000000040`. The `= {0}` plus full field assignment makes the whole image defined; `2.0` is `0x4000000000000000` little-endian. |
| `%d` skips whitespace, including newlines | `"\n\n\n   42\n"` → 42 | `scanf` reads *across* lines; a `fgets`-style line read would have failed here. |
| `%d` stops at the first non-digit | `42abc`→42, `3.9`→3, `12 34`→12, `1e9`→1, `1,234`→1 | Base 10 only, so `0x10` → **0** (the `x` terminates the conversion), and `010` → **10**, not octal 8. |
| Sign handling | `+5`→5, `-0`→0 | A sign with no following digit (`-`, `+`, `--5`, `+-5`, `- 5`, `-a`) is a matching failure → `x` stays 0. |
| Failed conversion keeps the initializer | `abc`, `_5`, `.5`, `!!!`, `""`, `"   "` → 0 | The return value of `scanf` is ignored, so failure is silent and still prints the `floors == 0` image. |
| Truncation to `int` | `4294967296` (2^32) → 0; `4294967297` → 1; `2147483648` → INT_MIN bytes; `2147483647999` → `ffffffff` | glibc converts with a 64-bit `long` and stores through an `int *`; the low 32 bits survive. |
| Out-of-`long` saturation | `9223372036854775808` and `"9"×100000` → `ffffffff` (i.e. `LONG_MAX` truncated to −1); the negative forms → `00000000` (`LONG_MIN` truncated to 0) | glibc's `%d` goes through `strtol`, which clamps to `LONG_MAX`/`LONG_MIN`. Formally UB in ISO C, but this is what the reference binary does, so the Rust code reproduces the clamp-then-truncate exactly. |
| Arbitrarily long digit runs | `"0"×100000 + "7"` → 7 | No width limit, so leading zeros never overflow. |
| Non-text stdin | `\x00`, `\xff\xfe`, `\x80\x819` handled without error | The Rust side treats stdin as raw bytes, so invalid UTF-8 cannot cause a panic or a decode error. |
| Unread stdin / closed stdin | identical output, exit 0, no hang | Only one conversion happens; the rest of the stream is discarded. Verified with `0<&-`, `/dev/null`, and 300 KB of trailing junk. |

## Test coverage

`cargo test` runs 17 test functions covering ~90 distinct inputs across the
classes above. No test is `#[ignore]`d, disabled or skipped. `tests/differential.rs`
builds the C reference with CMake on demand, so the suite is self-contained.

Nothing in `c_src/` was modified; the only addition there is the out-of-source
`c_src/build/` directory that `CMakeLists.txt` is designed to be configured into.
