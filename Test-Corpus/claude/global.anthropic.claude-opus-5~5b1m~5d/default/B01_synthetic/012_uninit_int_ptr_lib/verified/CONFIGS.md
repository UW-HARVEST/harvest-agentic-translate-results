# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C actually takes.

## Axes the C code branches on

```bash
grep -nE 'if|switch|#if|#ifdef|while|for|\?' c_src/src/driver.c c_src/include/driver.h
```

* **Runtime options/flags:** exactly one — the `int useGood` parameter of
  `driver`. It is consumed by a single truthiness branch `if (useGood)`, which
  toggles between two states: the `good()` path and the `bad()` path. There are
  no globals, no init/config struct, no setters, no persistent state, no
  `#ifdef`s, and no compile-time options (`CMakeLists.txt` sets no
  `target_compile_definitions`).
* **Public entry points (full set, lowest level first):**
  1. `printIntPtrLine(const int *)` — lowest level; the only function that
     formats/writes. Takes an arbitrary caller-supplied pointer, so its input
     *shape* axis is the pointed-to `int` value and the pointer's provenance.
  2. `good(void)` — no inputs; composes `printIntPtrLine(&5)`.
  3. `bad(void)` — no inputs; composes `printIntPtrLine(<uninit ptr>)` (CWE-457).
  4. `driver(int)` — the convenience/one-shot wrapper over `good`/`bad`.
* **Input shapes the code distinguishes:**
  * for `printIntPtrLine`: the value at `*intNumber` — sign, zero, single vs.
    multi digit (affects `%d` output width), and the `INT_MIN`/`INT_MAX`
    boundaries; also the pointer's storage class (stack, heap, static,
    `mmap`ed page, interior element of an array, misaligned byte offset).
  * for `driver`: `useGood == 0` vs. `useGood != 0` (the only distinction), with
    non-zero probed at `1`, negative, and both `int` extremes.
  * call counts: zero / one / many, and interleaving of different entry points
    (matters because output goes to the *shared* libc stdout buffer).

## Rows (cross-product, pruned to what the C distinguishes)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `printIntPtrLine` | pointer to a stack `int`; 4096 randomized `i32` values (seeded), incl. all widths/signs | [x] |
| C2 | `printIntPtrLine` | pointer to boundary values: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `9`, `10`, `-9`, `-10`, `99`..., `INT_MAX-1`, `INT_MAX` | [x] |
| C3 | `printIntPtrLine` | pointer to a **heap** (`malloc`ed) `int`, randomized values | [x] |
| C4 | `printIntPtrLine` | pointer to a **static/const** `int` in the caller's data segment | [x] |
| C5 | `printIntPtrLine` | pointer to an **interior element** of an `int` array (index 0, mid, last), randomized array contents | [x] |
| C6 | `printIntPtrLine` | pointer into a freshly `mmap`ed anonymous page (fully mapped, aligned) | [x] |
| C7 | `printIntPtrLine` | **misaligned** pointer: byte offsets 1,2,3 into a mapped buffer, randomized bytes | [x] |
| C8 | `printIntPtrLine` | call count = many (1000 sequential calls, randomized values) — accumulated stdout compared as one blob | [x] |
| C9 | `good` | no inputs; single call (must print `"5\n"`) | [x] |
| C10 | `good` | no inputs; many calls (100) — repeated identical output, no state drift | [x] |
| C11 | `driver` | `useGood = 1` (canonical true) | [x] |
| C12 | `driver` | `useGood != 0`, randomized non-zero `i32` (seeded, 2048 values) incl. negatives | [x] |
| C13 | `driver` | `useGood` at extremes: `INT_MAX`, `INT_MIN`, `-1`, `2`, `0x100`, `0x0001_0000` (low 32 bits non-zero) | [x] |
| C14 | `driver` | `useGood != 0`, many calls (100) | [x] |
| C15 | mixed pipeline | interleaved `driver(1)` → `printIntPtrLine(&v)` → `good()` → `printIntPtrLine(&v2)` sequence, randomized values; compares the whole accumulated stdout stream (catches buffering/order divergence invisible to per-call tests) | [x] |
| C16 | `bad` / `driver(0)` | the CWE-457 path — a *valid* call in the C's eyes but UB; verified in a forked child for outcome-class parity only (see `ERRORS.md` E6/E8) | [x] |

All 16 rows are exercised in `tests/differential.rs`, each by loading **both**
`.so`s via `libloading` and comparing captured stdout byte-for-byte.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the only build configuration is the default one. Verified:

```bash
$ grep -n 'features' Cargo.toml   # -> no matches
```

Hence "repeat Phases B–C for every feature combination" collapses to the single
default combination; it was additionally re-run with `--no-default-features`
(equivalent, and confirmed to compile and pass) to prove there is no hidden
default feature.
