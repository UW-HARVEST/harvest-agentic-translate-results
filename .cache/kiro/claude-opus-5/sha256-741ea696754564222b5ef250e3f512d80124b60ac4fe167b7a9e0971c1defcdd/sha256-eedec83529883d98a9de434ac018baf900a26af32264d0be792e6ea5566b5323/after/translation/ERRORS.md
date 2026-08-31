# Verification report: `c_src/src/main.c` → `translation/src/main.rs`

## Result

**No output mismatches were found.** Every input class enumerated from the C
source produced byte-identical stdout, byte-identical stderr (always empty) and
an identical exit status (always `0`) from both binaries.

No change to `translation/src/main.rs` was required, and nothing under `c_src/`
was modified. The only addition is the test suite in
`translation/tests/differential.rs`.

## How it was verified

Both programs were built and driven as subprocesses over identical stdin:

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`

`translation/tests/differential.rs` spawns both binaries for each input and
compares stdout, stderr, exit code and terminating signal. It performs roughly
3,000 differential comparisons across 17 tests. A further ~28,000 ad-hoc
comparisons (exhaustive 1–4 byte inputs over a strtol-relevant alphabet, plus
~5,500 random byte strings) were run during investigation; all matched.

## Branch enumeration

Every conditional in the C was mapped to at least one test input.

### `main()` — `fgets(in, sizeof(in), stdin)` with `char in[100] = ""`

| Branch | Input | Behaviour |
| --- | --- | --- |
| `fgets` returns `NULL` (immediate EOF) | `""`, `/dev/null`, closed fd 0 | buffer stays `""` → error path |
| newline-terminated | `"5\n"` | trailing `\n` retained in buffer |
| EOF without newline | `"7"` | no `\n` in buffer |
| buffer full at 99 bytes | 98 digits + `'5'`, 200 `'1'`s | 100th byte onward never read |
| newline stops the read | `"5\n9\n"` | second line ignored entirely |

The 99/100-byte cutoff is covered by a sweep (`n` zeros then `"7\n"` for
`n = 95..=105`). This is the one place where an off-by-one is silent on the
happy path: with `99` zeros followed by `9`, the significant digit is
truncated away and the value is `0`, not `9`.

### `parse_val()` — `strtol` + three guards

| Guard | Reached by | Result |
| --- | --- | --- |
| `endp != str` fails (no conversion) | `"abc"`, `"   \n"`, `"-"`, `"+"`, `"- 5"`, `"--5"`, `".5"`, `"\x00 5"`, `"\xff5"` | `An error occurred` |
| `errno == 0` fails (`ERANGE`) | `"9223372036854775808"`, `"-9223372036854775809"`, 20+ nines, 99 nines | `An error occurred` |
| `tmp >= INT_MIN` fails | `"-2147483649"`, `"-4294967296"`, `"-9223372036854775808"` | `An error occurred` |
| `tmp <= INT_MAX` fails | `"2147483648"`, `"4294967296"`, `"9223372036854775807"` | `An error occurred` |
| all guards pass | `"0"`, `"5"`, `"-3"`, `"+7"`, `"2147483647"`, `"-2147483648"` | `run(x); run(x);` |

`strtol` semantics that had to be reproduced exactly, each with a test:

- leading whitespace is skipped, using C `isspace` in the "C" locale — space,
  `\t`, `\n`, `\v` (`0x0b`), `\f` (`0x0c`), `\r`.
- base 10, so `"0x1f"` parses as `0` with `endp` after the `0`; because
  `endp != str`, this is a **success** returning `0`, not an error.
- a partial parse succeeds: `"12abc"`, `"2.5"`, `"1e5"`, `"1_000"`, `"12,34"`
  all yield the leading integer.
- `strtol` operates on the C string, so an embedded NUL terminates the scan
  (`"12\0" + "34"` → `12`).
- on overflow the return value saturates to `LONG_MIN`/`LONG_MAX` *and* sets
  `ERANGE`. Both are observable, but here the `ERANGE` guard fires first, so
  the saturated value never reaches the int-range guards.
- glibc's `strtol` does **not** set `errno` when no conversion is performed, so
  the `endp != str` guard is what rejects `"abc"`. This was confirmed
  empirically rather than assumed, since POSIX permits `EINVAL` here.

### `run()` and the mutable global `the_house`

`the_house` is a file-scope global, so the second `run(x)` continues from the
state the first one left behind. This is the most load-bearing detail in the
program: floors go `2 → 3 → 4`, bathrooms `2.5 → 3.5 → 4.5`, and bedrooms
`5 → 5 + x → 5 + 2x`. `golden_happy_path_exact_bytes` pins all eight output
lines as literal bytes so this cannot regress unnoticed.

`add_bedrooms` performs `house->bedrooms += extra_bedrooms` twice on an `int`.
For large `x` this overflows — formally UB in C, wrapping in practice on the
build targets used here. The Rust uses `wrapping_add`, which matches; tests
cover `INT_MAX`, `INT_MIN`, `715827882`, `-715827883` and `1073741824`, chosen
so the overflow happens on the first or the second `run` respectively. Note
that the release profile sets `panic = "abort"`, so a plain `+=` would abort
in debug builds instead of wrapping; `wrapping_add` is required, not stylistic.

`printf("%.1f", ...)` only ever formats `2.5`, `3.5` and `4.5`, all exactly
representable in binary floating point, so no rounding-mode difference between
C's `%.1f` and Rust's `{:.1}` can arise here.

## Non-stdin behaviour checked

- `argv` is ignored by both (`main()` takes no parameters).
- stdin closed, stdin from `/dev/null`, and stdin redirected from a directory
  all take the error path in both, exiting `0`.
- stdout redirected to `/dev/full`, and stdout closed early by a downstream
  reader, produce the same exit status from both.

## Known residual difference (not reachable through normal invocation)

Rust's runtime sets `SIGPIPE` to `SIG_IGN` at startup, whereas the C program
inherits the default disposition. If stdout were a pipe whose reader closed
before the program's ~450 bytes of output were flushed, the C could die from
`SIGPIPE` (status 141) while the Rust would ignore the failed write and exit
`0`. The direct test of this scenario had both exit `0`, because the output is
small enough to be delivered before the reader closes. Fixing it would require
resetting the signal disposition through `libc`, which the crate does not
depend on. It is recorded here as the one observable divergence that a
sufficiently adversarial harness could provoke; no input fed through stdin can
trigger it.

## Negative control

To confirm the suite is not vacuous, three bugs were injected into
`translation/src/main.rs` one at a time and the suite was re-run:

| Injected bug | Caught by |
| --- | --- |
| int-range guard widened to `INT_MAX + 1` | `int_range_boundaries` |
| `fgets` capacity off by one (100 payload bytes instead of 99) | `fgets_truncates_at_99_bytes` |
| `wrapping_add` → `saturating_add` in `add_bedrooms` | `int_range_boundaries`, `long_range_errors_set_errno`, `pseudorandom_inputs` |

Each injected bug produced a failure; the source was restored and verified
byte-identical afterwards, and the suite passes again.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo test --release
```

The test harness builds the C reference itself if `c_src/build/driver` is
absent. All 17 tests pass; none is `#[ignore]`d, skipped or disabled.
