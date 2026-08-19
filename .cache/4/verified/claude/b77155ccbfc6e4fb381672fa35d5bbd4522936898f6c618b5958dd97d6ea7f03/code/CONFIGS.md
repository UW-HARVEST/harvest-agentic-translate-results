# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Derived mechanically from the C source, not from assumptions.

## Build-time configuration

| source | knobs found | conclusion |
|--------|-------------|------------|
| `Cargo.toml` | no `[features]` table at all (`grep -n feature Cargo.toml src/` → no matches) | exactly **one** build configuration |
| `c_src/CMakeLists.txt` | no `option()`, no `if()`, no `add_definitions`, no `target_compile_definitions` | one C build configuration |
| `c_src/*.c/h` | only `#ifndef DRIVER_H_` include guard — no conditional compilation | no `#ifdef` variants |

**Feature combinations (all of them):**

| # | cargo invocation | meaning |
|---|------------------|---------|
| 1 | `cargo <cmd> --no-default-features` | the empty feature set (identical to the default, since no features exist) |
| 2 | `cargo <cmd>` (default) | same as #1 |
| 3 | `cargo <cmd> --all-features` | same as #1 |

All three are the *same* configuration; `check_all.sh` runs check + build +
symbol-diff + the full test suite for each of them anyway.

## Runtime configuration axes (from the C branches)

Public API (`c_src/include/driver.h`) is a single, lowest-level entry point —
there is no wrapper/convenience layer, so "driving it like a real consumer"
means calling `driver(x, y)` directly:

```c
void driver(int x, int y);
```

Axes = every condition the C code branches on (`driver.c`):

| axis | source line | distinct states |
|------|-------------|-----------------|
| A: loop guard | `30: while (x > 0 \|\| y > 0)` | `x>0&&y>0`, `x>0&&y<=0`, `x<=0&&y>0`, `x<=0&&y<=0` |
| B: special case | `33: if (x == 1 && y == 4)` | true; false via `x!=1`; false via `y!=4` |
| C: `label1` guard | `38: if (x > 0)` | `x>0`, `x==0`, `x<0` |
| D: `label2` guard | `44: if (y == 0)` | `y==0` (on entry), `y==0` (reached mid-body), `y!=0` |
| E: back-edge | `49: if (x < 3)` | `x<3` (backward `goto label1`, **no** `while` re-test), `x>=3` (`while` re-test ⇒ extra `loop\n`) |
| F: value shape | — | `0`, `1`, `2`, `3`, `4`, small, medium, large, `INT_MIN`, `INT_MAX`, negatives |
| G: sequencing | — | repeated / interleaved invocations (checks for hidden state); concurrent callers |
| H: output volume | — | small (< stdio buffer) vs. multi-MiB (forces many `stdout` flushes) |
| I: caller's `stdout` state | `printf`/`puts` inherit it | buffering mode `_IOFBF` / `_IOLBF` / `_IONBF` / inherited, and the resulting `write(2)` framing |
| J: kind of fd 1 | — | regular file, pipe, message-preserving socket, closed fd, pipe with no reader |

Rows below are the cross product of A–H pruned to the combinations the C code
actually distinguishes. Each row is exercised with **many randomized inputs**
(seeded LCG, fixed seed `0x2545F4914F6CDD1D`) wherever a range is involved, and
compared byte-for-byte between the C `.so` and the Rust `.so`.

`x > 0 && y < 0` is *excluded* from the valid-input rows: it makes the C library
run ≈2^31 iterations and hit signed-overflow UB (see `ERRORS.md` rows 14/15); it
is covered there by a byte-capped prefix comparison instead.

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `driver` | A=`x<=0&&y<=0`: randomized `x,y ∈ [INT_MIN, 0]` — loop never entered | [x] |
| 2  | `driver` | A=`x<=0&&y>0`, C=`x==0`: `x=0`, randomized `y ∈ [1,8]` — `label1` skipped, only the back-edge path runs | [x] |
| 3  | `driver` | A=`x<=0&&y>0`, C=`x<0` incl. `INT_MIN`: randomized `y ∈ [1,64]` | [x] |
| 4  | `driver` | A=`x>0&&y<=0`, D=`y==0` on entry, E=`x<3`: `x ∈ {1,2}`, `y=0` | [x] |
| 5  | `driver` | A=`x>0&&y<=0`, D=`y==0`, E=`x>=3`: randomized `x ∈ [3,40]`, `y=0` | [x] |
| 6  | `driver` | B=true: the exact `x==1 && y==4` special case (`goto label2`) | [x] |
| 7  | `driver` | B=false via `y!=4`: `x=1`, randomized `y ∈ [1,64]\{4}` | [x] |
| 8  | `driver` | B=false via `x!=1`: randomized `x ∈ [0,64]\{1}`, `y=4` | [x] |
| 9  | `driver` | E=`x<3` boundary: `x=2`, randomized `y ∈ [1,64]` | [x] |
| 10 | `driver` | E=`x>=3` boundary: `x=3`, randomized `y ∈ [1,64]` | [x] |
| 11 | `driver` | E flips mid-run: `x=4` (first decrement lands on 3), randomized `y ∈ [1,64]` | [x] |
| 12 | `driver` | A=`x>0&&y>0`, E mixed: randomized `x ∈ [5,40]`, `y ∈ [1,40]` — starts with `while` re-tests, switches to back-edge once `x<3` | [x] |
| 13 | `driver` | full pruned cross product: **exhaustive** `x ∈ [-4,24] × y ∈ [-4,24]` (minus `x>0&&y<0`) | [x] |
| 14 | `driver` | randomized `x,y ∈ [-64,64]` (minus `x>0&&y<0`), 2000 seeded cases | [x] |
| 15 | `driver` | medium magnitudes: randomized `x,y ∈ [0,1000]`, 200 seeded cases | [x] |
| 16 | `driver` | asymmetric shapes: `y ≫ x` (`x ∈ {0,1,2}`, `y ∈ {500,1000,4096}`) and `x ≫ y` (`x ∈ {500,1000,5000}`, `y ∈ {0,1,2}`) | [x] |
| 17 | `driver` | large scale: `x ∈ {5_000, 50_000, 200_000} × y ∈ {0, 7, 200_000}` | [x] |
| 18 | `driver` | G: 100 sequential invocations and C/Rust interleaved invocations in one process (no hidden state / no cross-call contamination) | [x] |
| 19 | `driver` | F extremes with bounded runtime: `(INT_MIN, y>0)`, `(INT_MIN,0)`, `(INT_MIN,INT_MIN)`, `(-1,0)`, `(0,-1)`, `(0,0)`, `(1,0)`, `(0,1)` | [x] |
| 20 | `driver` | H: multi-MiB output forcing repeated `stdout` buffer flushes (`x=0,y=400_000`; `x=400_000,y=0`; `x=150_000,y=150_000`) | [x] |
| 21 | `driver` | I × A–F: all four `stdout` buffering modes (`_IOFBF`/`_IOLBF`/`_IONBF`/inherited) × 12 fixed + 40 randomized `(x,y)` — byte stream must be identical in every mode | [x] |
| 22 | `driver` | I × J: `write(2)` **framing** per buffering mode, fd 1 = `SOCK_SEQPACKET` socket so that each write is one message — distinguishes the C's compiler-generated `puts("loop")` (payload + `'\n'` as two writes when unbuffered) from a `printf("%s", "loop\n")` translation | [x] |
| 23 | `driver` | G: concurrent callers — 4 threads driving the library simultaneously through one shared `stdout`; multiset of emitted lines must match | [x] |

## Status

All 20 rows are implemented in `tests/differential.rs` and pass under every
feature combination listed above (and additionally under `--release`).

| row | test in `tests/differential.rs` | status |
|-----|----------------------------------|--------|
| 1 | `row01_loop_never_entered` | [x] |
| 2 | `row02_x_zero_small_y` | [x] |
| 3 | `row03_x_negative_y_positive` | [x] |
| 4 | `row04_y_zero_small_x` | [x] |
| 5 | `row05_y_zero_large_x` | [x] |
| 6 | `row06_special_case_x1_y4` | [x] |
| 7 | `row07_x1_y_not_4` | [x] |
| 8 | `row08_x_not_1_y4` | [x] |
| 9 | `row09_back_edge_boundary_x2` | [x] |
| 10 | `row10_back_edge_boundary_x3` | [x] |
| 11 | `row11_back_edge_flip_x4` | [x] |
| 12 | `row12_mixed_back_edge` | [x] |
| 13 | `row13_exhaustive_small_grid` | [x] |
| 14 | `row14_random_small` | [x] |
| 15 | `row15_random_medium` | [x] |
| 16 | `row16_asymmetric_shapes` | [x] |
| 17 | `row17_large_scale` | [x] |
| 18 | `row18_sequencing_and_interleaving` | [x] |
| 19 | `row19_extremes_bounded` | [x] |
| 20 | `row20_multi_mib_output` | [x] |
| 21 | `row21_buffering_modes_byte_equality` | [x] |
| 22 | `row22_write_framing_matches` | [x] |
| 23 | `row23_concurrent_callers` | [x] |

Roughly 6 200 distinct configurations are compared per run, each with both
libraries invoked through their `.so` export.

## Finding from row 22 (fixed)

`gcc` (at every optimization level, including the reference build's implicit
`-O0`) rewrites all three `printf("…\n")` calls in `driver.c` into `puts("…")` —
the reference `.so`'s only libc import is `puts`. The first translation used
`printf("%s", "loop\n")`, which is byte-identical on the stream but emits **one**
`write(2)` where glibc's `puts` emits **two** (payload, then `'\n'`) once the
stream is unbuffered — observable to any caller that shares fd 1 or preserves
message boundaries. `src/lib.rs` now calls `puts`, so both libraries import the
same symbol and produce the same syscall framing; `row22_write_framing_matches`
locks this in (it fails if the `printf("%s", …)` form is restored).

## C build-configuration note

`CMAKE_BUILD_TYPE` is an implicit knob of any CMake project. The reference `.so`
is built with it unset (no `-O` flag). To confirm the C ground truth is not
optimization-dependent — which matters because `y--` is signed-overflow UB for
`y == INT_MIN` — `driver.c` was rebuilt at `-O0/-O1/-O2/-O3/-Os` and each
variant was compared over 1 977 configurations against the reference `.so` and
against both Rust profiles: a single MD5 for all nine artifacts.

## How the comparison works

`driver` is `void`-returning and communicates only through `stdout`, so the
observable output is the byte stream on fd 1, the `write(2)` framing of that
stream, **plus** the process termination status. `tests/common/mod.rs` performs
every call in a forked child whose fd 1 is a temporary file (or a
`SOCK_SEQPACKET` socket for framing, or a closed/broken fd for the write-error
rows), then compares the bytes *and* the raw `waitpid` status of the C child
against the Rust child. (A forked child is required: libtest keeps writing its
own progress lines to fd 1 from other threads, which would contaminate an
in-process redirect.)

Non-terminating inputs (`x > 0 && y < 0`, `x` near `INT_MAX`) are compared as a
byte-capped prefix produced in a child that is `SIGKILL`ed once a full window
has been read — see `ERRORS.md` rows 14/15/18.

Every capture child installs guards (`RLIMIT_FSIZE` = 16 MiB, `alarm(30)`,
and a 100 000-write cap for the framing captures) so that a *divergent*
implementation which loops forever is reported as a status difference within a
second instead of hanging the run or filling the filesystem. The largest
legitimate capture is ≈2.8 MiB (`driver(400_000, 0)`).

## Reproducing

```sh
./check_all.sh                     # every feature combination, symbols + tests
cargo build --offline              # NOTE: `cargo test` does not rebuild a cdylib
cargo test  --offline              # the harness refuses to run on a stale .so
```

The suite was validated by mutation testing — 12 independent mutations of
`src/lib.rs`, each detected by at least one failing test:

| mutation | detected by |
|----------|-------------|
| `y == 4` → `y == 5` (special case) | 5 tests |
| `x < 3` → `x <= 3` (back-edge) | 13 tests |
| `y == 0` → `y <= 0` (`label2` guard) | 1 test (`row14`, the capped-prefix one) |
| `x > 0` → `x >= 0` (`label1` guard) | 19 tests |
| `\|\|` → `&&` (loop guard) | 19 tests |
| `goto label2` → `goto label1` | 4 tests |
| `x--` → `x -= 2` | 18 tests |
| `y--` → `y += 1` (runaway) | 20 tests |
| `label1` falls through to loop top (runaway) | 22 tests |
| `"loop"` → `"Loop"` | 22 tests |
| `puts("x")` → `puts("y")` | 20 tests |
| `puts(s)` → `printf("%s", s)` (framing only) | 1 test (`row22`) |
