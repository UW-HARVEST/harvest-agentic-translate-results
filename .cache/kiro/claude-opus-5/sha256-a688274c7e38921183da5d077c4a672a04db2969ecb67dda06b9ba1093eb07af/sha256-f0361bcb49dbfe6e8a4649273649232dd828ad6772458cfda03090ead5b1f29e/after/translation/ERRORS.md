# ERRORS.md — Phase C error / rejection surface table

## How this table was derived (mechanically, not from docs)

```
grep -nE 'return|assert|NULL|errno|error|ERROR|exit|abort|if *\(|switch|#if|<|>|==|!=' \
     c_src/src/driver.c c_src/include/driver.h
```
after stripping the license comment block yields **exactly two matches**:

```
src/driver.c:30:    for (int i = 0; i < len; i++) {     <- fma_array loop guard
src/driver.c:37:    for (int i = 0; i < len; i++) {     <- inner  print loop guard
```

Therefore the C source contains:

* **0** `return -1` / `return NULL` / `RETURN_ERROR`-style statements
  (both public functions are `void` and have no `return` statement at all),
* **0** `assert()` calls,
* **0** explicit range checks, null checks, or min/max constants,
* **0** enums — so there is **no out-of-range-enum input class for this API**
  (the only scalar parameter is a plain `int len`; every one of its 2^32 values
  is an "in-range" argument as far as the C is concerned).

The library's entire rejection behaviour is therefore *implicit*: it consists of
the two loop guards silently treating non-positive `len` as "nothing to do", plus
the ABI-level boundaries (null pointers, degenerate and oversized lengths,
signed-overflow values). Every one of those is enumerated below — one row per
distinct condition the C actually distinguishes.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `fma_array` | `len == 0`, valid non-null buffers | loop guard `0 < 0` false → returns without writing a single byte; `out` unmodified | [x] PASS |
| 2 | `fma_array` | `len == -1` (one step past the low end of the useful range), valid buffers | negative length silently accepted as empty; no writes; `out` unmodified | [x] PASS |
| 3 | `fma_array` | `len == INT_MIN`, valid buffers | same as row 2: no writes, no crash | [x] PASS |
| 4 | `fma_array` | `len == 0` **and** all four pointers `NULL` | no dereference occurs → returns cleanly (null pointers never loaded) | [x] PASS |
| 5 | `fma_array` | `len < 0` **and** all four pointers `NULL` | no dereference occurs → returns cleanly | [x] PASS |
| 6 | `fma_array` | `mul1[i]*mul2[i]` overflows `int` (e.g. `INT_MAX * 2`, `INT_MIN * -1`, `65536*65536`) — signed overflow, UB in C, `imul` in the emitted code | wrapped two's-complement 32-bit product | [x] PASS |
| 7 | `fma_array` | `mul1[i]*mul2[i] + add[i]` overflows `int` (product in range, sum out of range) | wrapped two's-complement 32-bit sum | [x] PASS |
| 8 | `fma_array` | *both* the multiply and the add overflow, at `INT_MIN`/`INT_MAX` extremes | double-wrapped 32-bit result | [x] PASS |
| 9 | `fma_array` | `out` fully aliases `mul1`,`mul2`,`add` (the exact call `inner` makes) — writes are observed by later reads | in-place sequential update: `out[i] = out[i]*out[i] + out[i]` element by element | [x] PASS |
| 10 | `driver` | `len == 0`, valid `data` | VLA `int out[0]`, `memcpy(...,0)`, both loops skipped → **empty stdout**, no crash | [x] PASS |
| 11 | `driver` | `len == 0`, `data == NULL` | `memcpy(dst, NULL, 0)` copies nothing → **empty stdout**, no crash | [x] PASS |
| 12 | `driver` | `len == 1` (minimum non-degenerate length) | one line of stdout | [x] PASS |
| 13 | `driver` | `len < 0` (`-1`, `INT_MIN`) | `len * sizeof(int)` converts `int`→`size_t` (sign-extend) giving `0xFFFF_FFFF_FFFF_FFFC`, so `memcpy` copies an astronomical byte count → the **process dies on SIGSEGV before any output**. Verified differentially in a *child process*: C and Rust must die with the identical signal and produce identical (empty) stdout. | [x] PASS |
| 14 | `driver` | `len` so large the VLA exceeds the stack (e.g. `len == 1<<30`) | stack-clash → **SIGSEGV**, no output. Same child-process differential check as row 13. | [x] PASS |
| 15 | `driver` | data values that make `out[i]*out[i]+out[i]` overflow (`INT_MIN`, `INT_MAX`, `0x7fffffff`, `46341`) | wrapped 32-bit values printed with `%d` | [x] PASS |

Rows 13 and 14 are undefined behaviour in ISO C; they are nevertheless *real
inputs an external caller can pass*, so they are verified as differential
**crash-equivalence** tests in forked child processes (same terminating signal,
same stdout) rather than being skipped.

Deliberately **not** given a row, with justification:

* `fma_array` with `len` larger than the actual buffers, or `driver(data, len)`
  with `len` larger than `data` — out-of-bounds *reads of the caller's memory*.
  The C performs no bounds check (there is nothing to compare against: no length
  is passed for the buffers other than `len` itself), so the "expected C result"
  is "read whatever is adjacent", which is not a deterministic value in either
  language and therefore cannot have a byte-identical expectation. Row 13/14
  cover the observable, deterministic part of this class (the crash).

## Divergences actually found and fixed (both in `driver`, both in rows 13/14)

Every `fma_array` row and every valid-path row passed on the first run. The two
real translation defects were both in `driver`'s VLA modelling, and both were
only reachable through the ERRORS.md rows — no happy-path test could see them.

**Bug 1 — allocation failure aborted instead of faulting (row 14, `len == INT_MAX`).**
The Rust used `vec![0; len as usize]`, so an ~8 GiB request went to the Rust
global allocator, failed, and was routed through `handle_alloc_error` → `abort()`
→ **SIGABRT (6)**. The C simply moves `rsp` past the stack guard → **SIGSEGV (11)**.
Fixed by switching to a raw, uninitialised `std::alloc::alloc` whose failure is
deliberately *not* routed through `handle_alloc_error`: the resulting null
pointer makes the following `memcpy` fault with SIGSEGV, matching the C.

**Bug 2 — the optimizer deleted the `memcpy` for non-positive `len` (row 13, `len == -1`).**
After bug 1 was fixed the zero-length case was backed by a 4-byte local array, so
a `memcpy` of ~2^64 bytes into it was *provable* UB; LLVM concluded the branch was
unreachable and emitted `test %esi,%esi ; jle <epilogue> ; ret`. The Rust
therefore **exited 0** where the C dies with **SIGSEGV**. Fixed by transliterating
gcc's emitted VLA arithmetic literally —

```
size_bytes = (u64)(i64)len * 4                  // movslq ; lea (,rax,4)
frame      = (size_bytes wrapping+ 15) / 16 * 16 // add ; div $16 ; imul $16
out        = align_up_4(rsp - frame)             // sub %rax,%rsp ; (rsp+3)>>2<<2
memcpy(out, data, size_bytes)                    // count is size_bytes, not frame
```

— and passing the destination through `core::hint::black_box` so the call cannot
be reasoned away. The wrapping `+15` is load-bearing: for `len ∈ {-1,-2,-3}` it
wraps and `frame` collapses to `0`, so the C's `out` is a *valid* stack address
and only the count is absurd; for `len <= -4` `frame` is astronomical and `out`
itself is wild. Both sub-cases now die the same way in both libraries.

## Verification commands

```
./verify_all.sh          # builds both libs, runs all phases × all feature combos
```

All 24 CONFIGS.md rows, all 15 ERRORS.md rows and the 4 Phase D gates pass under
`default`, `--no-default-features`, and in both the `release` and `dev` profiles.
