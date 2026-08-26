# CONFIGS.md — Configuration-surface table (valid inputs)

Derived mechanically from the C source, the same way `ERRORS.md` is.

## Axes the C code actually branches on

**Build-time configuration axes: none.**
* `Cargo.toml` has **no `[features]`** section (`cargo metadata` reports
  `features = {}`), so there is exactly **one** feature combination: the default
  (empty) one, i.e. `--no-default-features` == default.
* `c_src/CMakeLists.txt` defines no options, no `target_compile_definitions`,
  and one target from one source file.
* `grep -nE '#\s*(if|ifdef|ifndef|elif|else)'` over the whole library finds only
  the `DRIVER_H_` include guard.

**Runtime option/mode/flag axes: none.** There is no init/config/context
object, no setter, no global, no mode enum. Both functions are pure
straight-line code with zero `if`/`switch`/ternary/loop.

So the entire configuration surface is the **shape of the argument values**.
The axes the code actually distinguishes are exactly the four fields, each
constrained by the width the C declares for it:

| axis | distinct shapes the C treats differently |
|---|---|
| `x` (`unsigned int x : 2`) | in-range `0..=3`; out-of-range `>= 4` (truncated `& 3`) |
| `y` (`unsigned int y : 3`) | in-range `0..=7`; out-of-range `>= 8` (truncated `& 7`) |
| `b` (`bool b : 1`)         | `0`; `1`; out-of-range byte (masked `& 1`); non-zero-low-byte int |
| `z` (`int`)                | `0`; positive; negative; `INT_MIN`; `INT_MAX` (printed by `%d`) |
| entry point                | `driver` (header API, packs the bit-fields) vs `print_foo` (exported low-level API, unpacks a caller-supplied `foo_t`) |
| `print_foo` storage byte   | padding bits 6..7 clear vs set; each of the 256 byte values |
| `print_foo` pointer        | aligned (4) vs misaligned |

## Rows — meaningful combinations (cross-product, pruned to what C distinguishes)

Every row is driven through **both** `.so`s via `libloading`, with `stdout`
captured and compared **byte for byte**. Rows marked *randomized* use many
inputs from a fixed-seed PRNG (seed `0x243F6A8885A308D3`), not one hand-picked
value.

### `driver` — the public header entry point

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 1 | `driver` | all-zero: `x=0,y=0,b=0,z=0` | [x] |
| 2 | `driver` | **exhaustive** in-range cross-product: every `x` in `0..=3` × every `y` in `0..=7` × `b` in `{0,1}` × `z` in a fixed set incl. `0`/`±1`/`INT_MIN`/`INT_MAX` (64 × |z| cases) | [x] |
| 3 | `driver` | in-range `x`,`y`,`b`; `z` *randomized* over the full `i32` range | [x] |
| 4 | `driver` | `x` out of range (`>=4`), `y`/`b` in range, `z` randomized | [x] |
| 5 | `driver` | `y` out of range (`>=8`), `x`/`b` in range, `z` randomized | [x] |
| 6 | `driver` | `b` out of range (`>=2`), `x`/`y` in range, `z` randomized | [x] |
| 7 | `driver` | **all four** arguments fully *randomized* over their whole 32-bit domains (the interaction case: simultaneous truncation of `x`, `y`, and `b`) | [x] |
| 8 | `driver` | boundary values only: `x`,`y`,`b` ∈ {0, max-in-range, max-in-range+1, `UINT_MAX`} × `z` ∈ {`INT_MIN`,`-1`,`0`,`1`,`INT_MAX`} | [x] |
| 9 | `driver` | `z` sign/magnitude sweep: powers of two and their negations, `x`/`y`/`b` randomized | [x] |

### `print_foo` — the exported low-level entry point (called directly, not via `driver`)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 10 | `print_foo` | caller-built `foo_t` with **every** storage byte `0x00..=0xFF` (covers all `x`/`y`/`b` combinations *and* both padding-bit states) × several `z` | [x] |
| 11 | `print_foo` | storage byte and `z` both fully *randomized* (raw 8-byte buffer) | [x] |
| 12 | `print_foo` | struct at a 4-aligned address vs. deliberately misaligned addresses (offsets 1,2,3) | [x] |
| 13 | `print_foo` | `z` extremes (`INT_MIN`, `INT_MAX`, `-1`, `0`) × padding bits set and clear | [x] |

### Composed pipeline (both entry points together, as a real consumer uses them)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| 14 | `driver` → `print_foo` | round-trip equivalence: for randomized `(x,y,b,z)`, assert C `driver(x,y,b,z)` output == Rust `print_foo(hand-packed foo_t)` output **and** vice-versa (cross-library: C's packer feeding Rust's unpacker and Rust's packer feeding C's unpacker), which pins the private `foo_t` layout across the ABI | [x] |
| 15 | `driver` + `print_foo` | many interleaved calls in one process, alternating C and Rust, verifying no cross-call state leaks (padding/stack reuse) and that `stdout` buffering behaviour matches over a long multi-line sequence | [x] |

## Feature-combination coverage

There is exactly one combination (no `[features]`), so rows 1–15 under the
default configuration constitute full coverage of every feature combination.
`cargo check`/`cargo test --no-default-features` is the same build as default
and is run explicitly to confirm.

## Status

All 15 rows pass across their randomized inputs, under both the debug and the
release cdylib. Run: `cargo test --test phase_b_configs` (16 tests including a
harness self-check), or `./verify_all.sh` for the whole Phase D matrix.

## Bug found by this phase

**Row 12 (misaligned `foo_t *`) exposed a real divergence, now fixed.**
`print_foo` formed a `&foo_t` reference from the caller's pointer, which trips
Rust's "misaligned pointer dereference" check and aborts the process. The C
imposes no alignment requirement and x86-64 loads unaligned addresses happily,
so the C library simply prints. `print_foo` now reads the image through libc
`memcpy` instead. This bug was reachable only by calling the low-level
`print_foo` entry point directly — driving the library solely through the
`driver` convenience wrapper (which always passes a properly aligned stack
object) cannot reach it.

## Harness validation (guard against vacuous passes)

A suite that never fails proves nothing, so the harness was mutation-tested:
9 single-edit mutations were injected into `src/lib.rs` (wrong bit-field masks,
wrong shift amounts, wrong `z` offset, `b` as a non-zero test instead of a bit-0
mask, …). **8 of 9 were caught**, 7–15 tests failing each.

The one survivor — `%u` → `%d` for the `x` conversion — was verified to be a
*semantically equivalent* mutant rather than a coverage gap: `x` is masked to
2 bits, and `printf` renders 0–3 identically under `%u` and `%d`, so no input
can distinguish the two. (The same holds for `y`, which is masked to 3 bits.)
