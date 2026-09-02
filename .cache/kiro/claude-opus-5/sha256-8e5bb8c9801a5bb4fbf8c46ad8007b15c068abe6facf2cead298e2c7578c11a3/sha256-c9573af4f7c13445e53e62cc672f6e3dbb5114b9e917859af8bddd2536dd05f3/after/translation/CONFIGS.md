# CONFIGS.md — configuration-surface table (valid inputs)

Mirror of `ERRORS.md`, derived the same mechanical way: from the branches and
entry points the C source actually has, not from a guess about which
configurations matter.

## Axes the C actually distinguishes

**Runtime options / modes / flags.** The public header exposes one function with
one parameter:

```c
void driver(int useGood);        /* include/driver.h:26 */
```

`useGood` is the library's only option. It is consumed by exactly one branch,
`if (useGood)` at `driver.c:50`, compiled as `cmpl $0x0,-0x4(%rbp)`. So the axis
has two states, and C truthiness (not equality with 1) selects them:

* `useGood == 0` → `else` arm → `bad()`
* `useGood != 0` → `then` arm → `good()` (any non-zero bit pattern, including
  negatives, `INT_MIN` and `INT_MAX`)

There are no other flags, no global/static state, no init/teardown, no `#ifdef`
configuration branches (`grep '#if' ` finds only the header include guard), and
no Cargo features (`Cargo.toml` has no `[features]` table).

**Input shapes.** The only data the library reads through a parameter is the
`int` behind `printIntPtrLine`'s pointer. The shape axes are therefore the value
domain of that `int` and the provenance/placement of the pointer:

* value: `0`, `1`, `-1`, `5`, `INT_MAX`, `INT_MIN`, `INT_MAX-1`, `INT_MIN+1`,
  powers of two, values whose decimal form is 1 / 10 / 11 digits, values with
  and without a `-` sign, uniformly random `i32`
* pointer placement: stack local, heap allocation, `static`/`.data`, interior of
  an array (index 0, middle, last), aligned vs. deliberately misaligned
* the read width is fixed at 4 bytes (`%d` / `int`); there is no size, count,
  length, format, byte-order or element-type parameter anywhere in the API, so
  those axes do not exist for this library

**Full set of public entry points**, including the low-level ones — all four
have external linkage and are in `nm -D`, even though only `driver` is declared
in the header. `driver` is the convenience wrapper; `printIntPtrLine` is the
lowest level and is tested directly, not just through the wrapper:

| level | entry point | reached from |
|-------|-------------|--------------|
| 0 (lowest) | `printIntPtrLine(const int*)` | called by `good` and `bad` |
| 1 | `good()`, `bad()` | called by `driver` |
| 2 (wrapper) | `driver(int)` | public header |

## Configuration-surface table

Cross-product of {entry point} × {option state} × {input shape}, pruned to the
combinations the C treats differently. Every row is exercised with many
randomized inputs from a fixed seed (`tests/configs.rs`), except where the row
is a single fixed configuration.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printIntPtrLine` | stack-local `int`, 4096 uniformly random `i32` values (seed 0x5EED_1234) | [x] |
| 2 | `printIntPtrLine` | stack-local `int`, exhaustive boundary set: `0, 1, -1, 5, 2, -2, 9, 10, -9, -10, 99, 100, INT_MAX, INT_MAX-1, INT_MIN, INT_MIN+1, 0x7FFFFFFF, -0x80000000`, all powers of two ±1 | [x] |
| 3 | `printIntPtrLine` | heap-allocated `int` (`Box`/`malloc`), randomized values — different pointer provenance, same read | [x] |
| 4 | `printIntPtrLine` | pointer into a `static` (`.data`) `int`, randomized values | [x] |
| 5 | `printIntPtrLine` | element 0 / middle / last of a multi-element `[i32; N]`, randomized values and randomized index — exercises non-zero pointer offsets | [x] |
| 6 | `printIntPtrLine` | deliberately misaligned but fully readable address (`buf.as_ptr().byte_add(1..=3)`), randomized backing bytes — valid on x86_64, must print the same little-endian reassembly | [x] |
| 7 | `printIntPtrLine` | called repeatedly in a tight loop (1024 back-to-back calls) with randomized values — checks stdio buffering / interleaving and that no per-call state leaks | [x] |
| 8 | `good` | no options; fixed configuration (takes no parameters). Must print `5\n` | [x] |
| 9 | `good` | called repeatedly (256×) and interleaved with `printIntPtrLine` calls in randomized order — checks output ordering through one shared `stdout` | [x] |
| 10 | `driver` | `useGood != 0`, the value being `1` — canonical "true" | [x] |
| 11 | `driver` | `useGood != 0`, 4096 randomized non-zero `i32` (incl. negatives, `INT_MIN`, `INT_MAX`, single-bit values, high-bit-only values) — C truthiness, not `== 1` | [x] |
| 12 | `driver` | `useGood == 0` → `bad()` arm. Unspecified output (uninitialised read); compared structurally: same exit status, and one `^-?[0-9]+\n$` line if it survives. Codegen parity (frame size, `call` not tail-`jmp`, lazy PLT binding) is asserted separately so the same stale slot is read | [x] |
| 13 | `bad` | called directly, not via `driver` — one frame shallower, so a *different* stale slot than row 12. Same structural comparison | [x] |
| 14 | `bad` | called directly after a `good()` call has written known bytes into an overlapping frame — exercises the residue-dependent path with a *controlled* predecessor, the one configuration where `bad`'s output is deterministic. Must match the C exactly, byte for byte | [x] |
| 15 | `bad` | called directly after a `printIntPtrLine(&v)` call with randomized `v` — second controlled-residue configuration | [x] |
| 16 | `driver` | alternating `driver(1)` / `driver(0)` sequence, randomized length and pattern — exercises the composed pipeline across both arms rather than one arm per test | [x] |
| 17 | `driver` | `useGood` with garbage in the upper 32 bits of `rdi` (`0x1_0000_0000`, `0xFFFFFFFF_00000000`): low half zero → `bad` arm; low half non-zero → `good` arm. Only `edi` is read | [x] |
| 18 | all four | full symbol-level codegen parity: `objdump -d` of `printIntPtrLine`, `bad`, `good`, `driver` in both `.so`s must be instruction-for-instruction identical (mnemonics + operands, PLT targets normalised) | [x] |

## Rows deliberately absent

There is no row for buffer sizes, element counts, endianness selection, output
formats, encoding modes, thread counts, or init/config structs, because the C API
has no parameter or branch for any of them. Inventing such rows would be guessing
rather than deriving from the source.

## What "passes" means per row

Rows 1–11 and 14–15, 17–18 are compared **byte for byte** between the two
libraries. Rows 12, 13 and 16 (the arms that reach the uninitialised read on an
uncontrolled slot) are compared on **termination status plus output shape**,
because the C library's own output for those is a leaked stack address that
changes on every run under ASLR — byte equality is not a property the C has
against itself. That distinction is confined to exactly those rows; rows 14 and
15 reach the *same* defect on a slot a predecessor deliberately wrote, and there
byte equality is asserted and holds.

Measured stability of the two comparison-by-termination cases (40 isolated runs
each, both libraries):

| spec | C survives | C faults | Rust survives | Rust faults |
|------|-----------|----------|---------------|-------------|
| `driver(0)` | 40 | 0 | 40 | 0 |
| `bad()` (from the test harness's frame) | 0 | 40 | 0 | 40 |

So the two agree on termination deterministically; it is only the printed
garbage value that is unspecified.

## A divergence this table found

The first translation implemented `driver` as an idiomatic Rust `if`/`else`.
LLVM compiled that to a **tail jump** to `bad`, whereas the C emits a stack frame
plus a real `call`. `bad` therefore read a stack slot 32 bytes higher than in the
C, and printed a small stale value (`3`) where the C printed a leaked stack
address (`603893760`, differing every run). Row 12 catches this; rows 16 and 17
catch it too. It was fixed by emitting all four functions as naked functions
carrying the C's `-O0` codegen verbatim — see `src/lib.rs`.

The second divergence was linkage rather than codegen: a Rust `cdylib` is linked
`-z now` by default while the C `.so` is lazily bound, and the first PLT call in
the C runs `_dl_runtime_resolve` straight through the slot `bad` reads.
`.cargo/config.toml` passes `-Wl,-z,lazy` to match. Row 17 and
`tests/symbols.rs::lazy_binding_matches_the_c_library` both catch a regression
here (verified by removing `.cargo/config.toml` and relinking).

## Test-harness notes

* The test targets are `harness = false`. Comparing the libraries means capturing
  fd 1, and libtest's own progress output ("ok", test names) is written to fd 1
  from parallel threads — that text was observed landing *inside* a capture
  window and being misreported as a divergence. A small sequential runner
  (`common::run_tests`) owns stdout for the whole run instead.
* Rows that reach `bad()` go through `assert_same_in_process_guarded`, which
  probes the configuration in a child process first, so a mistranslation is
  reported as a clean failure instead of killing the runner with a `SIGSEGV`.
* `./check_mutations.sh` applies nine plausible mistranslations and requires the
  suite to reject every one, so these rows are demonstrably not vacuous.
