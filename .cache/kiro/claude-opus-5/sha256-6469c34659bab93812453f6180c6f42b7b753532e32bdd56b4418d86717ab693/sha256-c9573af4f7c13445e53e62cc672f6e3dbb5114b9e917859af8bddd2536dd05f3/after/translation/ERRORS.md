# ERRORS.md — error / rejection surface table

Derived mechanically from the C source, not from docs. A full grep of
`c_src/src/*.c` and `c_src/include/*.h` for `return`, `assert`, `NULL`,
`errno`, `-1`, `if (`, `?`, `#define`, `#if` yields **exactly two `if`/`return`
rejection sites and zero asserts, zero null checks, zero explicit range checks,
zero error enums, and one numeric constant** (`N_SMOOTH == 16`):

```
src/match.c:37:  if(total(test, bins) < threshold * total(reference, bins)) return 0;
src/match.c:40:  return spectral_contrast(t, r, bins) >= threshold;
```

The library has **no error codes and no sentinel returns**. `match`'s "rejection"
is the value `0`; `spectral_contrast` has no rejection path at all — every
`return` is a computed `double`. Rows 1–2 below are therefore the only
*designed* rejections; rows 3–15 are the generic C-API boundaries (null
pointers, zero / negative / oversized lengths, one-past-range values,
out-of-range "enum" ints) that the task requires be covered regardless.

The smallest `bins` for which `match` has **defined** behaviour is `1`; the
smallest `length` for `spectral_contrast` is `0` (and every negative `length` is
defined too, because `i < length` is a signed comparison).

Legend for **expected C result**: `0` / `1` are `match`'s `int`; `+0.0` etc. are
`spectral_contrast`'s `double`; *UB* means the C has undefined behaviour and the
compiled `.so` crashes (established by running it, see `tests/errors.rs`).

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 1 | `match` | energy gate fails: `total(test,bins) < threshold * total(reference,bins)` (`comisd` + `jbe`; unordered/NaN compares false, so NaN does **not** trigger it) | `0`, returned before any preprocessing | `errors.rs::err01` |
| 2 | `match` | contrast gate fails: `spectral_contrast(t,r,bins) >= threshold` is false, incl. the NaN case (`setae` after `comisd` yields 0 when unordered) | `0` | `errors.rs::err02` |
| 3 | `match` | `bins == 0` — zero-length VLA. `differentiate` stores `v[length-1]` = `v[-1]`; the VLA base *is* `match`'s own `%rsp`, so that store lands on the return address `call preprocess` pushed at `%rsp-8` and zeroes it. `preprocess`'s `ret` then jumps to address 0. | *UB* — SIGSEGV (**measured** in `tests/ub.rs`, not assumed). Not reproduced in Rust | `ub.rs::ub_row03*` |
| 4 | `match` | `bins == 0` **and** `test == reference == NULL` — the NULL pointers are never dereferenced, but the row-3 return-address corruption still happens | *UB* — SIGSEGV | `ub.rs::ub_row04*` |
| 5 | `match` | `bins < 0` (`-1`, `-17`, `INT_MIN`) — `float_t t[bins]` is a negative-size VLA (`sub %rax,%rsp` with a ~`2^64` operand) and `preprocess` does `memcpy(v, source, (size_t)length * 8)` = ~`2^64` bytes | *UB* — SIGSEGV. Not reproduced in Rust (see “Deliberate non-reproduction” below) | `ub.rs::ub_row05*` |
| 6 | `match` | `bins > 0`, `test == NULL` — `total` dereferences `NULL` | *UB* — SIGSEGV | `ub.rs::ub_row06_07*` |
| 7 | `match` | `bins > 0`, `reference == NULL` — first `total(test,…)` succeeds, second dereferences `NULL` | *UB* — SIGSEGV | `ub.rs::ub_row06_07*` |
| 8 | `match` | `bins` oversized (`1<<24`, `1<<28`, `INT_MAX`) — the VLAs exceed the stack rlimit; there is no probe and no check | *UB* — SIGSEGV | `ub.rs::ub_row08*` |
| 9 | `match` | `threshold` = NaN — `threshold * total(ref)` is NaN, both `comisd`s are unordered | `0` (gate 1 not taken, gate 2 false) | `errors.rs::err09` |
| 10 | `match` | `threshold` = `-inf` / `+inf`, with `total(reference) == 0` — `inf * 0` = NaN | `0` | `errors.rs::err10` |
| 11 | `spectral_contrast` | `length == 0` — every loop is `for(i=0;i<length;i++)`; `dot_product` returns its initialiser | `+0.0` | `errors.rs::err11` |
| 12 | `spectral_contrast` | `length < 0` (`-1`, `INT_MIN`) — `i < length` is false immediately; `int` comparison, no cast to `size_t`, so **no** crash | `+0.0` | `errors.rs::err12` |
| 13 | `spectral_contrast` | `length <= 0` **and** `a == b == NULL` — pointers never dereferenced | `+0.0` | `errors.rs::err13` |
| 14 | `spectral_contrast` | `length > 0` with `a == NULL` / `b == NULL`, or `length` (`1<<24`, `INT_MAX`) running past the end of a valid buffer | *UB* — SIGSEGV | `ub.rs::ub_row14*` |
| 15 | `spectral_contrast` | all-zero input (`magnitude == 0`) — `normalize` divides by zero: `0.0/0.0` → the x86 “real indefinite” QNaN `0xFFF8000000000000`, narrowed to `0xFFC00000` | `-NaN` (bit-exact `0xFFF8000000000000`) | `errors.rs::err15` |

## Out-of-range enum values

The public API declares **no enum type** — the only integer parameters are
`int bins` / `int length`, whose full `int` range is covered by rows 3, 5, 8,
11, 12 (including `INT_MIN` and `INT_MAX`). There is consequently no
`enum`-with-no-valid-variant case to pass across the FFI boundary. `bins` and
`length` are treated as the "enum-like" ints and every out-of-domain value class
(`< 0`, `0`, `> stack capacity`, `INT_MIN`, `INT_MAX`) is exercised.

## Deliberate non-reproduction of UB (rows 3–8, 14)

For these rows the C `.so` executes undefined behaviour and dies on a fatal
signal. `tests/ub.rs` runs each one in a forked child and **asserts the C
actually crashes**, so the "UB" label above is measured, not assumed; a control
test in the same file asserts the defined cases (`bins ∈ {1,16,17,1024}`,
`length ≤ 0`) exit cleanly, proving the probe can tell the two apart.

The Rust translation does **not** reproduce the crash. It degenerates safely
(`bins <= 0` ⇒ empty buffers ⇒ `(0.0 >= threshold)`), because deliberately
faulting would trade a memory-safety guarantee for bug-compatibility with UB.
`tests/ub.rs` pins that safe behaviour down as well, so it is specified rather
than incidental. This is the only intentional behavioural divergence in the
crate and it is confined to inputs on which the C program has no defined
behaviour at all. **Every input for which the C is defined is byte-identical.**

## How this was verified (reproduce)

```bash
# 1. C reference
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Rust cdylib + symbol parity
cd translation && cargo build --release && ./check_symbols.sh

# 3. Phase B + C differential suite (all rows), every feature combo, both profiles
./check_features.sh

# 4. Heavier randomized pass
DIFF_ITERS=60000 cargo test --release --test configs --test errors

# 5. Prove the suite has teeth
./mutation_check.sh
```

`tests/harness.rs` additionally asserts the two loaded `.so`s are distinct files
and that `match` / `spectral_contrast` resolve to different addresses in each,
so the suite cannot silently compare one implementation against itself.
