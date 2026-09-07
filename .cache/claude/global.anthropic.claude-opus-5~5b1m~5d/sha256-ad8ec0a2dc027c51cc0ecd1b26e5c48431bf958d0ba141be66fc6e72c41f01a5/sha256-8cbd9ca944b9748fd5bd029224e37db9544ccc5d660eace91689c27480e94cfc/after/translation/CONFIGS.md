# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from the `if` / `switch` / guard structure of
`c_src/src/lib.c` plus the public header `c_src/include/lib.h`.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the
only build configuration is the default one. There is no `#ifdef` /
`#if` / conditional compilation anywhere in `c_src` either
(`grep -c '#if' c_src/src/lib.c` = 0). Enumerated mechanically:

```
default (no features)      <- the only combination
```

Therefore "repeat under every feature combination" collapses to the one
combination, but the suite is still run as `--no-default-features` and
as default to prove it.

There is **no binary/driver target** in either tree (the C
`CMakeLists.txt` only `add_library(... SHARED)`; the Rust `[lib]` is
`crate-type = ["cdylib"]`), so the "compare binary stdout" gate is N/A.

## Axes the C code actually branches on

| axis | values the code distinguishes | source |
|---|---|---|
| A. `arity` `len` (after `mov %al` truncation to `u8`) | `<2` (-> `-1`), `==2`, `==3`, `>=4` | `arity` |
| B. `apply_bitmask` `operation` | `0` (`&0xF0`), `1` (`&0x0F`), `2` (`\|0xAA`), `3` (`^0x55`), anything else (identity) | `apply_bitmask` switch |
| C. `arity4` `param1 % 4` (drives axis B from inside `arity4`) | `0,1,2,3` for `param1>=0`; `0,-1,-2,-3` for `param1<0` (negative -> identity) | `arity4` |
| D. `arity4` `param3` | `==0` (skip), `>0`, `<0` (negative truncating division), overflow-inducing | `arity4` |
| E. `arity4` `param4` | `==0` (skip), `!=0` | `arity4` |
| F. `compare_allocations` `val1` sign | `>0` (`+10`), `<=0` (`+0`) | ternary on `*uninit_ptr` |
| G. `compare_allocations` allocator state | `ptr1<ptr2` (`1`) vs `ptr1>ptr2` (`2`) — depends on **global glibc tcache LIFO order**, i.e. on the *sequence* of prior calls, not on the arguments | pointer comparison |
| H. `shift_array` guard | `0 < positions < size` (act) vs otherwise (no-op) | `shift_array` |
| I. `shift_array` shape | `size` = 1/2/4/many; `positions` = 1 / `size-1` / middle | memmove length `(size-positions)*4` |
| J. `process_string` | empty (`0`), 1 char, many chars, embedded high bytes | `if (*str)` + `strlen` |
| K. entry point | `shift_array`, `process_string`, `apply_bitmask`, `init_matrix`, `compare_allocations` (low level) / `arity4` / `arity3` / `arity2` / `arity` (composed) | header + `nm -D` |

### Critical note on axis G (drives the whole test design)

`compare_allocations` does `malloc(4)` twice, compares the two
**addresses**, then `free`s them in order `ptr1, ptr2`. glibc's tcache
is LIFO, so the *next* call gets them back swapped. Measured:

```
8 consecutive calls, same args (5,6): [11, 11, 12, 11, 12, 12, 11, 12]
```

The return value therefore depends on **process-global allocator
history**, not on the inputs. Both `.so`s share one glibc heap when
`dlopen`ed into the same process, so naively interleaving
`c(x); r(x); c(x); r(x)` yields `[(11,12),(11,12),...]` — a *false*
divergence that is an artifact of the shared heap.

**Consequence:** axis G is turned from nondeterministic noise into a
*controlled input axis*. Immediately before every call that reaches
`compare_allocations`, the harness normalizes the glibc tcache 32-byte
bin:

```rust
let a = malloc(4); let b = malloc(4);
let (lo, hi) = if a < b { (a, b) } else { (b, a) };
// tcache is LIFO: the LAST pointer freed is handed back FIRST.
if want_ascending { free(hi); free(lo); }   // -> ptr1 < ptr2, result 1
else              { free(lo); free(hi); }   // -> ptr1 > ptr2, result 2
```

Verified deterministic for both libraries and both arms:

```
asc=true   compare_allocations( 5,6) -> (C 11, Rust 11) x8
asc=true   compare_allocations(-5,6) -> (C  1, Rust  1) x8
asc=false  compare_allocations( 5,6) -> (C 12, Rust 12) x8
asc=false  compare_allocations(-5,6) -> (C  2, Rust  2) x8
```

So `asc` becomes an explicit configuration axis (`G=asc` / `G=desc`) and
**both** pointer-ordering arms of `compare_allocations` are exercised
deliberately, in-process, pairwise. Rows below marked `norm` are run
under *both* `G=asc` and `G=desc`.

## Configuration table

`R` = randomized, fixed seed (`SplitMix64`, seed `0x5EED_1234_ABCD`),
1000+ vectors per row unless stated. `pair` = in-process pairwise
compare. `norm` = in-process pairwise compare with the tcache normalized
before *every* call, run twice: once with `G=asc` and once with `G=desc`.

| # | entry point(s) | configuration (options set + input shape) | mode | pass |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `apply_bitmask` | `operation = 0`, `value` R over full `i32` range | pair | [x] |
| 2 | `apply_bitmask` | `operation = 1`, `value` R over full `i32` range | pair | [x] |
| 3 | `apply_bitmask` | `operation = 2`, `value` R over full `i32` range | pair | [x] |
| 4 | `apply_bitmask` | `operation = 3`, `value` R over full `i32` range | pair | [x] |
| 5 | `apply_bitmask` | `operation` R over full `i32` (mostly `default:`) x `value` R; plus exhaustive `operation` in `-8..=8` and `{INT_MIN, INT_MAX}` | pair | [x] |
| 6 | `apply_bitmask` | boundary `value`s: `0, ±1, 0xFF, 0xF0, 0x0F, 0xAA, 0x55, INT_MIN, INT_MAX` x `operation 0..=4` cross-product | pair | [x] |
| 7 | `process_string` | length 1 byte, all 255 non-NUL first-byte values | pair | [x] |
| 8 | `process_string` | length 2..64, R bytes in `1..=255` (no interior NUL) | pair | [x] |
| 9 | `process_string` | long string (256, 1024, 4096 bytes) | pair | [x] |
| 10 | `process_string` | interior NUL at position `k` (`strlen` stops early, buffer longer) | pair | [x] |
| 11 | `process_string` | high-bit bytes only (`0x80..0xFF`) — `char` signedness of `if (*str)` | pair | [x] |
| 12 | `init_matrix` | writes exactly 12 `int`s, `1..12` row-major; verify with guard cells before/after the 12-slot buffer | pair | [x] |
| 13 | `init_matrix` | called twice on the same buffer / on a pre-dirtied buffer (R fill) — must fully overwrite | pair | [x] |
| 14 | `shift_array` | `size = 4`, `positions = 1` (the shape `arity4` uses), R contents | pair | [x] |
| 15 | `shift_array` | `size` R in `1..=64` x `positions` R in `1..size` (guard passes), R contents | pair | [x] |
| 16 | `shift_array` | `positions = size - 1` (memmove length exactly 1 elem) | pair | [x] |
| 17 | `shift_array` | `positions` in the middle, large `size` (`32`, `64`) — overlapping memmove | pair | [x] |
| 18 | `shift_array` | `size = 1` (no valid `positions`; guard always false) | pair | [x] |
| 19 | `shift_array` | `size = 2`, `positions = 1` (smallest acting case) | pair | [x] |
| 20 | `compare_allocations` | `val1 > 0` (`+10` arm), R `val1` in `1..=INT_MAX`, R `val2` | norm | [x] |
| 21 | `compare_allocations` | `val1 <= 0` (`+0` arm), R `val1` in `INT_MIN..=0`, R `val2` | norm | [x] |
| 22 | `compare_allocations` | long run (512 calls) of R `(val1, val2)` — pins the full tcache LIFO sequence | norm | [x] |
| 23 | `arity4` | `param1 % 4 == 0` (`param1 = 4k, k>=0`) x `param3 = 0` x `param4 = 0` | norm | [x] |
| 24 | `arity4` | `param1 % 4 == 1` x `param3 = 0` x `param4 = 0` | norm | [x] |
| 25 | `arity4` | `param1 % 4 == 2` x `param3 = 0` x `param4 = 0` | norm | [x] |
| 26 | `arity4` | `param1 % 4 == 3` x `param3 = 0` x `param4 = 0` | norm | [x] |
| 27 | `arity4` | `param1 < 0` so `param1 % 4` in `{-1,-2,-3}` -> `default:` identity x `param3 = 0` x `param4 = 0` | norm | [x] |
| 28 | `arity4` | `param1 < 0`, `param1 % 4 == 0` (`param1 = -4k`) x `param3 = 0` x `param4 = 0` | norm | [x] |
| 29 | `arity4` | `param3 > 0` (`1, 2, 99, 100, 101, R`) x `param4 = 0`, R `param1`/`param2` | norm | [x] |
| 30 | `arity4` | `param3 < 0` (negative truncating division) x `param4 = 0`, R `param1`/`param2` | norm | [x] |
| 31 | `arity4` | `param3 = 0` x `param4 != 0` (R, both signs) | norm | [x] |
| 32 | `arity4` | `param3 != 0` **and** `param4 != 0` (both steps run, R both signs) | norm | [x] |
| 33 | `arity4` | full-range R `(param1, param2, param3, param4)` over all of `i32`, 2000 vectors | norm | [x] |
| 34 | `arity4` | boundary cross-product: each param in `{INT_MIN, -101, -100, -1, 0, 1, 100, 101, INT_MAX}` (9^4 = 6561 combos, all of them) | norm | [x] |
| 35 | `arity2` | R `(p1, p2)` full range — must equal `arity4(p1,p2,0,0)` | norm | [x] |
| 36 | `arity3` | R `(p1, p2, p3)` full range, incl. `p3 = 0` and boundary `p3` | norm | [x] |
| 37 | `arity` | `len = 2` -> `arity2`, R `params[0..2]` | norm | [x] |
| 38 | `arity` | `len = 3` -> `arity3`, R `params[0..3]` | norm | [x] |
| 39 | `arity` | `len = 4` -> `arity4`, R `params[0..4]` | norm | [x] |
| 40 | `arity` | `len` in `5..=255` (all of them) -> `arity4`, reads only `params[0..4]` | norm | [x] |
| 41 | `arity` | `len` needing truncation: `256..=520` and `{1000, 65535, 65536, INT_MAX, -1, -2, -256, INT_MIN}` x R `params` | norm | [x] |
| 42 | mixed sequence | interleave `arity`, `arity2`, `arity3`, `arity4`, `compare_allocations` in one R-ordered 1000-call sequence — pins composed allocator interaction across entry points | norm | [x] |
