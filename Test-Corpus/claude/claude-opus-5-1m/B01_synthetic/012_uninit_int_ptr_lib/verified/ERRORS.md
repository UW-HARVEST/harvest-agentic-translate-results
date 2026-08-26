# ERRORS.md — Error-surface table (Phase A / Phase C)

Derived mechanically from the C source. The grep for every rejection construct
returns **exactly one hit in the entire library**:

```sh
$ grep -nE "return|assert|NULL|if *\(|error|ERROR|-1|INT_M" \
      c_src/src/driver.c c_src/include/driver.h
c_src/src/driver.c:50:    if (useGood)
```

So, mechanically:

* every function returns `void` — there is **no** error code, sentinel, or
  `RETURN_ERROR`-style macro anywhere;
* there are **no** `assert`s, **no** null checks, **no** range checks, **no**
  min/max constants, and **no** error enums;
* the only conditional in the library is `if (useGood)`, which is a *dispatch*,
  not a rejection — both arms are "success" paths.

The library's rejection surface is therefore entirely **implicit**: invalid input
is not diagnosed, it is dereferenced. The rows below enumerate every distinct way
the C can be made to fault or to consume an out-of-contract input, which is the
error surface a caller can actually observe.

Legend for "expected C result": `SIGSEGV` = process killed by signal 11;
`prints "<n>\n"` = writes that exact decimal text to `stdout`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `printIntPtrLine` | `intNumber == NULL` — no null check exists, line 30 dereferences unconditionally | `SIGSEGV` (11) | [x] |
| 2 | `printIntPtrLine` | `intNumber` = non-null but unmapped address (e.g. `0x1`, page-unmapped high address) | `SIGSEGV` (11) | [x] |
| 3 | `printIntPtrLine` | `intNumber` = misaligned address (not a multiple of 4) | **no error** — GCC emits a plain `mov (%rax),%eax`, which loads unaligned on x86-64; prints the value | [x] |
| 4 | `printIntPtrLine` | `intNumber` = address of a *write-only* / `PROT_NONE` mapping | `SIGSEGV` (11) | [x] |
| 5 | `printIntPtrLine` | value at `*intNumber` = `INT_MIN` (`-2147483648`), the boundary `%d` must render with no overflow | prints `-2147483648\n` | [x] |
| 6 | `printIntPtrLine` | value at `*intNumber` = `INT_MAX` (`2147483647`) | prints `2147483647\n` | [x] |
| 7 | `printIntPtrLine` | pointer to the *last* 4 bytes of a mapping (one step past → next page unmapped) | prints the value, no fault | [x] |
| 8 | `printIntPtrLine` | pointer 1 int past the end of a 1-element allocation (out-of-range index) | reads the adjacent in-bounds bytes; no fault, prints whatever is there | [x] (asserted: the test plants a known sentinel there, so both must print it) |
| 9 | `bad` | *always* — `int *data;` (line 35) is never assigned, and line 36 dereferences it (CWE-457 / CWE-824) | **indeterminate**: loads the 8 leftover bytes at `rbp-0x8` and dereferences them. Prints garbage, or `SIGSEGV`, depending on the previous occupant of that stack region | [x] (see note A) |
| 10 | `bad` | called immediately after `good()` at the same stack depth | **deterministic**: prints `5\n` — `good()` stored `&data` into the very slot `bad()` reads (note A) | [x] |
| 11 | `driver` | `useGood == 0` → takes the `else` arm → `bad()` | same as row 9/10 (indeterminate in isolation; `5\n` when preceded by `driver(non-zero)`) | [x] |
| 12 | `driver` | `useGood` = out-of-range "enum" value with no valid variant, e.g. `2`, `-1`, `7`, `0x7fffffff`, `INT_MIN` | **no error / not rejected** — `if (useGood)` is a plain truthiness test, so *every* non-zero `int` takes the `good()` arm and prints `5\n` | [x] |
| 13 | `driver` | `useGood` = `INT_MIN` (`-2147483648`), the extreme negative | non-zero → `good()` → prints `5\n` | [x] |
| 14 | `driver` | `useGood` = a value whose *low 32 bits* are zero but which was passed in a 64-bit register with garbage high bits (`0x1_00000000`) | only `%edi` is compared (`cmpl $0x0,-0x4(%rbp)`), so this is **zero** → `bad()` arm | [x] |

## Note A — rows 9–11, the CWE-457 defect

`bad()` is the intentional defect and must **not** be fixed. Its value is
indeterminate by construction, so it cannot be byte-matched in general — the C
itself is not stable, printing `0`, `3`, `-2040302194`, `1420842379`, … across
runs depending only on ASLR and on what previously used that stack region.

There is, however, one **deterministic** observable, and the Rust reproduces it
exactly. At `-O0` GCC gives `bad()` and `good()` byte-identical 16-byte frames
and puts the `int *` local at the same offset, `rbp-0x8`, in both:

```
bad:                          good:
  sub  $0x10,%rsp               sub  $0x10,%rsp
  mov  -0x8(%rbp),%rax          movl $0x5,-0xc(%rbp)   ; data      = 5
  mov  %rax,%rdi                lea  -0xc(%rbp),%rax
  call printIntPtrLine          mov  %rax,-0x8(%rbp)   ; data_addr = &data
                                mov  -0x8(%rbp),%rax
                                mov  %rax,%rdi
                                call printIntPtrLine
```

So `good()` followed by `bad()` **at the same call depth** makes `bad()` load
back the pointer `good()` just stored, and print `5`. `src/lib.rs` reproduces this
by construction: `bad()` and `good()` both delegate to a single
`#[inline(never)] frame_body(init: bool)` helper, so the frame — and hence the
pointer slot — is literally the same function's frame in both cases. Verified
stable over 25 runs per case, in both debug and release, for `good();bad()`,
`good();bad();bad()`, `driver(1);driver(0)` and `driver(1);driver(0);driver(0)`.

Where the two calls are at **different** stack depths (e.g. `driver(1)` then
`bad()` directly, mixing one- and two-frame call paths), the slot is no longer
the one `good()` wrote and the C's output is genuinely arbitrary — C was observed
both printing `5` and taking `SIGSEGV` for the same sequence. Those combinations
are exercised but **not** asserted byte-equal; see
`rows26_28_indeterminate_ub_paths_are_recorded_not_asserted` in
`tests/phase_b_valid_paths.rs` and
`row09_and_row11_uninitialised_read_in_isolation_is_indeterminate` in
`tests/phase_c_error_paths.rs`, which record the outcomes instead.

## Generic FFI-boundary checks (also covered)

| # | check | result |
|---|-------|--------|
| G1 | null pointer into `printIntPtrLine` | rows 1 — both `SIGSEGV` |
| G2 | out-of-range enum value across FFI into `driver` | rows 12–14 — every non-zero `int` is accepted, no rejection |
| G3 | one step past a valid range | rows 7, 8 |
| G4 | boundary values `0 / ±1 / INT_MIN / INT_MAX` | rows 5, 6, 13; plus randomized sweep in `CONFIGS.md` |
| G5 | 64-bit garbage in the high half of a 32-bit `int` arg | row 14 |

There are no length/size parameters anywhere in the API, so "zero and oversized
lengths" has no analogue in this library.

## Divergences found by these tests, and the fixes

All three were genuine translation defects in `src/lib.rs`, invisible to
happy-path testing, and all three were fixed by changing the Rust (never the C).

1. **Row 3 — misaligned load aborted instead of succeeding.** The original
   translation used `*intNumber`. In a debug build that carries a
   "misaligned pointer dereference" check, so any pointer not 4-byte aligned
   aborted with `SIGABRT` (6) while the C printed a value. Found by
   `row11_print_misaligned_pointer`.

2. **Row 1 — null load raised the wrong signal.** The first fix for (1) used
   `core::ptr::read_unaligned`, which routes through `copy_nonoverlapping` and so
   carries a debug-only *non-null* precondition: `printIntPtrLine(NULL)` aborted
   with `SIGABRT` (6) where the C faults with `SIGSEGV` (11). Found by
   `row01_null_pointer_into_print_int_ptr_line`.

   Both are now handled by `load_int_unchecked`, a bare `mov` (inline `asm!` on
   `x86_64`) that carries neither precondition — exactly the instruction GCC
   emits for `*intNumber`. Unaligned addresses load; invalid addresses `SIGSEGV`.

3. **Rows 10, 22–25 — the `good()`/`bad()` shared stack slot was not shared.**
   `bad()` originally read an ad-hoc `MaybeUninit` slot that rustc placed at a
   different frame offset from `good()`'s (`entry_rsp-0x38` vs `entry_rsp-0x58`),
   so `good();bad()` printed garbage where the C deterministically prints `5`.
   Fixed by routing both through one `#[inline(never)] frame_body(init: bool)`,
   which makes the slot the same address by construction. Found by
   `row22_good_then_bad_same_depth`.

## Test-harness note — `RTLD_NOW`

`tests/common/mod.rs` opens both `.so`s with `RTLD_NOW`. With `libloading`'s
default `RTLD_LAZY`, the first call to `bad@plt`/`good@plt` from inside `driver`
runs the dynamic linker's lazy-resolution path, whose own stack usage overwrites
the slot `bad()` reads — the C's `driver(1);driver(0)` then prints garbage instead
of `5\n5\n`. That is a property of the loader, not of either implementation, so
eager binding is used to keep it out of the comparison.
