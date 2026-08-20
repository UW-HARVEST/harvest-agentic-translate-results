# CONFIGS.md — Configuration-surface table (Phase B gate)

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Public entry points (complete set)

`grep` of the public header and of the C `.so`'s dynamic symbol table yields
exactly one public entry point — there is no lower-level API to reach past it:

| entry point | signature | notes |
|-------------|-----------|-------|
| `driver` | `void driver(int x, int local_y, int z)` | sole export; `static multi_stage` and `static int y` are internal |

## Axes the C code actually branches on

Enumerated from every `if` in the translation unit (there are no `switch`
statements and no `#ifdef`s):

| axis | source | distinct states |
|------|--------|-----------------|
| `x` | `if (x != 1)` — line 33 | `x == 1` / `x != 1` |
| `local_y` | assigned to `static int y` (line 60), tested by `if (y != 2)` — line 39 | `local_y == 2` / `local_y != 2` |
| `z` | `if (z != 3)` — line 45 | `z == 3` / `z != 3` |
| static `y` initial value | `static int y = 123;` — line 29 | `123` before the first `driver` call; unobservable afterwards because `driver` overwrites it on entry |
| call multiplicity | `y` is process-global, mutated per call | first call after `dlopen` / repeated calls / interleaved argument patterns |
| value magnitude | none — plain `int` arithmetic, no range logic | `0`, `±1`, `INT_MIN`, `INT_MAX`, arbitrary |

There are **no** runtime options, modes, flags, byte-order choices, element
types, widths, or buffer/length parameters: the API takes three scalars and
writes text to `stdout`.

## Configuration-surface table

Full cross-product of the three branch axes (8 rows), plus the state /
multiplicity / magnitude axes. Every row is driven through the `.so` export of
BOTH libraries and the captured `stdout` byte streams are compared byte-for-byte.
Rows marked "randomized" use a fixed-seed xorshift64* PRNG (seed `0x2545F4914F6CDD1D`),
many samples per row.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `driver` | `x == 1`, `local_y == 2`, `z == 3` — the only success path (`Ok!`, `Result: 0`) | `cfg_row01_all_match` | [x] |
| 2 | `driver` | `x == 1`, `local_y == 2`, `z != 3` — randomized `z` over the full `int` range excluding 3 | `cfg_row02_x1_y2_zbad` | [x] |
| 3 | `driver` | `x == 1`, `local_y != 2`, `z == 3` — randomized `local_y` excluding 2 | `cfg_row03_x1_ybad_z3` | [x] |
| 4 | `driver` | `x == 1`, `local_y != 2`, `z != 3` — randomized both | `cfg_row04_x1_ybad_zbad` | [x] |
| 5 | `driver` | `x != 1`, `local_y == 2`, `z == 3` — randomized `x` excluding 1 | `cfg_row05_xbad_y2_z3` | [x] |
| 6 | `driver` | `x != 1`, `local_y == 2`, `z != 3` — randomized `x`, `z` | `cfg_row06_xbad_y2_zbad` | [x] |
| 7 | `driver` | `x != 1`, `local_y != 2`, `z == 3` — randomized `x`, `local_y` | `cfg_row07_xbad_ybad_z3` | [x] |
| 8 | `driver` | `x != 1`, `local_y != 2`, `z != 3` — randomized all three | `cfg_row08_all_bad` | [x] |
| 9 | `driver` | **First call after load**: exercises the initial `static int y = 123` before it is overwritten (fresh `dlopen` of each library, single `driver(1, 2, 3)` call) | `cfg_row09_first_call_after_fresh_load` | [x] |
| 10 | `driver` | **First call after load with `local_y == 123`**: distinguishes "initial value observed" from "assignment happened" (fresh `dlopen`, `driver(1, 123, 3)`) | `cfg_row10_first_call_y_equals_initial` | [x] |
| 11 | `driver` | **Repeated calls / static-state carry-over**: a long fixed-seed randomized *sequence* of calls against one loaded instance, comparing the concatenated output of the whole session (catches divergent handling of the persistent `static y`) | `cfg_row11_session_state_carryover` | [x] |
| 12 | `driver` | **Boundary magnitudes**: `x`, `local_y`, `z` each drawn from `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, 3, 4, INT_MAX-1, INT_MAX}` — full 11×11×11 = 1331-point grid | `cfg_row12_boundary_grid` | [x] |
| 13 | `driver` | **Success path re-entered after failures**: alternating good/bad calls to prove no latent state makes `Ok!` unreachable in one library only | `cfg_row13_alternating_good_bad` | [x] |
| 14 | `driver` | **Unrestricted random fuzz**: uniform random `int` triples over the whole 32-bit domain (mostly hits row 5/8 paths, but with no exclusion filtering, so value-dependent bugs surface) | `cfg_row14_unrestricted_fuzz` | [x] |
| 15 | `driver` | **`x == 1` biased fuzz**: `x` fixed to 1 and `local_y`/`z` drawn from a small biased set `{2, 3, 0, 1, -1, INT_MIN, INT_MAX}` so the deeper stages 2 and 3 are hit densely | `cfg_row15_deep_stage_biased_fuzz` | [x] |
| 16 | `driver` | **Buffering / interleaving**: many calls with no intervening flush, comparing one large captured buffer — verifies the Rust `printf` route produces the same stdio chunking as the C `printf`/`puts` mix | `cfg_row16_unflushed_bulk_interleaving` | [x] |

## Deliberately excluded axis: concurrency

`driver` mutates the file-scope `static int y` with a plain non-atomic store, so
two concurrent C calls are a data race, i.e. undefined behaviour with no defined
byte stream to compare against. The Rust translation uses `AtomicI32` with
`Ordering::Relaxed` (mutable module state cannot be touched from safe Rust
otherwise); for the single-threaded sequences the C code's contract actually
covers, a relaxed atomic load/store is observationally identical to the plain
`int`. There is therefore no differential row for concurrent calls — there is no
C ground truth to differentiate against.

## Feature combinations

`Cargo.toml` declares `[features] default = []` and no other feature; the C
`CMakeLists.txt` has no `option()`, no `target_compile_definitions`, and no
`#ifdef` in the sources. The complete set of build configurations is therefore:

| # | cargo invocation | equivalent C build |
|---|------------------|--------------------|
| 1 | `cargo test` (default) | default `cmake ..` |
| 2 | `cargo test --no-default-features` | same (default feature set is empty) |
| 3 | `cargo test --all-features` | same (no non-default features exist) |

All three are run by `./run_all.sh`.
