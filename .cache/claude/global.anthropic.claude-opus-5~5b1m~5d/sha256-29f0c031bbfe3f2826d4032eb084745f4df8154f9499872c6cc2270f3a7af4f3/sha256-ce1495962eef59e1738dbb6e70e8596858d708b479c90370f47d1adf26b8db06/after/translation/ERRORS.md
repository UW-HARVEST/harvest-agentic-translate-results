# ERRORS.md — Phase A error-surface table

Mechanically derived by grepping `c_src/src/lib.c` + `c_src/include/lib.h` for
**every** rejection construct:

```
grep -n 'return|assert|NULL|if|switch|default' c_src/src/lib.c
```

Findings:

* error-return macros (`RETURN_ERROR`, …): **none** — the library has no error
  macro and no error enum.
* `assert` / `static_assert`: **none**.
* `NULL` checks: **none** (the string `NULL` does not occur in the sources).
* explicit range / bounds / min-max constants: **none**.
* the only *rejection* branches in the whole library are the **three
  `default: return 0;`** arms of the `switch` nest in `collided`
  (lines 81-82, 91-92, 95-96). Every other `return` is a normal result.

Consequently, the entire error surface is "unrecognised shape tag ⇒ `0`", plus
the generic FFI boundary conditions (null / wrong-type / misaligned pointers)
which the C code does **not** check at all. Those are listed too, because an
external caller can trigger them and the Rust must not diverge (e.g. must not
panic where C quietly reads memory).

`0` is *also* the "no collision" answer, so each test below additionally pins
down that the C really took the `default` arm (it is verified by using operand
data that would return `1` under every valid tag pair — see
`tests/phase_c_errors.rs::sentinel_zero_is_the_default_arm_not_a_miss`).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `collided` | `typeA` matches no enumerator (`typeA >= 2`), any `typeB` — outer `default` (lib.c:95) | returns `0`; **neither** `A` nor `B` is dereferenced | `row01_typeA_out_of_range` |
| 2 | `collided` | `typeA == C2_TYPE_CIRCLE` **and** `typeB` matches no enumerator — first inner `default` (lib.c:81) | returns `0`; neither pointer dereferenced | `row02_circle_typeB_out_of_range` |
| 3 | `collided` | `typeA == C2_TYPE_AABB` **and** `typeB` matches no enumerator — second inner `default` (lib.c:91) | returns `0`; neither pointer dereferenced | `row03_aabb_typeB_out_of_range` |
| 4 | `collided` | out-of-range enum value passed across FFI: `2`, `3`, `0x7fff_ffff` (`INT_MAX`), `0x8000_0000` (`INT_MIN` bit pattern), `0xffff_ffff` (`-1`), in **either** tag position and in every combination with a valid tag | returns `0` (tags are compared with `cmpl $0`/`cmpl $1`, so only the 32-bit pattern matters; sign interpretation is unobservable) | `row04_enum_value_matrix` |
| 5 | `collided` | `typeA`/`typeB` exactly one step past the last valid enumerator (`C2_TYPE_AABB + 1 == 2`) — the classic off-by-one | returns `0` | `row05_one_past_last_enumerator` |
| 6 | `collided` | `A == NULL` and/or `B == NULL` **together with an invalid tag** (rows 1-3) | returns `0` — safe, because the `default` arm returns before any dereference | `row06_null_pointers_with_invalid_tag` |
| 7 | `collided` | `A == NULL` and/or `B == NULL` with **valid** tags — C has no null check and dereferences unconditionally (lib.c:78, 80, 88, 90) | undefined behaviour; in practice the process dies from a memory-access fault. Rust must behave the same way (fault, *not* a graceful `0` and not a Rust panic/abort with a different signal) | `row07_null_pointer_with_valid_tag_faults_identically` (runs each library in a forked child and compares the termination signal) |
| 8 | `collided` | pointer to an object *smaller* than the tag claims: a `c2v` (8 B) or `c2Circle` (12 B) passed with `C2_TYPE_AABB` (reads 16 B) | no check: C reads the adjacent bytes and computes on that garbage. Rust must read the **same** bytes and return the same answer | `row08_undersized_object_reads_same_bytes` |
| 9 | `collided` | pointer whose tag disagrees with the real object type (`c2AABB` object tagged `C2_TYPE_CIRCLE` and vice versa) | no check: the bytes are reinterpreted; both libraries must agree | `row09_type_confusion_agrees` |
| 10 | `collided` | misaligned pointer (`c2Circle`/`c2AABB` at an odd byte offset) | no check: x86 `movss`/`mov` handle it; C returns the value computed from the unaligned bytes. Rust must not trip an alignment assumption (it loads each field with a single unaligned `mov`) | `row10_misaligned_pointer` |
| 11 | *(all shape predicates)* | "out of range" **float** inputs — the C validates nothing: `NaN` (quiet, signalling, both signs, distinct payloads), `±inf`, `±0.0`, subnormals, `FLT_MAX` (products overflow to `inf`), negative radii, inverted boxes (`min > max`) | no rejection; the comparison `d2 < r2` is simply `false` for unordered operands (`comiss` + `seta`/`jbe`), and `!(d0|d1|d2|d3)` still evaluates. Both libraries must return the same `int` bit-for-bit | `row11_no_float_validation` (+ the whole of `phase_b_valid.rs`) |
| 12 | `c2V`, `c2Maxv`, `c2Minv`, `c2Clampv`, `c2Sub`, `c2Dot`, `c2CircletoCircle`, `c2CircletoAABB`, `c2AABBtoAABB` | *no* error path exists in any of these — no arg validation, no sentinel return, no `errno`. The only "invalid" inputs are the float classes of row 11 | every input produces a value (`0`/`1` for the predicates, a vector/float otherwise); nothing is rejected | `row12_shape_functions_have_no_error_path` |

## Boundary values covered even though the C has no explicit check

| condition | why it is covered | test |
|-----------|-------------------|------|
| zero-length / zero-size operand | not expressible: every entry point takes fixed-size by-value structs or a tagged pointer. The "zero" analogue is a zero-radius circle / zero-area (`min == max`) box | `phase_b_valid.rs` rows 14, 21 |
| oversized length | not expressible (no length parameter anywhere in the API) | — |
| one step past a valid range | enum tag `2` (row 5); radius one ULP either side of the exact touching distance (`d2 == r2` ⇒ `<` is false) | rows 5, `phase_b_valid.rs` row 12 |
| null pointer | rows 6, 7 | — |
| out-of-range enum across FFI | rows 4, 5 | — |

## Verification result (Phase C)

All 12 rows have a passing differential test in `tests/phase_c_errors.rs`, in
**both** profiles and under every feature combination
(`./run_all.sh`, `./feature_matrix.sh`):

```
running 14 tests
test row01_typeA_out_of_range ... ok            test row08_undersized_object_reads_same_bytes ... ok
test row02_circle_typeB_out_of_range ... ok     test row09_type_confusion_agrees ... ok
test row03_aabb_typeB_out_of_range ... ok       test row10_misaligned_pointer ... ok
test row04_enum_value_matrix ... ok             test row11_no_float_validation ... ok
test row05_one_past_last_enumerator ... ok      test row12_shape_functions_have_no_error_path ... ok
test row06_null_pointers_with_invalid_tag ... ok  test sentinel_zero_is_the_default_arm_not_a_miss ... ok
test row07_null_pointer_with_valid_tag_faults_identically ... ok
test result: ok. 14 passed; 0 failed
```

### Divergence found and FIXED — row 7 (null pointer with a valid tag)

This is the one real bug the error-path phase uncovered, and it was invisible to
every happy-path test:

| | before the fix | C (ground truth) |
|---|---|---|
| debug / UB-checked Rust `.so` | `SIGABRT` (6) — non-unwinding Rust panic: *"unsafe precondition(s) violated: ptr::read_unaligned…"* | `SIGSEGV` (11) |
| release Rust `.so` | `SIGSEGV` (11) | `SIGSEGV` (11) |

`collided` dereferences its `const void *` arguments unconditionally, so a null
pointer must fault. Every ordinary spelling of the load defeats that:
`ptr::read`, `read_unaligned`, `read_volatile` and `copy_nonoverlapping` (hence
also a `repr(packed)` field read) each carry an `assert_unsafe_precondition!`
null check, and even a plain `*(p as *const T)` picks up rustc's inserted
*"null pointer dereference occurred"* check. All of them convert the C's fault
into a Rust abort whenever the cdylib is built with `debug_assertions`.

Fixed in `src/lib.rs` by loading each `float` field with a single `mov` through
`core::arch::asm!` (`load32`/`read_circle`/`read_aabb`), which is opaque to
those checks and is exactly what gcc emits for the C. As a bonus this also
removes any alignment assumption, matching the C's unchecked misaligned reads
(rows 8-10).

Note that the divergence only manifested in the debug/UB-checked build, so
`./run_all.sh` and `./mutation_check.sh` deliberately exercise **both** profiles;
the `checked_null_read` mutant in `./mutation_check.sh` re-introduces the bug and
is caught by `row07` in the debug profile only.
