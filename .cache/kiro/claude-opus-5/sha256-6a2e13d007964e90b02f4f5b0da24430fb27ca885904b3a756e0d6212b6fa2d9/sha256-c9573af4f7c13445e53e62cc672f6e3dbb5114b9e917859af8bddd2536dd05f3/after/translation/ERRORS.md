# ERRORS.md — error / rejection surface table

## How this table was derived (mechanical greps over `c_src/src` + `c_src/include`)

```
grep -n "return"                    -> 0 hits in the library
grep -n "assert"                    -> 0 hits
grep -n "NULL\|nullptr"             -> 0 hits
grep -ni "error\|errno\|fail\|invalid" -> 0 hits
grep -n "if\|switch\|#ifdef\|#if "  -> 0 hits
grep -n "MIN\|MAX\|#define\|enum"   -> 0 hits
```

(The only hits under `c_src/` come from `build/CMakeFiles/.../CMakeCCompilerId.c`,
CMake's compiler-identification probe, which is not part of the library.)

**Conclusion: the C library has NO explicit error surface.** `flip_horizontal`
returns `void`, validates nothing, has no error enum, no sentinel and no
`assert`. Therefore every row below is an *implicit* rejection: a condition on
which the C code silently degenerates to a no-op because of its loop bounds, or
faults hard. These are the real inputs a caller can pass, and the Rust must
behave identically on each.

Relevant C control flow (`c_src/src/lib.c`), verbatim structure:

```
flips = h / 2;                 // C division truncates toward zero
for (i = 0; i < flips; ++i)    // guard 1: not entered unless flips >= 1
    a = pix + w*i;  b = pix + w*(h-i-1);
    for (j = 0; j < w; ++j)    // guard 2: not entered unless w >= 1
        swap(*a, *b); ++a; ++b;
```

## Error-surface table

| #  | function | trigger (exact invalid input / condition) | expected C result | test | ✅ |
|----|----------|-------------------------------------------|-------------------|------|----|
| 1  | `flip_horizontal` | `img == NULL` | reads `img->pix` unconditionally → SIGSEGV (fatal signal, no error code) | `err_01_null_img_faults_identically` | [x] |
| 2  | `flip_horizontal` | `img->pix == NULL`, `w >= 1`, `h >= 2` | first iteration dereferences `pix + 0` → SIGSEGV | `err_02_null_pix_with_work_faults_identically` | [x] |
| 3  | `flip_horizontal` | `img->pix == NULL`, `h == 0` (any `w`) | `flips == 0`, outer loop never entered → returns normally, no deref | `err_03_null_pix_h0_is_noop` | [x] |
| 4  | `flip_horizontal` | `img->pix == NULL`, `h == 1` (any `w`) | `flips == 0` → returns normally, no deref | `err_04_null_pix_h1_is_noop` | [x] |
| 5  | `flip_horizontal` | `img->pix == NULL`, `w == 0`, `h >= 2` | outer loop runs, inner loop guard `j < 0+1`? no — `j < w` is false → no deref, returns normally | `err_05_null_pix_w0_is_noop` | [x] |
| 6  | `flip_horizontal` | `img->pix == NULL`, `w < 0`, `h >= 2` | offsets `w*i`, `w*(h-i-1)` are *computed* from NULL but never dereferenced (`j < w` false) → returns normally | `err_06_null_pix_wneg_is_noop` | [x] |
| 7  | `flip_horizontal` | `h == 0` | `flips = 0` → buffer untouched | `err_07_h_zero_noop` | [x] |
| 8  | `flip_horizontal` | `h == 1` | `flips = 1/2 = 0` → buffer untouched (odd-height boundary) | `err_08_h_one_noop` | [x] |
| 9  | `flip_horizontal` | `h == -1` | `-1/2` truncates toward zero `= 0` → outer loop not entered → buffer untouched | `err_09_h_neg_one_noop` | [x] |
| 10 | `flip_horizontal` | `h == -2` | `-2/2 = -1`; `0 < -1` false → buffer untouched | `err_10_h_neg_two_noop` | [x] |
| 11 | `flip_horizontal` | `h` arbitrary negative (randomized, incl. odd/even) | `flips <= 0` → buffer untouched | `err_11_h_negative_random_noop` | [x] |
| 12 | `flip_horizontal` | `h == INT_MIN` (one step past the negative range) | `INT_MIN/2 = -1073741824` (well-defined) → loop not entered → untouched | `err_12_h_int_min_noop` | [x] |
| 13 | `flip_horizontal` | `w == 0`, `h >= 2` | outer loop executes `h/2` times; inner guard `0 < 0` false → buffer untouched | `err_13_w_zero_noop` | [x] |
| 14 | `flip_horizontal` | `w == -1`, `h >= 2` | inner guard `0 < -1` false → buffer untouched, despite negative pointer offsets being formed | `err_14_w_neg_one_noop` | [x] |
| 15 | `flip_horizontal` | `w` arbitrary negative (randomized), `h >= 2` | buffer untouched | `err_15_w_negative_random_noop` | [x] |
| 16 | `flip_horizontal` | `w == INT_MIN`, `h >= 2` (one step past negative range; `w*(h-i-1)` overflows `int`) | inner guard false → buffer untouched; the overflowed offset is never dereferenced | `err_16_w_int_min_noop` | [x] |
| 17 | `flip_horizontal` | both `w < 0` and `h < 0` | untouched | `err_17_both_negative_noop` | [x] |
| 18 | `flip_horizontal` | `w == INT_MAX`, `h == 0` / `h == 1` (oversized length with no work to do) | `flips == 0` → untouched, no OOB access | `err_18_w_int_max_h_le1_noop` | [x] |
| 19 | `flip_horizontal` | `h == INT_MAX`, `w == 0` (oversized height, zero-width rows) | `flips = 1073741823` spins with no memory access → untouched, returns normally | `err_19_h_int_max_w0_noop` (bounded surrogate, see note) | [x] |

### Generic FFI boundaries — coverage notes

* **Null pointers**: rows 1–6 (both the `cp_image_t*` argument and the inner
  `pix` field, each crossed with whether the loop guards allow a dereference).
* **Zero lengths**: rows 3, 5, 7, 13 (`w == 0`, `h == 0`, and their crossings).
* **Values one step past a valid range**: rows 9–12 (`h`: `-1`, `-2`, `INT_MIN`),
  rows 14–16 (`w`: `-1`, `INT_MIN`), row 18 (`w == INT_MAX`), row 8 (`h == 1`,
  the largest height that still produces zero flips).
* **Out-of-range enum values across FFI**: **not applicable.** `grep -n "enum"`
  finds no enum in `include/lib.h` or `src/lib.c`; the entire public API is
  `void flip_horizontal(cp_image_t*)`. There is no integer-coded mode/flag
  parameter, so there is no invalid-variant input to construct. Recorded here
  explicitly so the absence is a verified fact rather than an oversight.
* **Oversized lengths that would actually access out of bounds**: see
  "Deliberately NOT differentially tested" below.
* **Row 19 note**: `h == INT_MAX, w == 0` requires 2^30 empty outer iterations.
  Both libraries are verified to be a no-op on bounded surrogates
  (`h = 4_000_001 / 4_000_000 / 8_000_003`, `w = 0`) in the default suite, plus
  the exact `h == INT_MAX` run in `err_19_h_int_max_w0_noop_exact`, which is
  `#[ignore]`d only to keep the default suite fast. It has been **run and
  passes** under all four configurations via `scripts/verify_all.sh --slow`
  (≈4.8 s in release, ≈18 s in dev).

## Divergences found and fixed during Phase C

Two real fidelity problems surfaced from these rows; both were fixed in the
Rust side (the C was never touched).

### 1. Rust's UB checks turned the C's SIGSEGV into SIGABRT (rows 1, 2)

`err_01` initially failed with `C=Signaled(11)` vs `Rust=Signaled(6)`. The C
dereferences `img` with no null check, so an invalid `cp_image_t*` produces a
hardware fault (SIGSEGV). Rust's `debug_assertions`-gated UB checks intercepted
that raw-pointer deref and called `abort()` (SIGABRT) instead — an *observable*
behavioral difference on exactly the inputs under verification. The release
cdylib (what a consumer actually loads) already matched the C.

Fix: `[profile.dev] debug-assertions = false` in `Cargo.toml`, making the dev
cdylib semantically identical to the release one. Rows 1 and 2 now assert the
concrete signal number (`SIGSEGV`), not merely "both died".

### 2. `offset` / `add` are stricter than C's pointer arithmetic (rows 6, 16)

For `w < 0` and `w == INT_MIN` with `h >= 2`, the C *forms* the addresses
`pix + w*i` and `pix + w*(h-i-1)` — wildly out of bounds, and for `INT_MIN` the
`int` multiply even overflows — but never dereferences them, because the inner
guard `j < w` is false. Rust's `ptr::offset` / `ptr::add` require the result to
stay within one allocation, so the faithful translation uses
`wrapping_offset` / `wrapping_add`, which reproduce the same two's-complement
address arithmetic with no in-bounds precondition.

### 3. Fork-safety of the crash-equivalence harness (test-side, not a translation bug)

Rows 1–6 compare *how the process terminates*, which requires running the call
in a forked child. `err_01` initially resolved the two `.so` symbols **inside**
the child. `cargo test` is multi-threaded, so a child forked while another
thread held the `OnceLock` / loader / allocator lock inherited a lock with no
owner and hung forever (intermittently, roughly 1 run in 5). `run_in_child` now
forces `impls()` to completion in the parent and every child does nothing but
the FFI call and `_exit`. Verified stable over 12 consecutive runs and two full
`verify_all.sh --slow` passes.

Separately, the faulting children were each handing a core dump to the system
core handler, which accounted for ~80% of the suite's wall-clock time; the child
now sets `RLIMIT_CORE` to 0 (21 s -> 4.6 s).

## Deliberately NOT differentially tested (C behavior is undefined)

Oversized lengths where the inner loop *does* run against an undersized buffer
(e.g. `w = 1<<20, h = 4` over a 16-pixel allocation), and `w * i` overflowing
`int` while writes actually occur, are heap corruption in the C original.
"Byte-identical" is not a well-defined target for undefined behavior, and the
test process would corrupt its own heap. Rows 16, 18 and 21 cover the
overflow/oversize *value* boundaries in the configurations where the C's loop
guards make the outcome well-defined.
