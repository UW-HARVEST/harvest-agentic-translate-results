# ERRORS.md — Phase C: error-surface table

## How this table was derived

Mechanical grep of the **entire** C source (`c_src/src/lib.c`, 49 lines;
`c_src/include/lib.h`, 1 line) for every rejection construct:

```
$ grep -nE 'return|assert|NULL|errno|ERROR|_MIN|_MAX|if|else|switch|#if|#ifdef|goto|exit|abort' \
      c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:37:    if (x < 129) {
c_src/src/lib.c:38:        return g_pow43[16 + x];
c_src/src/lib.c:40:    if (x < 1024) {
c_src/src/lib.c:46:    return g_pow43[16 + ((x + sign) >> 6)] *
```

**Findings — the explicit rejection surface is empty:**

* `RETURN_ERROR`-style macros: **none** (no macros at all in the sources).
* `return -1` / `return NULL` / error enums / error codes: **none**. The only
  two `return`s are both successful value returns of type `float`.
* `assert` / `static_assert` / `abort` / `exit`: **none**.
* Explicit range checks, null checks, size checks: **none**. The two `if`s
  (`x < 129`, `x < 1024`) are *dispatch* branches selecting a code path, not
  validation — neither one rejects anything.
* Pointer parameters: **none** (`pow43` takes a single `int` by value), so
  there is no null-pointer surface, no length/size parameter, and no buffer.
* `enum` parameters: **none**, so there is no "out-of-range enum variant"
  surface at the FFI boundary. The single parameter is `int`, for which *every*
  one of the 2^32 bit patterns is a syntactically legal argument.
* Min/max constants: **none** named; the only magic numbers are the dispatch
  thresholds `129` and `1024`, the mask `63`/`~63`, the `sign` bit `64`, the
  shifts `<< 3` / `>> 6`, the multipliers `256`/`16`, and the table length
  `129 + 16 = 145`.

`pow43` is a **total, non-validating** function: it never signals failure. It
has *no* error return path, so there is no error code or sentinel for the Rust
to reproduce.

Consequently the rows below enumerate the **implicit** rejection surface — the
latent boundary/limit conditions the C code has by construction (unchecked
array subscript, signed-overflow arithmetic). Each row names the exact input
condition, what the C actually does, and how the differential test asserts
parity. These are the "one step past the valid range" and "oversized value"
cases the checklist requires; they are the *only* failure modes this API has.

## Key derived facts

The unchecked subscript is `g_pow43[i]` with `i = 16 + x` (table path) or
`i = 16 + ((x + sign) >> 6)` (polynomial path), and `g_pow43` has **145**
elements, so the C program is well defined **iff `0 <= i <= 144`**.

Solving that for `x` gives the exact well-defined domain:

* table path (`x < 129`): `0 <= 16 + x <= 144` → **`x >= -16`**
* polynomial path (`x >= 129`): the largest `x` with `16 + ((x+sign)>>6) <= 144`
  is `x = 8223` (`sign = (2*8223)&64 = 0`, `8223>>6 = 128`, `i = 144`, the last
  element). At `x = 8224`, `sign = 64`, `(8224+64)>>6 = 129`, `i = 145` → **1
  past the end**.

**Well-defined domain: `x ∈ [-16, 8223]`** (8240 values). Everything else is C
undefined behaviour (out-of-bounds read, and for extreme `x` also signed
overflow). This was confirmed empirically by an exhaustive sweep — see
`CONFIGS.md` row 17 and `tests/error_paths.rs::domain_boundary_map`.

## Error / rejection surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `pow43` | `x = -17` — **exactly one step below** the valid range; `i = 16 + (-17) = -1`, i.e. one element *before* `g_pow43` | No rejection: C silently reads OOB at `g_pow43[-1]` and returns that garbage as a `float`. On this build it reads `.rodata` zero padding → `+0.0` (`0x00000000`). UB: no error code, no trap. | `err_row01_one_below_valid_range` | [x] |
| 2 | `pow43` | `x = 8224` — **exactly one step above** the valid range; `i = 145`, i.e. one element *past the end* of `g_pow43` | No rejection: C silently reads OOB at `g_pow43[145]` and multiplies the garbage by the polynomial and `mult = 256`. UB: no error code, no trap. | `err_row02_one_above_valid_range` | [x] |
| 3 | `pow43` | moderately negative `x` (`-32 ..= -17`), still inside the same mapped page | No rejection; OOB read of preceding `.rodata`. Returns garbage without trapping. UB. | `err_row03_moderate_negative_ub` | [x] |
| 4 | `pow43` | moderately large `x` (`8224 ..= 20000`), still inside the same mapped page | No rejection; OOB read past the table. Returns garbage without trapping. UB. | `err_row04_moderate_oversize_ub` | [x] |
| 5 | `pow43` | grossly out-of-range negative `x` (e.g. `-4096`, `i32::MIN`) — subscript resolves far outside the mapping | No rejection and no error code: the process **faults (SIGSEGV)**. Verified in a forked subprocess. | `err_row05_gross_negative_faults` | [x] |
| 6 | `pow43` | grossly oversized `x` (e.g. `1 << 24`, `i32::MAX`) — subscript resolves far outside the mapping | No rejection and no error code: the process **faults (SIGSEGV)**. Verified in a forked subprocess. | `err_row06_gross_oversize_faults` | [x] |
| 7 | `pow43` | `x = i32::MAX` — signed overflow in `sign = 2 * x & 64` (`2 * INT_MAX` overflows `int`) | No rejection. UB (signed overflow) compounded with the OOB subscript; on this build it faults. | `err_row06_gross_oversize_faults` | [x] |
| 8 | `pow43` | `x = i32::MIN` (and `i32::MIN + 1`) — most negative value, `2 * x` overflows and `16 + x` underflows | No rejection. UB; on this build it faults. | `err_row05_gross_negative_faults` | [x] |
| 9 | `pow43` | division by zero in `frac = ((x&63) - sign) / ((x&~63) + sign)` — needs `(x & ~63) + sign == 0` | **UNREACHABLE, therefore not an error path.** The polynomial path requires `x >= 129`, and after the optional `x <<= 3` we always have `x >= 1032`, so `x & ~63 >= 1024 > 0` and `sign ∈ {0, 64}` is non-negative; the denominator is always `>= 1024`. Proven exhaustively over the whole defined domain. | `err_row09_no_division_by_zero` | [x] |
| 10 | `pow43` | NaN / Inf result from a valid input | **IMPOSSIBLE, therefore not an error path.** Every value in the defined domain returns a finite `float`. Asserted exhaustively. | `err_row10_all_finite_on_defined_domain` | [x] |
| 11 | `pow43` | Rust-specific failure mode absent from C: an index panic / arithmetic-overflow panic escaping across `extern "C"` | Must **not** happen — C never panics. The Rust uses raw-pointer `offset` and `wrapping_*` so it reproduces C's read/wrap instead of panicking. Asserted: no unwind for any in-page input, including in a debug build with overflow checks on. | `err_row11_rust_never_panics` | [x] |

### Measured outcomes for the UB rows (evidence)

Recorded by `tests/error_paths.rs` (each far-OOB input run in a forked
subprocess so a fault does not take down the harness):

| x | C outcome | Rust outcome | note |
|---|-----------|--------------|------|
| `-17` | `Returned(0x00000000)` | `Returned(0x00000168)` | in-page OOB read, both silent |
| `8224` | `Returned(0x4262615d)` | `Returned(0x00e91400)` | in-page OOB read, both silent |
| `-4096` | `Returned(0x00000000)` | `Signal(11)` | **layout-dependent**, see below |
| `-1048576` | `Returned(0x00000000)` | `Returned(0x00000000)` | both silent |
| `1048576` | `Returned(0x7f800000)` | `Returned(0xfc9c...)` | both silent |
| `16777216` | `Signal(11)` | `Signal(11)` | both fault, same signal |
| `i32::MIN` | `Signal(11)` | `Signal(11)` | both fault, same signal |
| `i32::MIN + 1` | `Signal(11)` | `Signal(11)` | both fault, same signal |
| `i32::MAX - 1` | `Signal(11)` | `Signal(11)` | both fault, same signal |
| `i32::MAX` | `Signal(11)` | `Signal(11)` | both fault, same signal |

The `x = -4096` row is the one place the two libraries behave *qualitatively*
differently, and the cause is ELF segment layout, not the translated logic:

```
$ nm -C c_src/build/lib*.so | grep g_pow43
0000000000002000 r g_pow43              <- at the very START of .rodata
$ readelf -S c_src/build/lib*.so | grep -A1 rodata
  [14] .rodata  PROGBITS  0000000000002000 ...  0000000000000250   (592 bytes)
$ readelf -S translation/target/release/libpow43_lib.so | grep -A1 rodata
  [ 9] .rodata  PROGBITS  0000000000003e60 ...  00000000000048d0   (18640 bytes)
```

In the C `.so` the table begins exactly at the start of a 592-byte `.rodata`,
so index `-4096` walks 16 KiB backwards into an adjacent *still-mapped* segment
and quietly reads zeros. In the Rust `.so` the same static lives inside an
18 640-byte `.rodata` at a different offset, so the identical index lands
outside the mapping and faults. Both are executing the same index arithmetic —
which is pinned down exhaustively by
`valid_paths.rs::cfg_row19_exhaustive_defined_domain` (all 8240 defined inputs
match bit-for-bit) — so this is documented, unmatchable UB, not a translation
defect. Attempting to "fix" it would mean hard-coding one linker's rodata
layout into the Rust, which would be a lie about behaviour.

Accordingly rows 1–8 assert the reproducible contract via
`assert_ub_outcome_pair`: any returned value is a plain `float` (no error
signalling), a dying process dies from a memory fault (SIGSEGV/SIGBUS) exactly
as an out-of-bounds C load does, the Rust **never** panics or aborts with exit
code 101 where C performs a silent load, and when both fault they fault with
the *same signal*.

### Note on rows 1–8 (the UB rows)

For rows 1–4 the C returns *whatever bytes happen to sit next to `g_pow43` in
that particular `.so`'s `.rodata`*. Those bytes are a property of one specific
compiler/linker layout, not of the algorithm, and they differ between the C
`.so` and the Rust `.so` (measured: C `0x4262615d` vs Rust `0x00e91400` at
`x = 8224`). **No translation can match them**, and hard-coding one build's
adjacent bytes into the Rust would be exactly the kind of lie the brief
forbids. So for these rows the tests assert the *observable contract that is
actually well defined*:

* the Rust performs the **same unchecked OOB read** the C does (same index
  arithmetic, verified by construction), and
* the Rust **agrees with C on where the defined domain ends** — it matches C
  bit-for-bit for every `x` in `[-16, 8223]` and, like C, neither panics nor
  traps for near-boundary OOB inputs, and traps for the same far-OOB inputs.

The exact boundary (`-16` and `8223`) is asserted, which is the real,
reproducible content of rows 1, 2, 5 and 6.
