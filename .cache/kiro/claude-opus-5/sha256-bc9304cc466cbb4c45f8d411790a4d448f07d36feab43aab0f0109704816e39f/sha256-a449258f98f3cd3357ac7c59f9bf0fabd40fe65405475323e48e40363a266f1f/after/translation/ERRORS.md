# Differential verification log — `container_of.c` → Rust

C ground truth: `c_src/src/container_of.c`
Rust under test: `translation/src/main.rs`
Test harness:    `translation/tests/differential.rs` (runs both binaries as
subprocesses; compares stdout bytes, stderr bytes, exit code and terminating
signal)

## Build commands

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
#   -> c_src/build/driver

# Rust
cd translation && cargo build --release
#   -> translation/target/release/driver

# Tests
cd translation && cargo test            # debug binary
cd translation && cargo test --release  # release binary (both green)
```

Run either program as: `driver <arg1> <arg2>`. Nothing is read from stdin.

## Result

**No behavioural mismatch was found.** Every input class enumerated below,
plus 4000 randomized argument pairs, produced byte-identical stdout, byte
identical stderr and an identical exit status (including identical terminating
signal) from both programs.

Because "no mismatch found" is only as strong as the tests behind it, the
suite was mutation-checked (see *Suite validation*) to confirm it actually
fails when the Rust program diverges.

## Input classes derived from the C source

`main` contains no `if`, no early `return` and no loop. All branching lives in
(a) glibc `atoi` on `argv[1]` / `argv[2]`, (b) the signed `int` addition, and
(c) whether `argv[1]` / `argv[2]` exist at all. Each row is an input class with
the C behaviour that must be reproduced.

| # | Input class | Example | C behaviour |
|---|---|---|---|
| 1 | Both args valid decimals | `1 2` | prints `a+b` and a newline, exit 0 |
| 2 | Zeros / negatives / mixed signs | `-1 -2` | plain `%d` output, exit 0 |
| 3 | Sum overflows `int` | `2147483647 1` | wraps to `-2147483648` |
| 4 | `int` boundaries exactly | `-2147483648 0` | exact, no clamping |
| 5 | Non-numeric | `abc def`, `0x10`, `1e5`, `--5`, `.5` | `atoi` yields 0, **no diagnostic, still exit 0** |
| 6 | Empty string / sign only / whitespace only | `"" ""`, `+ -`, `" " " "` | 0 |
| 7 | Leading whitespace skipped | `"  12"`, `"\t\n5"`, `"\v5"`, `"\f6"`, `"\r5"` | all six C-locale space chars skipped |
| 8 | Trailing garbage | `12abc` | parses the leading digits, stops silently |
| 9 | Leading zeros | `010`, `000…005` | **decimal**, not octal |
| 10 | Fits `long`, not `int` | `99999999999`, `4294967296`, `2147483648` | `long` truncated to `int` |
| 11 | Exceeds `long` | `9223372036854775808`, `-99999999999999999999` | `strtol` saturates to `LONG_MAX`/`LONG_MIN`, *then* truncates (→ `-1` / `0`) |
| 12 | `LONG_MIN` written exactly | `-9223372036854775808` | representable, no saturation |
| 13 | Very long digit strings | 400 `9`s, 400 `0`s + `5` | saturation / no overflow |
| 14 | Non-UTF-8 argv bytes | `\xff\xfe`, `5\xff` | argv is raw bytes; must not be rejected or lossily converted |
| 15 | More than two args | `1 2 3 4 5` | `argv[3..]` ignored |
| 16 | Only `argv[1]` given | `driver 7` | `atoi(argv[2])` reads the NULL argv terminator → **SIGSEGV**, no output, no exit code |
| 17 | No args given | `driver` | faults on `argv[1]` first → **SIGSEGV**, no output |
| 18 | Anything on stdin | `echo 999 \| driver 1 2` | stdin never read; output unchanged |

Classes 16 and 17 are the reason stderr and exit status are asserted and not
just stdout: a translation that printed nothing and exited 0 would pass a
stdout-only comparison.

## Aspects of the C that are deliberately reproduced, not "fixed"

These are the places a naive translation would silently disagree. Each is
covered by a named test.

1. **`atoi` = `(int) strtol(s, NULL, 10)`, saturate-then-truncate.**
   Out-of-range input does not clamp to `INT_MAX`. It clamps to `LONG_MAX`
   and then keeps the low 32 bits, so `99999999999999999999` becomes `-1`
   and `-99999999999999999999` becomes `0`. A translation using Rust's
   `str::parse::<i32>()`, or saturating at `i32` bounds, gets both wrong.
2. **Malformed input is not an error.** No message, no non-zero exit — just
   `0`. There is no validation in the C at all.
3. **Signed overflow of `a + b` wraps.** The C is UB here; the compiled
   behaviour is two's-complement wraparound, which the Rust matches with
   `wrapping_add`. Rust's default `+` would panic in a debug build.
4. **Missing arguments crash with SIGSEGV, they are not handled.** The C
   passes a NULL `argv` slot to `atoi`, which dereferences it. The Rust
   reproduces this with a volatile read from address 0 so the process dies
   from signal 11 with empty stdout and stderr, rather than printing a usage
   message or exiting non-zero.
5. **`argv` is bytes, not UTF-8.** The Rust uses `args_os()` + `OsStrExt`;
   `std::env::args()` would panic on the `\xff` cases.
6. **Output is exactly `%d` plus one `\n`** — no padding, no precision, no
   second newline, nothing on stderr.
7. **`container_of` offsets.** `offsetof(struct test, a) == 0` and
   `offsetof(struct test, b) == 4`; both recoveries must land on the same
   struct so the printed value is `a + b`. Using the wrong offset for `b`
   reads the neighbouring member and yields `a + a`.

## Suite validation (mutation check)

To prove the tests can detect divergence, five defects were injected into
`translation/src/main.rs` one at a time and `cargo test` was re-run. All five
were caught; `main.rs` was then restored and verified byte-identical to its
pre-mutation copy.

| Injected defect | Outcome |
|---|---|
| `find_container_of_b` uses offset of `a` | FAILED (12 tests) |
| `wrapping_add` → `saturating_add` | FAILED (3 tests) |
| missing arg → `exit(1)` instead of NULL deref | FAILED (3 tests) |
| drop the trailing `\n` from the `printf` | FAILED (15 tests) |
| `atoi` wraps instead of saturating at `long` | FAILED (3 tests) |

## Coverage not achieved

* **`argc == 0`** (exec'ing the binary with a completely empty `argv`). In the
  C this indexes `argv[1]` past a NULL `argv[0]`, which is undefined. It is
  unreachable from a shell, and `os.execv(path, [])` refuses to launch it, so
  the two programs could not be compared on this input. The Rust would take
  the same NULL-dereference path it takes for class 17.
* Everything else in the C source is reached: both `container_of` helpers,
  both `atoi` calls, the `memset`, the addition and the single `printf`.
