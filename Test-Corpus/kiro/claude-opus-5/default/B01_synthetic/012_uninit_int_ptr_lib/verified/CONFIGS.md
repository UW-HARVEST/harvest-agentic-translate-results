# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from what the C code branches on, the same way `ERRORS.md`
is derived.

## Axes the C actually distinguishes

1. **Public entry points (4).** `nm -D` and `grep -nE '^void' c_src/src/driver.c`
   agree on the full set. `include/driver.h` only declares `driver`, the
   *convenience / one-shot wrapper*; the three lower-level entry points
   (`printIntPtrLine`, `good`, `bad`) are non-`static` and therefore part of the
   public ABI and are driven **directly**, not only through `driver`.
   Call hierarchy: `driver` → {`good`, `bad`} → `printIntPtrLine` → `printf`.
2. **Runtime options/flags: exactly one.** `driver(int useGood)`, tested by the
   single `if (useGood)` at `driver.c:50`. State it toggles: which of the two
   lower-level entry points runs. Two states: zero and non-zero. There are no
   other options, no global config, no init function, and no `#ifdef` (grep for
   `#if` finds only the `DRIVER_H_` include guard). `translation/Cargo.toml`
   declares no `[features]`, so there is exactly **one** feature combination —
   `verify.sh` enumerates `[features]` mechanically and confirms this rather than
   assuming it.
3. **Input shapes for `printIntPtrLine`'s `const int *`:**
   * *value* shape, because the sink is `printf("%d")`: `0`, `+1`, `-1`,
     `INT_MAX`, `INT_MIN`, random 32-bit patterns.
   * *pointer provenance* shape: stack, heap, static/`.data`, aligned vs
     misaligned. The C emits an alignment-tolerant `mov (%rax),%eax`.
4. **Call-sequence / stack-state shape.** A real axis, not an invented one:
   `bad` reads an *uninitialized stack slot*, so its output is a function of what
   previous calls left on the stack. The composed pipeline is therefore
   observable and per-wrapper tests cannot see it. Distinct shapes:
   nothing-before, `good` before, `printIntPtrLine` before, `driver(1)` before,
   `bad` twice, and reached-via-`driver` (one extra stack frame) versus
   called-directly.
5. **Output-stream shape.** `printf` is the only side effect, so stdout buffering
   mode is part of the observable surface: fully buffered (pipe) vs line
   buffered, and byte-exact ordering across many calls in one process.

## How the rows are executed

Every row runs the `harness` example twice as a subprocess — once per `.so` —
with an identical op script, and compares raw stdout bytes, exit code and
terminating signal. Both libraries are loaded with `libloading` and every
function is called through a `dlsym`-resolved symbol.

Rows that observe `bad()` poison the dead stack below the call site with an index
table first, so the uninitialized read becomes a deterministic observation of
*which slot* was read (see `examples/harness.rs`). All comparisons run with
`LD_BIND_NOW=1`; the reason is documented and verified by
`lazy_binding_first_call_is_not_reproducible_even_c_vs_c`.

## Table — one row per combination the C treats differently

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `printIntPtrLine` | stack-allocated aligned `int`, 512 seeded random values | `cfg_01_print_stack_random` | [x] |
| 2 | `printIntPtrLine` | boundary values `0, 1, -1, INT_MAX, INT_MIN, INT_MIN+1, INT_MAX-1, 10, -10, 100000` on a stack `int` | `cfg_02_print_stack_boundaries` | [x] |
| 3 | `printIntPtrLine` | heap (`malloc`) aligned `int`, 256 seeded random values | `cfg_03_print_heap_random` | [x] |
| 4 | `printIntPtrLine` | static / `.data` aligned `int`, 256 seeded random values | `cfg_04_print_static_random` | [x] |
| 5 | `printIntPtrLine` | misaligned pointer (`buf+1`, `+2`, `+3`) into a mapped byte array, 256 random 4-byte patterns per offset, plus aligned controls at `+0` and `+4` | `cfg_05_print_misaligned_random` | [x] |
| 6 | `printIntPtrLine` | called N times in one process (N = 1, 2, 17, 1000) with distinct random values — buffered stdout byte order over many calls | `cfg_06_print_repeated_counts` | [x] |
| 7 | `good` | no options; called once. Lower-level entry point, bypassing `driver` | `cfg_07_good_direct` | [x] |
| 8 | `good` | called repeatedly (1, 2, 64 times) — checks for hidden state | `cfg_08_good_repeated` | [x] |
| 9 | `driver` | `useGood` non-zero: `1` (the documented value) | `cfg_09_driver_one` | [x] |
| 10 | `driver` | `useGood` non-zero: 256 seeded random `int`s incl. negatives; all must take the `good` arm | `cfg_10_driver_random_nonzero` | [x] |
| 11 | `driver` | `useGood` non-zero boundaries: `1, -1, 2, INT_MAX, INT_MIN, INT_MIN+1, INT_MAX-1` | `cfg_11_driver_nonzero_boundaries` | [x] |
| 12 | `driver` | `useGood == 0` → routes into `bad`; nothing called before it in the process | `cfg_12_driver_zero_first_call` | [x] |
| 13 | `driver` | `useGood == 0` after `driver(1)` in the same process (stack-residue shape) | `cfg_13_driver_zero_after_one` | [x] |
| 14 | `driver` | alternating `driver(1)`/`driver(0)` 8 times in one process, poisoned and unpoisoned | `cfg_14_driver_alternating` | [x] |
| 15 | `bad` | called directly as the first library call (lowest-level entry point, no `driver` frame above it) | `cfg_15_bad_direct_first_call` | [x] |
| 16 | `bad` | called directly twice in a row | `cfg_16_bad_twice` | [x] |
| 17 | `bad` | called directly after `good()` — residue from `good` + `printIntPtrLine` + `printf` | `cfg_17_bad_after_good` | [x] |
| 18 | `bad` | called directly after `printIntPtrLine(&v)` | `cfg_18_bad_after_print` | [x] |
| 19 | `bad` | called directly after `driver(1)` | `cfg_19_bad_after_driver_one` | [x] |
| 20 | `bad` vs `driver(0)` | same process: `bad` reached directly and through `driver`, confirming the extra `driver` frame shifts the uninitialized read by the same 4 slots (32 bytes) in C and Rust | `cfg_20_bad_direct_vs_via_driver` | [x] |
| 21 | mixed pipeline | 32 seeded random operations from {`printIntPtrLine` stack/heap/static, `good`, `driver(rand≠0)`, `driver(0)`, `bad`}, 40 independent seeds, replayed identically against both `.so`s | `cfg_21_mixed_pipeline_random` | [x] |
| 22 | stdout shape | mixed pipelines with stdout line-buffered (`setvbuf`) and explicit `fflush` points, 12 seeds | `cfg_22_stdout_buffering_modes` | [x] |

All 22 rows pass under both the `release` and `dev` profiles and under the single
existing feature configuration.

## No binary executable

`c_src/CMakeLists.txt` contains no `add_executable`, and `translation/Cargo.toml`
declares only `crate-type = ["cdylib"]` with no `[[bin]]`. There is no driver
program, so the "compare C and Rust binary stdout" clause of Phase B has nothing
to apply to. The equivalent coverage is provided by rows 6, 14, 21 and 22, which
compare whole-process stdout byte streams for scripted multi-call sequences.

---

## Divergences found and fixed

Symbol parity and the happy path (`good`, `driver(1)`, `printIntPtrLine` with a
valid pointer) were already identical before this verification pass. All three
real divergences were on the `bad()` path, and none of them would have been
visible to per-function happy-path tests.

### 1. `driver(0)` — optimized tail call instead of a real frame

`driver` was translated as idiomatic Rust and LLVM compiled it to
`test %edi,%edi; je ...; jmp bad`. A tail jump means `bad` executes with **no
`driver` frame beneath it**, so its uninitialized read of `-0x8(%rbp)` lands 32
bytes higher than in the C, which builds a `-O0` frame and issues a real
`call bad`.

Observed, same harness, same input, `driver(0)`:

```
C:    prints stale stack data, exit 0
Rust: SIGSEGV
```

Fixed by emitting all four functions as naked functions reproducing GCC's `-O0`
instruction sequence, so the frame sizes, spill offsets and call/tail-call
structure match exactly. Caught by 10 tests; `cfg_20_bad_direct_vs_via_driver`
localises it precisely (direct call reads poison slot *n*, via-`driver` reads
*n+4*; the tail-jumping version read *n* in both cases).

### 2. Eager vs lazy PLT binding

The C `.so` is linked with the system compiler's default — partial RELRO with
**lazy** PLT binding. rustc defaults to full RELRO / `-z now`. Normally
invisible; not here, because the first `call bad@plt` under lazy binding detours
through `_dl_runtime_resolve`, which consumes a large amount of stack below the
caller's frame — precisely the bytes `bad()` then reads.

Observed, `driver(0)` three times in one process:

```
C:    1137197056   608471368   608471368
Rust:  608471368   608471368   608471368
```

The two agreed from the second call onward and disagreed only on the first — the
one that ran the resolver in the lazily-bound C library. Fixed in `build.rs` with
`-Wl,-z,lazy`. Guarded by `link_mode_matches_c` and
`lazy_binding_first_call_is_not_reproducible_even_c_vs_c`.

### 3. `std` linkage changing the loader's stack footprint

With both libraries lazily bound, a further systematic difference appeared. The C
`.so` records one `NEEDED` entry and a 43-entry `.dynsym`; the std-linked Rust
`cdylib` recorded three (`libgcc_s.so.1`, `libc.so.6`,
`ld-linux-x86-64.so.2`) and 1003. The resolver's stack consumption scales with
the symbol-lookup work, so the residue it left differed:

```
40 lazily-bound runs of driver(0):
  C:    0/40 faults
  Rust: 40/40 faults (SIGSEGV)
```

Fixed by making the crate `#![no_std]` (nothing here needs an allocator or
unwinding — the four functions are naked assembly plus one `printf` import) and
recording the same `libc.so.6` dependency the C library does. The Rust `.so` went
from 402 696 bytes / 1003 dynsym / 3 `NEEDED` to 6 416 / 44 / 1, and the fault
rates now match at 0/40. Guarded by `elf_02_dependency_parity`,
`elf_03_dynsym_size_comparable` and `lazy_binding_fault_behaviour_matches`.

Note that whether a *particular* std-linked build faults is sensitive to the whole
link configuration, not to `std` alone; this is why two of the three guards check
the ELF-level cause directly rather than relying only on the observable symptom.

### What remains genuinely uncomparable

Under lazy binding, the value the **first** `driver(0)` prints is derived from
ASLR-randomised addresses the resolver leaves on the stack. It is not reproducible
even when the C library is compared against *itself* — 40 runs produced 40
distinct values. No translation can be required to match it, so the differential
comparisons run with `LD_BIND_NOW=1`, where the value is fully deterministic and
does match. `lazy_binding_first_call_is_not_reproducible_even_c_vs_c` asserts the
non-reproducibility rather than assuming it, so that if the loader ever becomes
deterministic here the test fails and tells us to compare the case directly.

## Reproducing

```
cd translation && ./verify.sh
```

`verify.sh` builds the C library, enumerates `[features]` from `Cargo.toml`,
runs the suite for every combination under both profiles, then checks symbol
parity, unresolved symbols, per-function instruction streams and the ELF profile.
