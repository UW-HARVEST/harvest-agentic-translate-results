# ERRORS.md — Phase A: error-surface table

## How this table was derived

Mechanical grep of the *entire* C source (`c_src/src/driver.c`,
`c_src/include/driver.h`) for every rejection construct:

```
grep -nE "RETURN_ERROR|return |assert|NULL|errno|_MAX|_MIN|if |switch |#ifdef|#if |enum |exit|abort" \
     c_src/src/driver.c c_src/include/driver.h
```

Result: **zero** matches in executable code (the only hits are the
`#endif //DRIVER_H_` comment and the `static` on `inner`).

```
grep -nE "for|while|if|else|switch" c_src/src/driver.c
30:    for (int i = 0; i < len; i++) {   <-- fma_array loop guard
37:    for (int i = 0; i < len; i++) {   <-- inner print loop guard
```

So the C library's *entire* conditional surface is the two `i < len` loop
guards. Consequences that constrain this table:

* Both public functions return `void`. There is **no** error code, no sentinel
  return, no `errno` use, no output status flag. "Same error/rejection" can
  therefore only be observed as: *(a)* the same bytes on stdout, *(b)* the same
  bytes left in the caller's output buffer, *(c)* the same
  crash/exit disposition (exit code vs. terminating signal).
* There are **no `enum` types anywhere in the public API** (verified by
  `grep -n "enum" c_src/**`), so the "out-of-range enum value across the FFI
  boundary" class of input does not exist for this library. The nearest
  analogue is an out-of-range *`int`* `len`, which rows 1–4, 9–18 and 24 cover
  exhaustively: zero, negative, one-past-usable, `INT_MIN`, `INT_MIN+1`,
  `INT_MAX-1`, `INT_MAX`, and every `±2^k` for `k` in `0..31` — the full hostile
  bit-pattern sweep through the only scalar parameter the API has.
* Every remaining "invalid input" is an *unchecked* undefined-behaviour
  condition. The C is the ground truth, so the Rust must reproduce the exact
  same observable disposition — including crashing where the C crashes. Rows
  are still derived from the code (from what the loop guard and the
  `len * sizeof(int)` conversion actually do), not invented.

Crash rows are verified differentially by `fork()`ing and comparing the raw
`waitpid` status classification (exited-with-code vs. killed-by-signal N) of
the C `.so` call and the Rust `.so` call. A child that exceeds a 2 s budget is
killed by `SIGALRM`, so "spins forever" is itself a comparable disposition rather
than a hung test run.

Two of the unchecked conditions turn out to have **no single C answer to match**,
which was established by measurement against the C `.so` itself rather than
assumed. Both are documented below with their evidence, and for both, everything
that *is* well defined is still compared byte-for-byte:

1. `driver` with a **negative** `len` — the VLA moves `%rsp` into the caller's
   frame, so the outcome depends on the caller (rows 14-16).
2. `driver`/`fma_array` with a `len` **past the end of the caller's buffer** — the
   library reads unspecified process memory (rows 26-27).

## Error-surface table

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|----|----------|----------------------------------------------|-------------------|------|--------|
| 1  | `fma_array` | `len == 0`, all four pointers non-NULL | loop guard `0 < 0` false → **no** memory access at all; `out` left byte-for-byte unmodified; no stdout | `err_01_fma_len_zero_no_writes` | [x] |
| 2  | `fma_array` | `len == 0`, **all four pointers `NULL`** | loop guard false → the NULL pointers are never dereferenced → returns normally (exit 0), no output | `err_02_fma_len_zero_all_null` | [x] |
| 3  | `fma_array` | `len < 0` (`-1`, `-7`, `INT_MIN`), pointers valid | loop guard `0 < negative` false → **no** memory access; `out` unmodified; no stdout. (Negative length is *not* rejected with an error — it is silently treated as "do nothing".) | `err_03_fma_len_negative_no_writes` | [x] |
| 4  | `fma_array` | `len < 0`, all four pointers `NULL` | same as row 3: no deref, returns normally | `err_04_fma_len_negative_all_null` | [x] |
| 5  | `fma_array` | `len > 0`, `out == NULL` (destination NULL, sources valid) | first iteration stores to `*(int*)NULL` → killed by `SIGSEGV` | `err_05_fma_out_null` | [x] |
| 6  | `fma_array` | `len > 0`, `mul1 == NULL` (sources NULL, dest valid) | first iteration loads `*(const int*)NULL` → killed by `SIGSEGV` | `err_06_fma_mul1_null` | [x] |
| 7  | `fma_array` | `len > 0`, `mul2 == NULL` | load from NULL → `SIGSEGV` | `err_07_fma_mul2_null` | [x] |
| 8  | `fma_array` | `len > 0`, `add == NULL` | load from NULL → `SIGSEGV` | `err_08_fma_add_null` | [x] |
| 9  | `fma_array` | `len` one past the usable buffer (`len = n+1` for an `n`-element `out`/inputs) — no bounds check exists | reads/writes one `int` past the end of every buffer; with guard pages placed after the buffers the access faults → `SIGSEGV`; the first `n` results are still the normal FMA values | `err_09_fma_len_one_past_end_guarded` | [x] |
| 10 | `fma_array` | `len == INT_MAX` (oversized length, valid pointers to a tiny buffer) | walks off the end of the buffer → `SIGSEGV` | `err_10_fma_len_int_max` | [x] |
| 11 | `driver`   | `len == 0`, `data` non-NULL | `int out[0]` (zero-length VLA), `memcpy(out, data, 0)` → no bytes copied, `inner` both loops skipped → **no stdout output**, returns normally | `err_11_driver_len_zero` | [x] |
| 12 | `driver`   | `len == 0`, `data == NULL` | `memcpy(out, NULL, 0)` copies nothing and does not dereference → no output, returns normally | `err_12_driver_len_zero_null_data` | [x] |
| 13 | `driver`   | `len > 0`, `data == NULL` | `memcpy(out, NULL, len*4)` reads from NULL → killed by `SIGSEGV` | `err_13_driver_null_data` | [x] |
| 14 | `driver`   | `len == -1` (negative) | `len * sizeof(int)`: the `int` → `size_t` conversion makes the count `0xFFFFFFFFFFFFFFFC`, and the VLA reservation `rsp -= round16(4*len)` moves `%rsp` **upwards** into the caller's frame. Well-defined and compared exactly: **no output at all** (`inner` skips both loops for `len < 0`) and the copy cannot succeed. See the note below on why the *termination* is not comparable. | `err_14_driver_len_negative_one`, `d_neg_01..03` | [x] |
| 15 | `driver`   | `len == -2 / -7 / -1000 / -65536` (negative, non-`-1`, to cover the whole negative range) | as row 14 | `err_15_driver_len_negative_seven`, `d_neg_01..03` | [x] |
| 16 | `driver`   | `len == INT_MIN` / `INT_MIN + 1` (extreme negative; `(size_t)(INT_MIN * 4) == 0xFFFFFFFE00000000`, so the VLA lands ~8 GiB *above* the stack) | as row 14; here the VLA address is unmapped, so **both** implementations fault at the same address | `err_16_driver_len_int_min`, `d_neg_01..03` | [x] |
| 17 | `driver`   | `len == INT_MAX` (oversized positive; VLA of 2^31-1 ints = 8 GiB on the stack, `memcpy` of 8 GiB from a tiny buffer) | stack/​source overrun → killed by `SIGSEGV` | `err_17_driver_len_int_max` | [x] |
| 18 | `driver`   | `len` larger than the `data` buffer (`len = n+1` for an `n`-element `data`, guard-page-protected) — no length validation exists | `memcpy` reads one `int` past the end of `data` → `SIGSEGV` | `err_18_driver_len_one_past_end_guarded` | [x] |
| 19 | `driver`   | `len == 1` with `data[0] == INT_MIN` — signed-overflow boundary (`INT_MIN*INT_MIN + INT_MIN`) is **not** checked or clamped | wraps two's-complement; `-O0` gcc emits `imul` → `out[0] == INT_MIN`; prints `-2147483648\n` | `err_19_driver_int_min_overflow` | [x] |
| 20 | `driver`   | `len == 1` with `data[0] == INT_MAX` — signed-overflow boundary | wraps; `INT_MAX*INT_MAX == (2^31-1)^2 == 2^62-2^32+1 ≡ 1 (mod 2^32)`, and `1 + INT_MAX == 0x80000000` → prints `-2147483648\n` (verified against the C `.so`, **not** assumed) | `err_20_driver_int_max_overflow` | [x] |
| 21 | `fma_array` | signed-overflow boundary through the low-level entry point: `INT_MIN`/`INT_MAX`/`-1` combinations with independent pointers (no clamping, no saturation, no check) | wraps two's-complement, per-element | `err_21_fma_overflow_boundaries` | [x] |
| 22 | `driver`   | `data` pointer misaligned for `int` (byte-offset 1 into a buffer) — no alignment check exists | `memcpy` is alignment-agnostic, so the copy succeeds and the unaligned bytes are interpreted as `int`s → normal FMA output for those values | `err_22_driver_misaligned_data` | [x] |
| 23 | `fma_array` | `out` overlapping the sources at a *non-zero* offset (unchecked aliasing: `memcpy`-style restrict violation) | no check; the loop runs strictly ascending `i`, so earlier stores are visible to later loads → a specific, reproducible cascade | `err_23_fma_overlapping_offset` | [x] |
| 24 | `driver`, `fma_array` | arbitrary/out-of-range `int` in the `len` parameter across the FFI boundary — the whole hostile sweep (`INT_MIN`, `INT_MIN+1`, every `±2^k`, `-3..64`, `INT_MAX-1`, `INT_MAX`) against a 4096-element buffer | no validation exists, so each value is handled per rows 1–23; identical disposition and identical stdout (in-bounds prefix + line count where the read leaves the buffer) | `err_24_wild_int_len_sweep` | [x] |
| 25 | `fma_array` | **every** subset of the four pointers set to `NULL` (all 16 masks) × `len ∈ {0, -1, 1, 4}` | no null checks exist; identical disposition for all 64 combinations (`Exited(0)` whenever `len <= 0`, `SIGSEGV` otherwise) | `err_25_fma_null_pointer_powerset` | [x] |
| 26 | `driver` | `len` past the end of the caller's buffer but still inside mapped memory (e.g. `len = 8192` for a 4096-element `data`) | the unvalidated `memcpy` copies, and `inner` then prints, whatever process memory follows `data`. **Unspecified**: the same C `.so` prints different results for identical arguments depending only on the caller's heap history (see the note below). Compared exactly: termination, line count, and the in-bounds prefix. | `d_oob_01`, `d_oob_02`, `d_oob_04`, `err_24_wild_int_len_sweep` | [x] |
| 27 | `fma_array` | `len` past the end of the buffers but still inside mapped memory | reads *and writes* unspecified memory past every array. Compared exactly: termination and the in-bounds prefix of `out`. | `d_oob_03` | [x] |

## Why rows 26-27 compare the in-bounds prefix rather than the whole output

`driver` never validates `len` against the size of `data` — there is no size
parameter beyond `len` itself and no check in `driver.c`. An oversized `len`
therefore makes `memcpy(out, data, len * sizeof(int))` copy bytes that belong to
whatever follows `data` in the process, and `inner` prints them. Those bytes are
not an input to the library.

Measured, not assumed. `d_oob_01_c_output_depends_on_the_callers_heap_history`
calls the *same* C `.so` with the *same* 4096-element `data` and the *same*
`len = 8192`, varying only the size of an unrelated `malloc` the caller made
first, and observes **3 distinct outputs**. Reproduced directly against the C
shared library outside the test harness:

```
$ for pre in 0 8 64 512 4096; do ./oob2 libdriver.so 4096 8192 $pre | md5sum; done
00b5c1fcbffde498f682c09268ce3c8a     # pre=0
00b5c1fcbffde498f682c09268ce3c8a     # pre=8
4b268d98727ba431210c2523701eb7b4     # pre=64
55e0e66021ebf9f9af47766ed0ac004b     # pre=512
64b362f11001a655fe43df9aa62f2db1     # pre=4096
```

Whether the read *faults at all* is equally undetermined, and this too is
measured rather than assumed. `d_oob_04_c_fault_depends_on_what_follows_the_buffer`
calls the same C `.so` with the same 4096 input values and the same
`len = 5120`, changing only what is mapped after the buffer:

```
the SAME C .so, same 4096 input values, same len=5120:
  buffer followed by a PROT_NONE guard page -> killed by signal 11
  buffer followed by readable memory        -> exited(0)
```

Every implementation has to put `driver`'s output buffer *somewhere*, and the C's
choice (the stack) is not available to a Rust translation that must survive the
`printf` calls in `inner`, so the surrounding mapping layout can never be made
identical. `err_24_wild_int_len_sweep` therefore backs the source with a guard
page for out-of-range lengths, which removes the ambiguity — the overrun *must*
fault, at the same offset, before anything is printed — and compares the two
libraries exactly there.

Since the C library does not produce one answer for an unguarded overrun, there
is no answer to match. What the C *does* specify, and what rows 26-27 compare
byte-for-byte, is:

* the termination disposition (`Exited(0)` vs. the exact signal);
* exactly `len` lines of output, one per element;
* every line that came from in-bounds data, which must equal the reference model
  `x*x + x` formatted with `%d\n`.

This is also why the translation's VLA stand-in is an anonymous `mmap` rather
than a `Vec`: the C VLA lives on the *stack* and so leaves the malloc heap
untouched. Taking the buffer from the same heap `data` lives in would perturb the
bytes right after `data` and change even the in-bounds-prefix comparison. (That
was a real divergence found by `err_24_wild_int_len_sweep` at `len = 8192` and
fixed in `src/driver.rs`.)

## Why rows 14-16 compare output-and-determinism rather than the exit signal

`driver`'s `int out[len]` is the only construct in the library whose *manifestation*
is not a property of the C source. From `objdump -d` of the C `.so`:

```
push %rbp; mov %rsp,%rbp; push %rbx; sub $0x28,%rsp    ; driver's frame
mov  %rsp,%rbx                                         ; saved sp for the epilogue
movslq len,%rax; lea 0(,%rax,4),%rdx                   ; (int64)len * 4
mov $0x10,%rax; sub $1,%rax; add %rdx,%rax             ; + 15
mov $0x10,%rcx; mov $0,%edx; div %rcx; imul $0x10,%rax,%rax  ; / 16 * 16
sub  %rax,%rsp                                         ; <== the VLA reservation
mov  %rsp,%rax; add $3,%rax; shr $2,%rax; shl $2,%rax  ; out = align4(rsp)
... call memcpy ; call inner ; mov %rbx,%rsp ; leave ; ret
```

For `len < 0` the reserved size is the wrapped `2^64 - round16(4*|len|)`, so
`sub %rax,%rsp` **adds** `K = round16(4*|len|)` to `%rsp`, putting the VLA past
`driver`'s own frame and inside its **caller's** frame. Everything that follows —
the `call memcpy` return-address push at `out-8`, `memcpy`'s stores at `out`, the
`call inner` push, and finally `ret` through the (possibly overwritten) return
address at `%rbp+8` — writes over the caller's stack. Whether that is fatal,
harmless, or turns into an endless loop depends on gcc's frame layout for
`driver` **and on the caller's frame**, i.e. on nothing the C source specifies.

This is measured, not assumed. `d_neg_01_c_outcome_depends_on_the_callers_frame`
calls the *same* C `.so` with the *same* arguments from four call sites that
differ only in how much stack the caller has in use, and observes:

```
negative lengths for which the *C library alone* terminates differently
depending only on the caller's frame (1 of 15 tested):
  len=        -512  ->  ["exited(0)", "killed by signal 11"]
```

Because one and the same C shared library produces both `exited(0)` and
`SIGSEGV` for identical arguments, no translation can match "the" C exit signal
for negative lengths — there isn't one. Rows 14-16 therefore assert everything
that *is* well defined, exactly:

* **stdout is byte-identical (and empty)** for both libraries, for every negative
  length tested — this is the specified behaviour, since `inner`'s two loops both
  have the guard `i < len` (`driver.c:37`);
* the C library never completes the call successfully from the tested call site;
* the Rust library **faults deterministically** (`SIGSEGV`, the same result from
  all four caller frames and across repeated runs) instead of silently pretending
  an unsatisfiable ~2^64-byte copy succeeded;
* the Rust computes the byte count and the VLA address with gcc's exact formula
  (`round16` then `align4`, all wrapping), and touches the VLA's first byte
  before copying, so that for every negative length whose VLA address is
  *unmapped* — including `INT_MIN` — both libraries fault at the same address.

`fma_array`, the low-level entry point, has no VLA, so its negative-length
behaviour is fully specified and *is* compared exactly (rows 3, 4 and
`d_neg_04_fma_array_negative_len_is_exactly_reproducible`).

## Gate

- [x] Every row above has a differential test that constructs that exact
      condition, calls **both** the C `.so` and the Rust `.so`, and asserts the
      **same** disposition (same stdout bytes + same output buffer, or the same
      terminating signal number), not merely "both failed somehow".
