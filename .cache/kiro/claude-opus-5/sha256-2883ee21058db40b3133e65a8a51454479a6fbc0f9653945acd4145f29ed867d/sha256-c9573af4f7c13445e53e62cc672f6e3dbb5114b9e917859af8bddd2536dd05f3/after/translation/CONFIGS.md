# CONFIGS.md — Phase B configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the axes
the C source actually branches on.

## Axis inventory (derived from the source, not guessed)

Public headers: `c_src/include/driver.h` declares exactly one entry point:

```c
void driver(int x, int y, int z);
```

| axis | kind | values the C distinguishes | evidence in `c_src/src/driver.c` |
|------|------|----------------------------|----------------------------------|
| A — `x` | argument | `== 1` vs `!= 1` | `if (x != 1)` |
| B — `local_y` | argument (written into file-scope `static int y`) | `== 2` vs `!= 2` | `y = local_y;` then `if (y != 2)` |
| C — `z` | argument | `== 3` vs `!= 3` | `if (z != 3)` |
| D — call sequence / residual state | process state | 1st call vs Nth call; `static int y = 123` initial value | `static int y = 123;` is file-scope and mutated by every `driver` call |
| E — cross-library independence | process state | C `.so` and Rust `.so` each own a private `y` | both loaded into one process; each has its own local `y` |
| F — runtime option / mode / flag | — | **none** | no globals settable from outside, no `#ifdef` in the source, no env var read, no config struct |
| G — input shape (size / width / count / format / byte order / empty-one-many) | — | **none beyond the 3 scalars** | no pointers, buffers, lengths, arrays, or element-type parameters exist |
| H — Cargo features | build config | **none** — `translation/Cargo.toml` declares no `[features]` section, so the default build is the only build | `grep -n '\[features\]' translation/Cargo.toml` → no match |

Axes F and G are empty *because the C has nothing there*, not because they were
skipped: `driver` is the entire public surface and it takes three `int`s.

The full set of public entry points is `{driver}` — there is no lower-level
public API hidden behind it (`multi_stage` is `static`, i.e. not linkable), so
"exercise the lowest-level entry point directly" is satisfied by calling
`driver` itself, which is simultaneously the lowest and highest level.

## Configuration-surface table

Cross-product of A × B × C is 2×2×2 = 8; the C code distinguishes all 8 (the
guard chain makes 4 of them alias to the same message but via different input
combinations, and each is still asserted separately). Axes D and E add
sequencing rows. Every row is driven with **many randomized inputs**
(`SplitMix64`, fixed seed `0x243F6A8885A308D3`), not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | randomized inputs | ✔ |
|---|----------------|-------------------------------------------|-------------------|-----|
| 1 | `driver` | A:`x==1`, B:`y==2`, C:`z==3` — the sole success path | fixed triple (only one member) + repeated | [x] |
| 2 | `driver` | A:`x!=1`, B:`y==2`, C:`z==3` | 256 random `x != 1` | [x] |
| 3 | `driver` | A:`x==1`, B:`y!=2`, C:`z==3` | 256 random `y != 2` | [x] |
| 4 | `driver` | A:`x==1`, B:`y==2`, C:`z!=3` | 256 random `z != 3` | [x] |
| 5 | `driver` | A:`x!=1`, B:`y!=2`, C:`z==3` | 256 random pairs | [x] |
| 6 | `driver` | A:`x!=1`, B:`y==2`, C:`z!=3` | 256 random pairs | [x] |
| 7 | `driver` | A:`x==1`, B:`y!=2`, C:`z!=3` | 256 random pairs | [x] |
| 8 | `driver` | A:`x!=1`, B:`y!=2`, C:`z!=3` — all guards would fail | 256 random triples | [x] |
| 9 | `driver` | unconstrained: full 32-bit random triples over the whole `int` domain (hits rows 1–8 by chance, and every value in between) | 4096 random triples | [x] |
| 10 | `driver` | D: sequence of calls in one process, mixing success and failure configurations, asserting residual `static y` behaves identically | 512-call random sequence | [x] |
| 11 | `driver` | D: first-ever call in the process (initial `y == 123` still in effect) — confirms `y = local_y` happens before the read, so `123` is unobservable | 1 dedicated fresh-process-order test | [x] |
| 12 | `driver` | E: C and Rust `.so` calls interleaved in one process, each library's `y` independent | 512 interleaved calls | [x] |
| 13 | `driver` | D+A/B/C: same argument triple called repeatedly (idempotence of output) | 64 triples × 3 repeats | [x] |
| 14 | `driver` | boundary shapes: `{INT_MIN, -1, 0, 1, 2, 3, 4, INT_MAX}` cross-product in all three slots | 8³ = 512 exhaustive triples | [x] |
| 15 | `driver` | H: no Cargo features exist, so the default build is the only feature combination; the table above **is** the complete per-combination matrix | n/a | [x] |
