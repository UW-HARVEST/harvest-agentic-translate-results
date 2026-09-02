# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`, by
enumerating the branches and input shapes the C code actually distinguishes.

## Axes the C actually branches on

| axis | where in the C | values the C distinguishes |
|------|----------------|----------------------------|
| A1 — `driver` mode flag | `driver.c:75` `if (useGood)` | zero → `bad()`; non-zero → `good()` (C truthiness, **not** `== 1`) |
| A2 — entry point / call depth | five external-linkage functions in `driver.c` | *top-level wrapper*: `driver`. *mid-level, called by the wrapper but also directly callable*: `bad`, `good`. *lowest-level leaf printers*: `printIntLine`, `printLine`. All five are exported, so all five are public entry points and each must be driven directly, not only through `driver`. |
| A3 — `printLine` payload shape | `driver.c:32` null guard, then `printf("%s\n", line)` | NULL / empty / 1 byte / many bytes / >stdio-buffer / embedded `%` specifiers / high bytes `0x80..0xFF` / embedded `\n` / embedded `\t` |
| A4 — `printIntLine` value shape | `driver.c:38` `printf("%d\n", ...)` | `0` / positive / negative / `INT_MAX` / `INT_MIN` / random full-range |
| A5 — `alloca` sizing inside the mid-level fns | `driver.c:46` `alloca(10)` vs `driver.c:62` `alloca(10*sizeof(int))` | the *defect axis*: `bad` under-allocates 10 **bytes** then writes 10 `int`s (40 bytes); `good` allocates 40 bytes. Both then copy `source[10] = {0}` and print `data[0]`. Observable output is identical (`0`) for both — this axis must be confirmed, not assumed. |
| A6 — repetition / accumulated state | none — no `static`, no global, no heap in `driver.c` | the library is stateless, so *N* calls must equal *N* independent calls; sequences and interleavings of the five entry points are still a distinct shape to test (a stateful Rust translation would diverge here) |
| A7 — `#ifdef` / compile-time config | `grep -c "#if\|#ifdef\|#ifndef" c_src/src/driver.c` → only the `driver.h` include guard | none. `translation/Cargo.toml` has **no `[features]` section**, so the only feature combination is the default one (see "Feature combinations" below) |

`bad` and `good` take no parameters, so their only axes are A5 and A6.

## Table — one row per combination the C treats differently

| # | entry point(s) | configuration (options set + input shape) | ✓ |
|---|----------------|--------------------------------------------|---|
| 1 | `printIntLine` (lowest level) | value `0` | [x] |
| 2 | `printIntLine` | small positive values `1..=9` | [x] |
| 3 | `printIntLine` | small negative values `-9..=-1` | [x] |
| 4 | `printIntLine` | boundary values: `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1` | [x] |
| 5 | `printIntLine` | 512 randomized full-range `i32` (seeded, reproducible) | [x] |
| 6 | `printIntLine` | digit-width sweep: `±10^k` and `±(10^k − 1)` for k = 0..9 (crosses every `%d` field-width) | [x] |
| 7 | `printLine` (lowest level) | NULL pointer (guard false) | [x] |
| 8 | `printLine` | empty string `""` | [x] |
| 9 | `printLine` | single ASCII byte | [x] |
| 10 | `printLine` | short ASCII string, many randomized (seeded) | [x] |
| 11 | `printLine` | long string spanning past the stdio buffer (1 B … 64 KiB length sweep) | [x] |
| 12 | `printLine` | payload containing `printf` conversion specifiers (`%s %d %n %%`) | [x] |
| 13 | `printLine` | payload containing high / non-ASCII bytes `0x80..0xFF` (all 255 non-NUL byte values) | [x] |
| 14 | `printLine` | payload containing embedded `\n`, `\r`, `\t` | [x] |
| 15 | `bad` (mid level, direct) | no args; under-sized `alloca(10)` path, single call | [x] |
| 16 | `good` (mid level, direct) | no args; correctly-sized `alloca(40)` path, single call | [x] |
| 17 | `bad` | repeated 256× — confirms statelessness / no accumulated corruption from the 30-byte overrun | [x] |
| 18 | `good` | repeated 256× — confirms statelessness | [x] |
| 19 | `bad`, `good` | randomized interleaving of the two (seeded), 512 calls, one captured stream | [x] |
| 20 | `driver` (top-level wrapper) | `useGood = 0` → must take the `bad()` branch | [x] |
| 21 | `driver` | `useGood = 1` → must take the `good()` branch | [x] |
| 22 | `driver` | `useGood` non-zero but ≠ 1: `2`, `-1`, `7`, `0x100`, `INT_MAX`, `INT_MIN` → truthiness, all take `good()` | [x] |
| 23 | `driver` | 512 randomized `i32` flags (seeded) — mixes zero and non-zero across the full range | [x] |
| 24 | `driver` | randomized *sequence* of flags in one captured stream (order-dependence / statefulness check) | [x] |
| 25 | `driver` vs `bad`/`good` | equivalence check: `driver(0)` output ≡ `bad()` output, and `driver(nonzero)` output ≡ `good()` output — asserts the wrapper composes the low-level fns exactly as the C does | [x] |
| 26 | all five, mixed | randomized pipeline: seeded interleaving of `driver`/`bad`/`good`/`printIntLine`/`printLine` (1024 ops) into one captured stream — exercises the composed pipeline, which per-wrapper tests cannot see | [x] |

## Observationally equivalent by construction (recorded, not tested differentially)

Two properties of `bad`/`good` cannot be observed through the exported ABI, so
no differential test can cover them — and neither can any consumer of the
original C library:

| property | why it is unobservable |
|----------|------------------------|
| the loop bound `10` in `for (i = 0; i < 10; i++)` | `source[10] = {0}` is all zeros and only `data[0]` is printed, so any bound >= 1 prints the same `0` |
| the printed index `data[0]` | every element of `source` is `0`, so every index prints `0` |

`scripts/mutation_check.sh` records these as EXPECTED SURVIVORS (mutants
`loop_off_by_one`, `bad_prints_index_1`): the mutants leave the differential
suite green, and that is correct rather than a gap. They are confirmed by
reading the Rust against the C line for line instead.

Two further properties are *also* invisible to stdout — because C's `bad()` and
`good()` print identical bytes — but ARE pinned, structurally, in
`tests/structural.rs`:

| property | check |
|----------|-------|
| `driver` maps zero → `bad()` and non-zero → `good()` (branch **direction**, not just truthiness) | `struct_03_driver_maps_zero_to_bad_and_nonzero_to_good` — resolves the conditional-jump sense and both branch targets via `objdump -d` + the `objdump -R` relocation table, for C and Rust, dev and release |
| `bad` and `good` are independent implementations, neither forwarding to the other | `struct_04_bad_and_good_do_not_call_each_other` |

Both were found by the mutation campaign (mutants `driver_inverted` and
`fold_bad_into_good` initially SURVIVED Phases B and C) and are now killed.

## Feature combinations

`translation/Cargo.toml` declares no `[features]` table and no optional
dependencies, therefore the complete set of feature combinations is the single
default one. Verified mechanically by `scripts/check_features.sh`, which parses
`Cargo.toml` and loops `cargo check` / `cargo test` over every combination it
finds (default, `--no-default-features`, and each individual feature); with no
features declared this reduces to the default plus `--no-default-features`,
both of which are run.
