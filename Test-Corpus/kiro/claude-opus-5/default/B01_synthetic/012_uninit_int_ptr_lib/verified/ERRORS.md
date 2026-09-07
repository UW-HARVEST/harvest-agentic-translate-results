# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from documentation.

## Mechanical grep results

```
grep -nE 'return|assert|NULL|ERROR|errno|exit|abort|if *\(|switch|#if' \
     c_src/src/driver.c c_src/include/driver.h
```

After removing the license header and the include guard, the entire C source
contains exactly **two** non-comment matches:

* `c_src/src/driver.c:26: #include <stdio.h>`
* `c_src/src/driver.c:50: if (useGood)`

Consequences, stated explicitly because they define the shape of this table:

* All four public functions return `void`. There is **no** error code, **no**
  sentinel return, **no** `return -1`, **no** `return NULL`, and **no** error
  enum anywhere in the library.
* There is **no** `assert`, **no** null check, **no** range check, and **no**
  min/max constant.
* `useGood` is a plain `int`, not an enum. All 2^32 values are accepted;
  `if (useGood)` splits them into zero / non-zero. There is therefore no such
  thing as an out-of-range enum value for this API — but the out-of-range
  integers are still tested (rows 9–12), because that is the bug class the
  instructions call out, and row 12 turned out to be a real trap.

So the library's only rejection mechanism is hardware/UB: an invalid pointer
reaching the single dereference at `driver.c:30`
(`printf("%d\n", *intNumber)`). Each row below is one distinct way that
dereference can be reached with an invalid or undefined operand, plus the generic
FFI boundary cases.

## How the rows are executed

Each row runs the `harness` example twice as a subprocess — once per `.so`, loaded
via `libloading`, every function called through `dlsym` — and compares raw stdout
bytes, exit code **and terminating signal number** individually. "Both failed
somehow" is not accepted: `assert_same_fault` requires the specific signal, and
`assert_match` additionally refuses to pass vacuously if a row produced neither
output nor a signal.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `printIntPtrLine` | `intNumber == NULL` — no null check exists, so `*intNumber` reads address 0. Also covered with prior buffered output pending, and with the null arriving as a raw integer across FFI (`(int*)0`) | fatal `SIGSEGV` (11); stdout up to that point flushed identically | `err_01_print_null_ptr` | [x] |
| 2 | `printIntPtrLine` | non-null unmapped low address: `0x1, 0x2, 0x3, 0x4, 0x8, 0xff, 0xfff` (several also misaligned) | `SIGSEGV`, no output | `err_02_print_unmapped_low` | [x] |
| 3 | `printIntPtrLine` | large unmapped canonical address: `0x7fffffff0000, 0x7ffffffff000, 0x600000000000` | `SIGSEGV`, no output | `err_03_print_unmapped_high` | [x] |
| 4 | `printIntPtrLine` | non-canonical / kernel-space address: `0xfffffffffffffff0, 0xffffffffffffffff, 0x8000000000000000` | `SIGSEGV`, no output | `err_04_print_noncanonical` | [x] |
| 5 | `printIntPtrLine` | misaligned but mapped (`buf+1`, `+2`, `+3`). x86-64 permits the unaligned `mov (%rax),%eax`, so this is *accepted*, not rejected. In the table because it is the boundary a naive translation using an alignment-requiring Rust read would wrongly reject | no fault, exit 0; prints the unaligned little-endian `int` (`-559038737`, `-2147483648`, `-1` for the patterns used) | `err_05_print_misaligned` | [x] |
| 6 | `printIntPtrLine` | one-past-the-end of a heap array — dangling but in practice still mapped | no fault; prints the following bytes; identical in both | `err_06_print_one_past_end` | [x] |
| 7 | `bad` | no input at all: `int *data;` never initialized, then dereferenced (`driver.c:35-36`). The read is of `-0x8(%rbp)`, i.e. `entry_rsp - 16` | UB. With a controlled (poisoned) stack: prints the index of the slot at `entry_rsp-16` and exits 0 — Rust must read the *same* slot. With the natural stack: prints stale data, exit 0 | `err_07_bad_uninit_read` | [x] |
| 8 | `driver` | `useGood == 0` — the false arm, routing into row 7's UB | garbage printed via `bad`, exit 0; slot index must match row 7's + 4 | `err_08_driver_zero` | [x] |
| 9 | `driver` | `useGood == INT_MIN` (`-2147483648`) — one step past the low end of `int`; non-zero, so the `good` arm | prints `5`, exit 0 | `err_09_driver_int_min` | [x] |
| 10 | `driver` | `useGood == INT_MAX` (`2147483647`) — one step past the high end | prints `5`, exit 0 | `err_10_driver_int_max` | [x] |
| 11 | `driver` | "out-of-range enum" values with no valid variant if `useGood` were a 2-variant enum: `-1, 2, 3, 4, 255, 256, 65535, 65536, INT_MIN, INT_MAX` plus 128 seeded random non-zero values | all non-zero → prints `5`, exit 0. Never rejected (138 lines of `5`) | `err_11_driver_out_of_range_enumlike` | [x] |
| 12 | `driver` | 64-bit garbage in the argument register with only the low 32 bits zero: `0x1_00000000`, `0xffffffff_00000000`, `0xdead0000_00000000`. The C spills and tests `edi` (`mov %edi,-0x4(%rbp)` / `cmpl $0x0,-0x4(%rbp)`), so the high half is discarded and the value counts as **zero** | takes the `bad` arm, **not** `good` — asserted by requiring the output is not `5`. Controls with a non-zero low half take the `good` arm | `err_12_driver_high_half_only` | [x] |
| 13 | `printIntPtrLine` | `INT_MIN` at the `%d` conversion — the value whose negation overflows — from stack, heap and static storage | prints `-2147483648` three times | `err_13_print_int_min` | [x] |
| 14 | all four | repeated invocation with no initialization: no init function, no re-entrancy guard, no "already used" state. Each entry point exercised first-in-process and then repeated | never rejected; identical output each time | `err_14_no_init_required` | [x] |

All 14 rows pass under both the `release` and `dev` profiles and under the single
existing feature configuration.

## Notes on specific rows

**Row 5** is the mirror image of an error row: it documents an input the C
*accepts* that a plausible Rust translation would reject. The naked-assembly
`printIntPtrLine` emits the same alignment-agnostic 32-bit load as the C; the
portable non-x86-64 fallback uses `read_unaligned` for the same reason.

**Row 7** is the library's entire point, and it is where all three real
divergences were found. Because the read is undefined, the harness first poisons
the dead stack below the call site with a table of pointers to `table[j] == j`, so
the printed value identifies exactly which 8-byte slot was read. That converts
"undefined" into a precise, reproducible differential observation. Details and
measurements are in the findings section of `CONFIGS.md`.

**Row 12** is the out-of-range-value trap that happy-path testing misses. A Rust
`driver` that took `u64`/`usize` or tested the whole register would send
`0x1_00000000` to `good()` and print `5`, whereas the C sends it to `bad()`. The
naked implementation performs the same 32-bit spill and compare.
