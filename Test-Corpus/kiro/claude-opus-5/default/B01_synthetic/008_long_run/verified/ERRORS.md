# Verification log: `c_src/src/main.c` vs. the Rust translation

Method: both programs are built and then executed as subprocesses with the same
argument vector (including the same `argv[0]`, see below); stdout, stderr and
the exit status are compared byte for byte. The tests live in
`tests/differential.rs`; supporting unit tests for the `rand()`, `strtoul()` and
arithmetic-kernel replicas live in `src/main.rs`.

Reference commands:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
```

Runtime warning: an accepted seed makes both programs perform
`2000 * 100 * 256Ki` arithmetic steps. The C build has no optimisation flags
(`CMakeLists.txt` sets none), so one accepted-seed run of the C program takes
roughly 7-8 minutes on the test machine. `cargo test` runs the eight
accepted-seed cases in parallel, so the whole suite takes about that long.

## Input classes enumerated from the C source

Every branch in `main()` and the inputs that reach it:

| Class | Example arguments | C behaviour |
| --- | --- | --- |
| `argc != 2` | *(none)*, `42 43`, `1 2 3 4 5`, `42 ""` | `Usage: <argv0> <seed>` on stderr, exit 1 |
| `strtoul` performs no conversion, `*endptr != 0` | `abc`, `" "`, `\t`, `\n`, `\r`, `\v`, `\f`, `+`, `-`, `+-1`, `--1`, `x` | `Invalid seed: '<arg>'` on stderr, exit 1 |
| partial conversion, `*endptr != 0` | `12abc`, `"42 "`, `1.5`, `0x10`, `0b1`, `1e3`, `7\n`, `1,000`, `9-9`, `\v42x` | same error, exit 1 |
| negated value, `> UINT_MAX` | `-1`, `-42`, `-4294967295`, `-4294967296`, `-2147483648` | same error, exit 1 |
| converts cleanly but `> UINT_MAX` | `4294967296`, `4294967297`, `5000000000`, `18446744073709551614`, `18446744073709551615` | same error, exit 1 |
| `errno == ERANGE` | `18446744073709551616`, `99999999999999999999999999`, `-18446744073709551616`, `-99999…`, 4096 nines | same error, exit 1 |
| argument is not valid UTF-8 | `\xff\xfe`, `42\x80`, `\xc3`, `\xf0\x9f\x92\xa9` | `%s` echoes the raw bytes, exit 1 |
| accepted, seed 0 | `0`, `""` (!), `-0`, `-18446744073709551615` | prints `42032659`, exit 0 |
| accepted, ordinary seed | `42`, `"\t\n\v\f\r +0000000042"` | prints `430392287`, exit 0 |
| accepted, `> INT_MAX` | `2147483648` | prints `269448949`, exit 0 |
| accepted, `== UINT_MAX` | `4294967295` | prints `494145113`, exit 0 |

Quirks of the C that are deliberately reproduced, not fixed:

* **The empty string is a valid seed.** `strtoul("", &endptr, 10)` performs no
  conversion, so it returns 0, leaves `errno` alone and sets `endptr == nptr`
  — which points at the terminating NUL. `*endptr != '\0'` is therefore false
  and the program runs with seed 0.
* **`-0` and `-18446744073709551615` are valid seeds.** `strtoul` negates modulo
  2^64, so the first yields 0 and the second yields 1; both pass the
  `> UINT_MAX` test. (`-1`, by contrast, becomes `ULONG_MAX` and is rejected.)
* **`srand(0)` behaves like `srand(1)`** — glibc substitutes 1 for a zero seed,
  so the four seed-0-ish inputs and seed 1 all print the same number.
* **Signed overflow, `x << 1` on a negative value and `x >> 3` on a negative
  value** are UB/implementation-defined in C; gcc realises them as
  two's-complement wrap-around and an arithmetic shift, and the Rust uses
  `wrapping_*` plus a signed `>>` to match.
* An argument containing an embedded NUL is not a reachable case: `execve()`
  cannot deliver one.

## Mismatches found

### 1. `argv[0]` in the usage message (test-harness defect, not a translation bug)

`fprintf(stderr, "Usage: %s <seed>\n", argv[0])` echoes `argv[0]`, and the two
executables sit at different paths (`c_src/build/driver` vs
`translation/target/*/driver`). A naive harness that spawns each binary by path
reports a stderr mismatch on every `argc != 2` input even though both programs
behave identically. Fixed in the harness, not in the program: every invocation
sets `argv[0]` explicitly to `"driver"` for both processes via
`std::os::unix::process::CommandExt::arg0`.

### 2. SIGPIPE disposition differed (fixed in `src/main.rs`)

Found by inspection rather than by a failing test. The Rust runtime sets
`SIGPIPE` to `SIG_IGN` before `main` runs, whereas a C program inherits the
default `SIG_DFL`. With stdout connected to a closed pipe the C program dies
from `SIGPIPE` (shell status 141) while the Rust program would ignore the
signal, drop the failed `write` and exit 0. `main()` now restores
`signal(SIGPIPE, SIG_DFL)` as its first action. This is not covered by the
differential tests because a captured-output harness always keeps the pipe open,
so the divergence is unreachable from the test suite — it is fixed for fidelity.

### 3. No output mismatches

Beyond the two items above, every enumerated input already produced identical
stdout, stderr and exit status. Specifically confirmed against the real C
implementation:

* the glibc `rand()` replica (`src/glibc_rand.rs`) reproduces
  `srand()`/`rand()` for seeds 0, 1, 42, 12345, 2147483647, 2147483648,
  3000000000 and 4294967295, at draws 0-7 and 1008-1011 (checked against a
  program calling glibc directly);
* the arithmetic kernel (`expensive_step`) reproduces the C step function for
  the starts 1804289383, `INT_MIN`, `INT_MAX`, 0, ±1, ±2, ±7 and ±1000000007
  over 8-24 steps (checked against the same C code compiled on its own);
* `strtoul` acceptance/rejection and the resulting seed match glibc for all 28
  probe strings listed above.

### Non-differences worth recording

* **Threading.** The Rust spreads the per-element transformation over several
  threads. Each array element evolves independently in the C
  (`perform_expensive_operations` reads and writes only `array[i]`), and the
  final reduction is an XOR, so the result cannot depend on the order; the
  observed outputs confirm it.
* **`%d` vs `{}`.** Both print an `i32` in decimal with no padding, so the
  formatting agrees for negative results too, even though no tested seed
  produced one (after the kernel every element observed had its sign bit set,
  and `ARRAY_SIZE` is even, so the XOR comes out non-negative).
* **Zero-initialised global array.** The C array lives in BSS but every slot is
  overwritten by `rand()` before it is read, so the Rust `vec![0; …]` is
  equivalent.
* **`[profile.dev] opt-level = 3`** was added to `Cargo.toml`. `cargo test`
  drives the dev-profile binary, and an unoptimised build of this workload would
  make the suite take hours. It changes no observable behaviour: the dev and
  release binaries print the same values, and the dev profile keeps
  `overflow-checks` on, which passing tests show never fire.
