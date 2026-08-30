# Differential testing log: `c_src` (ground truth) vs `translation`

## How the two programs are run

```bash
# C reference
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver            # reads stdin

# Rust
cd translation && cargo build --release
./translation/target/release/driver   # reads stdin
```

Both are compared as executables: same bytes on stdin, then stdout, stderr and
exit status are diffed. The suite lives in `translation/tests/differential.rs`
and spawns both binaries as subprocesses (`cargo test`). It builds the C binary
via CMake automatically if `c_src/build/driver` is missing.

## What the C program does

```c
int x = 0;
scanf("%d", &x);
if (x) good(); else bad();
return 0;
```

* `good()` prints `5\n`.
* `bad()` declares `int *data;` and dereferences it **without initializing it**
  — undefined behavior (CWE-457/824).
* Exit status is always `0`; stderr is always empty.

So there are exactly two observable outputs, `5\n` and whatever `bad()` prints,
and the only interesting logic is which branch `scanf` steers into.

## Mismatches found

**None.** Every input class below produced byte-identical stdout, byte-identical
stderr and an identical exit status on the first comparison run. No change to
`translation/src/main.rs` was required, and nothing in `c_src/` was modified.

Because "no mismatches" is only meaningful if the risky cases were actually
probed, the candidate mismatches that were specifically hunted are recorded
below, together with the evidence that the Rust code already matches.

## Candidate mismatches investigated (all confirmed matching)

### 1. The uninitialized-pointer read in `bad()`

The dangerous one: `bad()`'s output is undefined behavior, so it could in
principle be garbage, vary per run, or segfault. Measured behavior of the
reference build (CMake default, unoptimized) is that it **deterministically
prints `0\n` and exits 0**.

* Verified stable across 30 in-test repetitions
  (`uninitialized_pointer_branch_is_stable`) and across 40 runs with the
  environment block padded to different sizes (which shifts stack addresses).
* Never crashed, never printed anything else.

The Rust translation models this as printing `main`'s `x` through a
`thread_local` stale-slot, which is `0` on this path because `bad()` is only
reached when `x` is falsy. That coincides with the reference build for **all**
inputs, since the branch condition itself guarantees `x == 0`.

*Caveat for the next reader:* this is UB, so it is contingent on the reference
build. It was re-confirmed against the actual `c_src/build/driver` binary rather
than assumed.

### 2. `scanf` overflow: truncation vs saturation

This is where a plausible translation goes wrong, because it can **flip the
branch**. glibc's `%d` converts into a `long` (saturating at `LONG_MAX` /
`LONG_MIN` on overflow) and then stores it into an `int` by **truncating to 32
bits**. A translation that clamped to `INT_MAX` instead of truncating would
disagree.

Inputs where truncation makes a nonzero number produce `x == 0`, sending
control to `bad()` instead of `good()`:

| input | glibc `long` | stored `int` | branch | output |
|---|---|---|---|---|
| `4294967296` (2^32) | 4294967296 | 0 | `bad()` | `0\n` |
| `-4294967296` | -4294967296 | 0 | `bad()` | `0\n` |
| `8589934592` (2^33) | 8589934592 | 0 | `bad()` | `0\n` |
| `429496729600` | 429496729600 | 0 | `bad()` | `0\n` |
| `4294967297` (2^32+1) | 4294967297 | 1 | `good()` | `5\n` |
| `18446744073709551616` (2^64) | `LONG_MAX` (saturated) | -1 | `good()` | `5\n` |
| `-18446744073709551616` | `LONG_MIN` (saturated) | 0 | `bad()` | `0\n` |

The last two rows pin down **saturation, not wrap-around**, of the internal
accumulator: true modular wrapping of 2^64 would give 0 (`bad()`), but the C
program prints `5`. The Rust `scanf_int` saturates at `i64::MAX` / `i64::MIN`
and then does `acc as i32`, which reproduces every row.

Covered by `truncation_to_zero_flips_the_branch`, `int_boundaries_and_overflow`
and `powers_of_ten` (10^k and -10^k for k = 0..24, which walks through both
truncated-zero and truncated-nonzero results and past accumulator overflow).

### 3. `scanf` reads across newlines (unlike `fgets`)

`scanf("%d")` skips *all* leading whitespace, newlines included, so a value on
the second or third line is still found. A translation built on line-oriented
reads would return failure and take `bad()`.

Verified for every C whitespace character (space, `\t`, `\n`, `\v` = 0x0b,
`\f` = 0x0c, `\r`), for blank lines before the value, and for a 5000-byte run of
spaces. `is_c_space` in the Rust matches C's `isspace` set exactly.

### 4. Input failure vs matching failure — `x` must stay `0`

`scanf`'s return value is ignored, so on *any* failure `x` keeps its initializer
`0` and control reaches `bad()`. Both failure kinds were exercised:

* input failure: empty stdin, whitespace-only stdin, closed fd 0, `/dev/null`.
* matching failure: `abc`, `.5`, `,`, `0x10` (`%d` stops after the leading `0`),
  and a bare sign with no digit (`-`, `+`, `-x`, `--5`, `- 5`, `-\n5`).

The bare-sign cases matter because a parser that treated `-` as `0` and one that
treated it as a failure both end up at `bad()` here — they agree by
luck, not by construction, so they were checked rather than reasoned about.

### 5. Signed-zero and trailing-junk forms

`-0` and `+0` both yield `x == 0` → `bad()`. Conversion stops at the first
non-digit, and since nothing else in the program reads stdin the remainder is
simply discarded: `7abc` → `5\n`, `0abc` → `0\n`, `0 1` → `0\n`, `3.9` → `5\n`.

### 6. Non-UTF-8 and NUL bytes

The C program sees raw bytes; a Rust translation that decoded stdin as UTF-8
could diverge or panic. Checked `\x00` prefixes, `\xff\xff`, a UTF-8 BOM, a lone
continuation byte `\x80`, and the invalid sequence `\xc3\x28`. The Rust reads
bytes, so all match.

### 7. Buffering and large inputs

* `printf` output is identical whether stdout is a pipe or a redirected file
  (fully-buffered case checked with `cmp`); exact bytes are `35 0a` = `5\n`.
* A short number followed by 1 MiB of unread junk: both programs exit promptly
  without draining stdin. The test writes stdin on a helper thread so this
  cannot deadlock the harness.
* 4096-digit runs of `9` and of `0`.

## Coverage of the C source

Every statement and both sides of the single branch are reached:

| C location | reached by |
|---|---|
| `main` `scanf` success, nonzero | `single_nonzero` |
| `main` `scanf` success, zero | `single_zero` |
| `main` `scanf` input failure | `empty_input`, `whitespace_only_input` |
| `main` `scanf` matching failure | `matching_failure_leaves_x_zero`, `sign_without_digits` |
| `if (x)` true → `good()` | `single_nonzero`, `int_boundaries_and_overflow` |
| `if (x)` false → `bad()` | `single_zero`, `truncation_to_zero_flips_the_branch` |
| `printIntPtrLine` | both branches above |
| `return 0` | exit status asserted in every case |

## Test volume

* `cargo test`: 15 tests, all passing, none `#[ignore]`d, skipped or disabled.
* Additional ad-hoc sweeps run during verification, all with 0 mismatches:
  * 3093 pseudo-random and boundary inputs against the release binary.
  * Exhaustive enumeration of all 2-byte and 3-byte inputs over the alphabet
    `" \t\n+-09xa.\x00\xff"` (1331 + 121 cases) against **both** the release and
    the debug binary, confirming the two Cargo profiles behave identically.
