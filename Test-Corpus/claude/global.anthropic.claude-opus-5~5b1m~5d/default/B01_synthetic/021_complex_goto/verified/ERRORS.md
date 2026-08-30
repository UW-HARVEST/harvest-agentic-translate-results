# Differential findings: C (`c_src/src/main.c`) vs. Rust (`translation/src/main.rs`)

The C is the ground truth. Everything below was found by running both executables
on the same stdin and diffing stdout, stderr and exit status
(`translation/tests/differential.rs`).

Commands used:

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
# Rust
cd translation && cargo build --release                                 # -> translation/target/release/driver
```

Both programs read two integers with a single `scanf("%d %d", &x, &y)` and print
only to stdout. Neither ever writes to stderr and both always exit 0 (`main`
returns 0 unconditionally; there is no error path, no `exit()`, no `abort()`).

---

## Mismatch 1 — negative `scanf` overflow produced the wrong value (`x`/`y` off by one)

**Input:** `-99999999999999999999 4` (any negative literal whose magnitude exceeds
`LONG_MAX`)

**C output:** `loop / y / y / y / y` (4 `y` lines, 12 bytes)
**Rust output (before fix):** 14 bytes — an extra `x` line and a different shape

**Cause.** glibc implements `%d` on top of `strtol`. On range error `strtol`
*clamps* to `LONG_MAX` / `LONG_MIN` and the resulting `long` is then truncated to
`int`. The two directions are therefore asymmetric:

| literal | `strtol` result | stored `int` |
|---|---|---|
| `99999999999999999999` | `LONG_MAX` = `0x7FFF…FF` | `-1` |
| `-99999999999999999999` | `LONG_MIN` = `0x8000…00` | `0` |

The Rust accumulated into a *signed* `i64` saturating at `i64::MAX` and then
applied `wrapping_neg()`, giving `-i64::MAX` = `0x8000…01`, which truncates to
`1` instead of `0`.

That one-off matters enormously here, because `foo` has the special guard
`if (x == 1 && y == 4) goto label2;`. With input `-99999999999999999999 4` the C
takes the ordinary `label1` path with `x == 0`, while the Rust hit `x == 1 &&
y == 4` and jumped to `label2` — a completely different trace.

**Fix.** Accumulate the *magnitude* into a `u64` with an explicit overflow flag,
then reproduce `strtol`'s clamping before the truncation to `i32`:
`LONG_MIN` for negative overflow (magnitude `> 2^63`), `LONG_MAX` for positive
overflow. Note `-9223372036854775808` is exactly `LONG_MIN` and is *not* an
overflow; it is handled by comparing against `2^63` rather than `2^63 - 1`.

Verified against C for: `99999999999999999999`, `9223372036854775808`,
`18446744073709551616`, `-99999999999999999999`, `-9223372036854775809`,
`-9223372036854775808`, `9223372036854775807`, `4294967296`, `-4294967297`,
`2147483648`, `68719476736`, in both operand positions.

---

## Mismatch 2 — `SIGPIPE` was ignored, so the Rust would not die with its reader

**Input:** any input driving a long run (e.g. `2147483647 0`) with a stdout reader
that closes early, as in `./driver | head -c 2000000`.

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main`, whereas a C program keeps the default disposition. The translation also
discards write errors (`let _ = write!(…)`), matching C's ignoring of `printf`'s
return value — but combined with `SIG_IGN` that meant the Rust process kept
grinding through all ~2^31 remaining iterations after its reader vanished, while
the C was killed immediately. Observable as a wildly different exit status and a
~40 s hang per case.

**Fix.** Restore `signal(SIGPIPE, SIG_DFL)` at the top of `main`
(`restore_default_sigpipe`).

---

## Confirmed-matching behaviours (checked, not changed)

These looked like candidates for divergence and were each verified identical:

- **`scanf` reads across newlines.** `1\n4`, `1\n\n\n4`, `1\r\n\t 4` all parse as
  `x=1, y=4`; the `%d` conversions skip arbitrary whitespace. (Not `fgets`
  semantics — a line-oriented reader would have got this wrong.)
- **Partial conversion leaves variables at their initialisers.** `scanf` stops at
  the first failure, so `1 abc` gives `x=1, y=0` and `abc 4` gives `x=0, y=0` —
  the `4` is never reached because the *first* conversion already failed.
- **One character of push-back on matching failure.** For `- 4`, glibc consumes
  the `-`, fails, and pushes back only the space; the `-` is lost. Emulated with
  the single-byte push-back in `Scanner`.
- **`0x10` → `x=0`, `1e5` → `x=1`.** `%d` is decimal-only; no hex or exponent.
- **Leading zeros / signs:** `007 008` → `7, 8`; `+3 +4` → `3, 4`.
- **Empty and whitespace-only stdin:** `scanf` returns `EOF`, `x` and `y` stay `0`,
  `while (x > 0 || y > 0)` is false, nothing is printed, exit 0.
- **The `goto` control flow**, including that `goto label2` skips the `x`
  decrement, that `goto label1` re-enters the body *without* reprinting `loop`,
  and that `continue` jumps to the `while` condition rather than to `label1`.
  Verified by an exhaustive sweep of `x, y ∈ [-3, 12]`.
- **Signed wrap-around of `y--`.** When `x > 0` and `y < 0` the C decrements `y`
  past `INT_MIN` and wraps (UB in ISO C; gcc at `-O0` wraps). The Rust uses
  `wrapping_sub` so it behaves identically *and* independently of the build
  profile — a plain `y -= 1` would panic under `cargo test`'s debug
  overflow checks while wrapping in release. Both profiles are exercised by the
  test suite.
- **Output buffering.** stdout only; the Rust `BufWriter` is flushed before exit,
  so the byte stream matches C's fully-buffered stdio flush at exit.

---

## Inputs that cannot be compared in full, and how they are covered

`foo` iterates once per unit of `x`/`y`, so operands near `INT_MAX` emit multiple
gigabytes (`2147483647 0` is ~3.4 GB of `loop\nx\n`; `1 99999999999999999999`
sets `y = -1` and wraps all the way around for ~8 GB). Capturing those in full
exhausts memory. `huge_output_prefixes_match` compares a 2 MiB stdout prefix —
still hundreds of thousands of iterations of every branch — and then kills both
children. Everything else is compared in full, including stderr and exit status.
