# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`, not
from documentation. The grep used to enumerate every rejection construct:

```sh
grep -nE 'return|assert|NULL|errno|if *\(|else|switch|case|#if|goto|ERROR|exit|abort|<|>|==|!=|enum|#define' \
    c_src/src/driver.c c_src/include/driver.h
```

## What the grep actually found

* `c_src/src/driver.c:50` — `if (useGood)`
* `c_src/src/driver.c:54` — `else`
* `c_src/include/driver.h:24-25` — the include guard `#ifndef/#define DRIVER_H_`
* `c_src/src/driver.c:26` — `#include <stdio.h>`

and **nothing else**. Specifically the library contains:

* **zero** `return` statements of any kind (all four functions are `void`, and
  none has an early `return`);
* **zero** error-return macros / sentinels (`RETURN_ERROR`, `return -1`,
  `return NULL`, …);
* **zero** `assert` / `abort` / `exit` calls;
* **zero** null-pointer checks, range checks, size checks or capacity checks;
* **zero** `enum`s, so there is no enum-domain validation to test;
* **zero** min/max constants;
* **zero** `#ifdef` configuration branches (the only preprocessor conditional is
  the header's include guard);
* exactly **one** data-dependent branch in the whole library — `if (useGood)`
  in `driver` — and both of its arms are *valid*, not error, paths.

So the C library has **no explicit rejection surface at all**. It never reports
an error to its caller; there is no error code, no sentinel and no `errno` use.
The only way an invalid input is "rejected" is that the process faults, which is
what the table below enumerates. `printIntPtrLine` dereferences its argument
unconditionally at `driver.c:30` (`printf("%d\n", *intNumber)`), so the pointer
domain is where every failure mode lives.

## Error-surface table

Each row is a distinct way the C can reject/fault on input. "Expected C result"
is what the C library actually does, verified by running it — not what a
defensive API would do.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `printIntPtrLine` | `intNumber == NULL` | no validation; `mov eax,[rax]` on address 0 faults → `SIGSEGV` (signal 11), nothing written to stdout, stdio buffer lost unflushed |
| 2 | `printIntPtrLine` | `intNumber` = non-null but unmapped address (e.g. `0x1`, `(int*)-1`, a freshly `munmap`ed page) | no validation; read faults → `SIGSEGV`, no output |
| 3 | `printIntPtrLine` | `intNumber` = misaligned but readable address (e.g. `buf+1`, `buf+3`) | **not** an error on x86_64: unaligned 32-bit load succeeds; prints the little-endian `int` assembled from those 4 bytes |
| 4 | `printIntPtrLine` | `intNumber` points at the last valid bytes of a mapping such that the 4-byte read straddles the end of the mapping | partial read faults → `SIGSEGV`, no output |
| 5 | `printIntPtrLine` | `intNumber` points at a `PROT_NONE` / write-only mapping | read faults → `SIGSEGV`, no output |
| 6 | `bad` | *always* — `int *data;` at `driver.c:35` is never initialised and is dereferenced via `printIntPtrLine` (CWE-457 / CWE-824). No input can avoid it. | **unspecified**: reads the stale 8 bytes at `[rbp-8]`. If that stale value is a readable address, prints the garbage `int` it points at and exits 0; if not, `SIGSEGV`. Outcome depends on prior stack residue, so it is not a function of any input and is not reproducible run-to-run. |
| 7 | `driver` | `useGood == 0` — the `else` arm at `driver.c:54` reaches the defect in row 6 | same unspecified outcome as row 6, but `bad`'s frame sits 32 bytes lower (through `driver`'s frame + the `call` return address), so it reads a *different* stale slot — in practice one clobbered by `_dl_runtime_resolve`, yielding a leaked stack address that changes on every run under ASLR |
| 8 | `driver` | `useGood` = out-of-range / unexpected integer (`INT_MIN`, `INT_MAX`, `-1`, `0x80000000`, high bits set) | **not** an error: `driver` takes `int`, not an enum, and tests C truthiness with `cmpl $0,-0x4(%rbp)`. Every non-zero value takes the `good` arm and prints `5\n`. Only exact zero takes the `bad` arm. No value is rejected. |
| 9 | `driver` | `useGood` passed as a 64-bit value with garbage in the upper 32 bits (a real FFI condition: caller sets `rdi`, callee reads `edi`) | upper 32 bits ignored — the spill is `mov %edi,-0x4(%rbp)`, so only the low 32 bits are tested. `0x1_0000_0000` (low half zero) therefore takes the `bad` arm. |
| 10 | `good` | *none reachable* — `data` is initialised to 5 and only its own address is taken | cannot fail from input; always prints `5\n`, exit 0 |
| 11 | any | stdout closed / redirected to a full or unwritable fd | `printf` returns negative; the return value is **discarded** at `driver.c:30`, so the library reports nothing and returns normally |
| 12 | *(N/A — no enum in the C)* | out-of-range enum value across the FFI boundary | the required generic check has no enum to apply to: `grep -c enum` over both C files is 0. The nearest analogue is row 8/9 — `int` arguments with no valid-range restriction — which are covered. |

## Notes on rows 6 and 7

Rows 6 and 7 are the *point* of this test case, and they are the only rows whose
expected result is not a fixed value. The C library's own output for these rows
differs on every execution:

```text
$ for i in 1 2 3 4 5 6; do ./probe1 libdriver.so driver0; done   # the C library
498909184
-396595200
851451904
-1475596288
1391702016
-2119532544
```

A byte-for-byte comparison against the C is therefore impossible *for these two
rows only* — the C is not byte-identical to itself. `tests/errors.rs` asserts on
the properties that *are* specified and that a mistranslation would break:
both implementations survive or fault identically (same exit status / signal),
and when they survive both emit exactly one line matching `^-?[0-9]+\n$`.
Everything the C fixes, the tests compare exactly.

Measured over 40 isolated runs per library, the termination agreement is
deterministic — `driver(0)` survives 40/40 in both, and a bare `bad()` from the
test harness's frame faults 40/40 in both — so only the printed value is
unspecified, not whether the process lives.

Rows 6 and 7 also have *residue-controlled* variants where the same defect
becomes fully specified, and there the tests do assert byte equality:

| configuration | slot `bad()` reads | C and Rust both print |
|---------------|--------------------|-----------------------|
| `good(); bad();` at one stack depth | the pointer `good` stored at `[rbp-8]`, which still points at its `5` | `5\n5\n` |
| `printIntPtrLine(&v); bad();` | the argument `printIntPtrLine` spilled to `[rbp-8]`, i.e. `&v` | `<v>\n<v>\n`, for every `v` tested |
| `bad(); bad();` | the slot the first `bad()` left | the same value twice |

These are the strongest checks in the suite: they make the uninitialised read
observable as a *function of the input*, so any difference in frame size, slot
offset or call-vs-tail-jump shows up as a wrong number rather than as noise.
They are what caught the original translation's tail-called `driver`.

## Test status

Every row above has a passing differential test in `tests/errors.rs`
(`err01`…`err12`), plus two generic-boundary tests. Verified:

```text
running 14 tests
test err01_pipl_null ... ok
test err02_pipl_unmapped_addresses ... ok
test err03_pipl_misaligned_is_not_an_error ... ok
test err04_pipl_read_straddles_mapping_end ... ok
test err05_pipl_unreadable_mapping ... ok
test err06_bad_uninitialised_read ... ok
test err07_driver_zero_reaches_the_defect ... ok
test err08_driver_out_of_range_values_accepted ... ok
test err09_driver_wide_rdi_upper_bits_ignored ... ok
test err10_good_cannot_fail ... ok
test err11_unwritable_stdout_is_not_reported ... ok
test err12_no_enum_domain_but_check_the_int_domain_anyway ... ok
test generic_null_and_extreme_pointers ... ok
test generic_zero_and_oversized_lengths_do_not_exist ... ok

test result: ok. 14 passed; 0 failed
```

Confirmed under both the `debug` and `release` profile, and under both
`--no-default-features` and the default feature set (the crate declares no
features, so those are the only configurations that exist).
