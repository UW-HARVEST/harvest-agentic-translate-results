# CONFIGS.md — configuration-surface table (Phase A → gates Phase B)

Derived mechanically from the C source and the public header, not from what
looks important.

## Axes the C code actually branches on

**Public entry points** (`c_src/include/driver.h`) — the complete set:

- `void driver(int x, int y, int z)` — the *only* public entry point.

**Lower-level entry points:** `static int multi_stage(int x, int z)` is the
lowest-level function in the library, but it has internal linkage in C
(`static`, driver.c:31) and is *not* exported by the C `.so` (confirmed by
`nm -D`). It is therefore not reachable by an external caller and cannot be
driven directly through either `.so`; it is exercised through `driver` in every
row below, with its full input space (`x`, the static `y`, `z`) covered and its
return value observed via the `Result: %d` line.

**Runtime options / modes / flags:** the library has no option setter, no mode
enum, no flag argument, and no `#ifdef` (grep: zero `#if`/`#ifdef` in
driver.c). The only mutable state is the file-scope `static int y = 123`
(driver.c:29), which `driver` writes on every call (driver.c:59). So the
"configuration" axis is exactly: *the value latched into the static `y`*.

**Input shapes the code special-cases:**

- `x`: `== 1` vs `!= 1` (driver.c:33)
- `y` (latched from `local_y`): `== 2` vs `!= 2` (driver.c:39)
- `z`: `== 3` vs `!= 3` (driver.c:45)
- value class within each: the sentinel value, sentinel±1, `0`, negative,
  `INT_MIN`, `INT_MAX`, arbitrary 32-bit garbage
- call-sequence shape (static-`y` persistence): one call, two calls, many
  calls, and alternating success/failure orderings

The cross-product of the three boolean predicates is 8; all 8 are distinguished
by the code (they map onto 4 distinct outputs, but via distinct paths), so all 8
get a row. Rows 9–16 cover value classes and sequence shapes on top.

Every row is driven through **both** `.so` files via `libloading` and asserted
byte-for-byte on stdout (the only observable output; `driver` is `void`).
Each row uses many randomized inputs from a fixed-seed PRNG (seed `0x2545F491_4F6CDD1D`),
except where the row is defined by exact constants.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` → `multi_stage` | `x==1`, `y==2`, `z==3` — the all-valid success path (`Ok!`, `Result: 0`) | [x] |
| 2 | `driver` → `multi_stage` | `x==1`, `y==2`, `z!=3` (randomized `z != 3`) | [x] |
| 3 | `driver` → `multi_stage` | `x==1`, `y!=2` (randomized), `z==3` | [x] |
| 4 | `driver` → `multi_stage` | `x==1`, `y!=2` (randomized), `z!=3` (randomized) | [x] |
| 5 | `driver` → `multi_stage` | `x!=1` (randomized), `y==2`, `z==3` | [x] |
| 6 | `driver` → `multi_stage` | `x!=1` (randomized), `y==2`, `z!=3` (randomized) | [x] |
| 7 | `driver` → `multi_stage` | `x!=1` (randomized), `y!=2` (randomized), `z==3` | [x] |
| 8 | `driver` → `multi_stage` | `x!=1`, `y!=2`, `z!=3` — all three randomized invalid | [x] |
| 9 | `driver` | boundary value class: each argument independently swept over `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, 3, 4, INT_MAX-1, INT_MAX}` (full 11³ = 1331 cross-product, exhaustive) | [x] |
| 10 | `driver` | unconstrained randomized cross-product: all three arguments uniform over the full `i32` range, 4000 draws (fixed seed) | [x] |
| 11 | `driver` | randomized draws biased to the sentinel neighbourhood `{0,1,2,3,4}` in all three positions, 2000 draws — makes success and every guard fire often | [x] |
| 12 | `driver` | static-`y` persistence, shape "one call": a freshly `dlopen`ed pair receives exactly one call (first-ever call, initial `y==123` still in place) | [x] |
| 13 | `driver` | static-`y` persistence, shape "two calls": failure then success, and success then failure, on the same loaded handles — the latched `y` from call 1 must not affect call 2 | [x] |
| 14 | `driver` | static-`y` persistence, shape "many calls": 3000-call randomized sequence on one pair of handles, comparing the entire concatenated transcript byte-for-byte (catches any state divergence that a per-call comparison would reset away) | [x] |
| 15 | `driver` | stdout formatting/width shape: the `%d` conversion in `printf("Result: %d\n")` driven at every produced value `{0,1,2,3}`; plus confirmation that the fail path emits exactly 3 lines and the success path exactly 2 | [x] |
| 16 | `driver` | interleaving shape: C and Rust calls interleaved on the *same* redirected stdout stream within one capture, verifying the two libraries share libc stdio buffering identically (C's `puts`-optimised literals vs Rust's `printf` must still produce the same bytes in the same order) | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains only `add_library(driver SHARED src/driver.c)`
— there is no `add_executable`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`.
**The project builds no binary driver**, so the "compare C and Rust binary
stdout" gate is not applicable. Its intent is nevertheless satisfied: every row
above compares stdout byte-for-byte, which is the library's entire output.

## Feature combinations

No `[features]` in `Cargo.toml` and no `#[cfg(feature = ...)]` in the crate, so
the default build is the only combination; `--no-default-features` and
`--all-features` are identical to it. All three are run in
`scripts/verify.sh`.
