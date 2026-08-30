# Differential verification report — `c_src/src/main.c` vs `translation/`

## How the two programs are run

| | command |
|---|---|
| C | `cmake -S c_src -B <build> && cmake --build <build>` → `<build>/driver ARG1 ARG2` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver ARG1 ARG2` |

The program takes **no stdin**; all behaviour is driven by `argv`. The test suite
(`translation/tests/differential.rs`) builds the C program with CMake into
`translation/target/c_build` (deliberately *outside* `c_src/`, so the C tree is
never written to), then spawns **both executables** with identical `argv` and
compares stdout, stderr and exit status. The Rust code is never linked as a
library.

## Enumerated input classes (every branch in the C)

| C line / branch | input class | covered by |
|---|---|---|
| `argc != 3` → error, `return 1` | 0, 1, 3, 5 trailing args | `argc_*` tests |
| `end == argv[1]` → error, `return 1` | `""`, `abc`, `"   "`, `-`, `+`, `--5`, `.`, `$`, `\xff9` | `first_arg_*`, `non_utf8_arguments` |
| `end == argv[2]` → error, `return 1` | `""`, `abc`, `" \t "`, `-`, `+`, `0x`, `*` | `second_arg_*` |
| loop never entered (`iterations <= 0`) | `0`, `-1`, `-1000`, `-2147483648`, `2147483648` (truncates to `INT_MIN`) | `zero_iterations`, `negative_iterations`, `long_to_int_truncation` |
| single iteration | `1 1`, `0 1`, `-1 1`, `100 1` | `single_iteration` |
| `*outer >= inner` (then-branch, returns `&inner`) | `1 5`, `5 5`, `12 10` | `happy_path_positive` |
| `*outer < inner` (else-branch, returns `outer`) then later switch | `-3 5`, `-5 4`, `-1 10`, `-100 5`, `0 5` | `else_branch_then_switch` |
| signed `int` overflow of `inner` (doubling) | `1 40`, `1 64`, `1073741824 6`, `2000000000 5`, `2147483647 5` | `wraparound_on_doubling` |
| `long` → `int` truncation of both `strtol` results | `4294967296`, `4294967297`, `2147483648`, `-2147483649` | `long_to_int_truncation` |
| `strtol` range saturation to `LONG_MAX`/`LONG_MIN` | `9223372036854775808`, `-9223372036854775809`, 60-digit number | `strtol_saturates_out_of_range` |
| `strtol` acceptance details | leading `isspace()` (` \t\n\v\f\r`), `+`/`-`, `007`, trailing junk (`12abc`, `1e5`, `3 4`), base-10 only (`0x10`, `0b11`) | `strtol_leading_whitespace_and_sign`, `strtol_trailing_junk_is_accepted`, `strtol_base10_only`, `all_c_whitespace_is_skipped` |
| non-UTF-8 argv bytes | `\xff9`, `9\xff`, `\xc3`, `\xe2\x82\xac5` | `non_utf8_arguments` |

Plus three sweeps: the full grid `initial ∈ [-20,20] × iterations ∈ [0,12]`,
a table of `int`-range extremes × 10 iteration counts, and 650 deterministic
pseudorandom cases (numeric and random textual arguments).

## Mismatches found

**None.** Every input class above produced byte-identical stdout, byte-identical
stderr (always empty — the C prints its errors to *stdout* via `printf`, not
stderr) and an identical exit status. `cargo test` reports 31/31 passing in both
the `debug` and `release` profiles, and no test is `#[ignore]`d.

The C tree is unmodified (`c_src/src/main.c` `md5 bc307974bca17e9618bdc55bf57c50fe`,
`c_src/CMakeLists.txt` `md5 88b0836e1b60d97bef2e41ef476e5044`); the only thing
added under `c_src/` is the `build/` directory the task instructions asked for.

## Behaviours that had to be preserved (and were verified, not assumed)

These are the places where a naive translation *would* have diverged; each was
checked explicitly rather than reasoned about.

1. **Errors go to stdout, not stderr.** The C uses `printf` for all three error
   messages, so stderr is empty even on the failure paths and the exit code is
   `1`. A translation using `eprintln!` would pass a stdout-only test and fail
   here. The Rust writes them to stdout. Verified: stderr is empty for every
   input.
2. **`strtol`, not `atoi`/`str::parse`.** `12abc` is *accepted* (value 12,
   `end != nptr`), `0x10` yields `0` in base 10 (`end` points at `x`), leading
   whitespace and a `+`/`-` sign are consumed, and `""`/`"abc"`/`"-"` are the
   only shapes that leave `end == nptr` and take the error path. `str::parse`
   would reject `12abc` and `+7`-with-space and accept nothing with trailing
   junk — the hand-written `strtol_base10` reproduces C exactly.
3. **`isspace()` includes `\v` and `\f`.** `is_c_space` covers
   `' ' \t \n \v \f \r`; confirmed with raw-byte arguments.
4. **Out-of-range `strtol` saturates, then `long`→`int` truncates.**
   `99999999999999999999` → `LONG_MAX` (`0x7fff…ff`) → `(int)` = `-1`, so the
   program prints `0 1 2`. The negative counterpart → `LONG_MIN` → `(int)` = `0`,
   printing `1 2 4`. Accumulating the magnitude negatively in the Rust
   `strtol_base10` is what makes `LONG_MIN` representable without an i64
   overflow.
5. **`iterations` is an `int`, so `4294967297` means one iteration** and
   `2147483648` (→ `INT_MIN`) means the loop body never runs.
6. **Signed overflow wraps two's-complement.** `inner += *outer` and
   `*outer += inner` are UB in C once they overflow; the Rust uses
   `wrapping_add`, which matches what GCC emits. Checked against the C built
   both with CMake's default (unoptimised) flags **and** with `-O3` (GCC 11.5) —
   identical output in both, e.g. `1 40` ends `… 1073741824, -2147483648, 0, 0,
   …`. Using plain `+` in Rust would panic in a debug build; the release tests
   would still have passed, so this was tested in both profiles.
7. **The aliasing itself.** Once `static_alias` returns `&inner`, the next call
   passes `outer == &inner`, so `*outer >= inner` is trivially true and `inner`
   *doubles* every subsequent iteration. While the pointer still aliases
   `initial_value`, the else-branch increments it by `inner` each time. The Rust
   models the two possible pointees as a `Cells { outer, inner }` pair plus a
   `Target` tag, so a store through the pointer is visible to both readers
   exactly as it is in C. Verified over the whole small grid, which contains
   both branch orders and the switch-over point.
8. **Validation order.** `argc` first, then `argv[1]`, then `argv[2]`. With both
   arguments invalid (`abc def`) only the *first* message is printed.
9. **Exact message text and trailing newlines**, including the parenthesised
   `(integer)` and the `!`, compared byte for byte.
10. **`static int inner = 1` is initialised once per process**, not once per
    call — the Rust seeds `Cells.inner = 1` before the loop, never inside it.

## Running the suite

```
cd translation && cargo test              # 31 passed
cd translation && cargo test --release    # 31 passed
```
