# CONFIGS.md — Phase B configuration-surface table

## Mechanical derivation of the axes

Public API, from `c_src/include/driver.h` (the complete public header):

```c
void driver(int x, int y);
```

That is the **full set of public entry points, one entry point, and it is also
the lowest-level one** — there is no convenience wrapper layer above it and no
internal helper below it. `nm -D --defined-only` on the C `.so` confirms exactly
one exported symbol, `driver`.

Axes the C actually branches on:

* **runtime options / modes / flags**: none. There is no init function, no
  context/handle struct, no setter, no global, no environment variable read, and
  no `if`/`switch` anywhere in `src/driver.c`. The grep in `ERRORS.md` shows the
  only preprocessor conditional in the subtree is the `DRIVER_H_` include guard,
  so there is no `#ifdef`-selected behaviour either.
* **compile-time features**: none (`Cargo.toml` has no `[features]`).
* **input shapes**: the two `int` parameters. `int` is 32-bit two's complement on
  the target, so the shape axes the code's behaviour actually depends on are the
  *bit patterns* of `x` and `y` (feeding `x | ~y`) and, via `printf("%d", …)`,
  the *sign* and *decimal digit width* of the result.
* **call-sequence shape**: `driver` is stateless, but it writes to the shared
  `stdout` `FILE` buffer with `printf` (no newline) followed by `puts("")`, so
  the number and ordering of successive calls is a genuine observable axis
  (`empty / one / many`).

The rows below are the pruned cross-product of those axes: the boundary values of
each parameter against each other, the classes that make the result's sign and
digit width vary, the algebraically special pairs (`y = 0` ⇒ `~y = -1` ⇒ result
`-1` for any `x`; `x = -1` ⇒ result `-1` for any `y`; `x = 0, y = -1` ⇒ result
`0`), full-range random bit patterns, and the call-count axis.

Every row is driven through **both** `.so` files loaded with `libloading` and the
real `stdout` bytes are captured and compared. Rows marked *(randomized)* run
many pseudorandom inputs from a fixed-seed SplitMix64 generator, so they are
reproducible.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `driver` | no options (none exist); `x = 0`, `y = 0` — zero/zero boundary, result `-1` | [x] |
| 2  | `driver` | `x = 0`, `y = -1` — the unique pair whose result is `0` (1-digit non-negative output) | [x] |
| 3  | `driver` | `x = -1`, `y` *(randomized, full 32-bit range)* — `x` saturates the OR, result always `-1` | [x] |
| 4  | `driver` | `x` *(randomized, full range)*, `y = 0` — `~y = -1` saturates the OR, result always `-1` | [x] |
| 5  | `driver` | `x = 0`, `y` *(randomized negative)* — result is `~y` ∈ `[0, INT_MAX]`, non-negative output | [x] |
| 6  | `driver` | `x = 0`, `y` *(randomized non-negative)* — `~y` has the sign bit set, negative output | [x] |
| 7  | `driver` | all 25 pairs from the boundary set `{INT_MIN, -1, 0, 1, INT_MAX}` × itself | [x] |
| 8  | `driver` | `x = INT_MIN`, `y = INT_MAX` — result `INT_MIN`, the widest negative output (`-2147483648`, 11 bytes) | [x] |
| 9  | `driver` | `x = INT_MAX`, `y = INT_MIN` — result `INT_MAX`, the widest non-negative output (`2147483647`, 10 bytes) | [x] |
| 10 | `driver` | `x` *(randomized positive)*, `y` *(randomized negative)* — the only class that yields non-negative results, mixed digit widths | [x] |
| 11 | `driver` | `x` *(randomized negative)*, `y` *(randomized, full range)* — sign bit of `x` forces a negative result | [x] |
| 12 | `driver` | `y = INT_MAX` (`~y = INT_MIN`), `x` *(randomized, full range)* — sign bit forced on by `~y` | [x] |
| 13 | `driver` | `y = INT_MIN` (`~y = INT_MAX`), `x` *(randomized, full range)* — all value bits forced on | [x] |
| 14 | `driver` | `x = INT_MAX`, `y` *(randomized, full range)* — all value bits of `x` set, sign bit taken from `~y` | [x] |
| 15 | `driver` | `x = INT_MIN`, `y` *(randomized, full range)* — sign bit of `x` set, value bits from `~y` | [x] |
| 16 | `driver` | non-negative-result digit-width sweep: inputs chosen so the result is each of `0,1,2,…` and `10^k`, `10^k − 1`, `10^k` for `k = 0..9` (1- to 10-byte output) | [x] |
| 17 | `driver` | negative-result digit-width sweep: inputs chosen so the result is `−1`, `−(10^k)`, `−(10^k − 1)` for `k = 0..9` plus `INT_MIN` (2- to 11-byte output) | [x] |
| 18 | `driver` | single-bit and single-hole patterns: `x = 1<<i`, `y = ~(1<<j)` for all `i, j` in `0..31` (1024 pairs) — walks every bit position through both operands | [x] |
| 19 | `driver` | `x` and `y` both *(randomized, full 32-bit range, uniform over all bit patterns)*, large batch — the general case | [x] |
| 20 | `driver` | call-count axis, *empty*: zero calls — asserts neither library writes anything to `stdout` on load/unload | [x] |
| 21 | `driver` | call-count axis, *one*: exactly one call, verifying the `printf` (no trailing newline) + `puts("")` pair emits exactly `<digits>\n` with no extra bytes | [x] |
| 22 | `driver` | call-count axis, *many*: a long sequence of *(randomized)* calls with no flush in between, comparing the whole concatenated buffered `stdout` stream — catches buffering/ordering divergence invisible to single-call tests | [x] |
| 23 | `driver` | exhaustive low-byte sweep: `x, y` over `{-128..127}` × `{-128..127}` (65 536 pairs) driven through both `.so`s in one captured stream | [x] |
