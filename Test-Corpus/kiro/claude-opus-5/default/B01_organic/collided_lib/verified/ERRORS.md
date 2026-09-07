# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

Grep audit of every rejection mechanism the C source could use:

```sh
grep -nE 'RETURN_ERROR|return -1|return NULL|assert|errno|ERROR|MIN|MAX|default:|return 0' \
    src/lib.c include/lib.h
```

Result: the library contains **no** `assert`, **no** `errno`, **no** error enum,
**no** `return -1` / `return NULL`, **no** explicit range/null check, and **no**
`MIN`/`MAX` constants. The *only* rejection construct in the whole library is the
`default:` label of the three `switch` statements in `collided`, each of which
does `return 0`. Everything else is unchecked arithmetic whose "rejection" is the
floating-point `comiss`/`seta` comparison evaluating false (the NaN case).

Rows 1–3 are the three literal `default: return 0` branches (lib.c:81-82,
lib.c:91-92, lib.c:95-96). Rows 4–13 are the generic FFI boundaries required by
Phase C (out-of-range enum values, null pointers, sentinel/NaN rejection,
one-step-past-valid-range enum values).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `collided` (lib.c:95-96, outer `default:`) | `typeA` is not `C2_TYPE_CIRCLE(0)` nor `C2_TYPE_AABB(1)` — e.g. `2`, `3`, `-1`, `INT_MIN`, `INT_MAX`, `0x7fffffff`. `typeB` and both pointers irrelevant (pointers are **not** dereferenced). | returns `0` |
| 2 | `collided` (lib.c:81-82, inner `default:` under `C2_TYPE_CIRCLE`) | `typeA == 0` and `typeB` out of range (`2`, `-1`, `INT_MAX`, …). `A` is not dereferenced. | returns `0` |
| 3 | `collided` (lib.c:91-92, inner `default:` under `C2_TYPE_AABB`) | `typeA == 1` and `typeB` out of range (`2`, `-1`, `INT_MAX`, …). `A` is not dereferenced. | returns `0` |
| 4 | `collided` | `typeA` one step past the valid range: `typeA == 2` (`C2_TYPE_AABB + 1`) with every `typeB` in `{0,1,2}` | returns `0` (row 1 path) |
| 5 | `collided` | `typeB` one step past the valid range: `typeB == 2` with `typeA` in `{0,1}` | returns `0` (rows 2/3 path) |
| 6 | `collided` | Null pointers with an out-of-range `typeA` (`A == NULL`, `B == NULL`, `typeA == 2`) — the switch rejects before any dereference, so this is defined behaviour | returns `0` |
| 7 | `collided` | Null `B` with `typeA == 0`, `typeB == 2` — inner `default:` reached before `B` is dereferenced | returns `0` |
| 8 | `collided` | Null `B` with `typeA == 1`, `typeB == 2` | returns `0` |
| 9 | `collided` | Non-null but *unaligned* / oversized-read shapes: `A` points at a `c2AABB` (16 B) while `typeA == C2_TYPE_CIRCLE` (12 B read) and vice versa — C reinterprets the bytes with no check; Rust must reinterpret identically. **Also covers misaligned pointers**: the C does `*(c2Circle *)A` on a `const void *`, which on x86-64 lowers to ordinary loads that succeed at any address. | same `int` as C for the reinterpreted bytes |
| 10 | `c2CircletoCircle` | `A.r` or `B.r` is `NaN` ⇒ `r2` is `NaN` ⇒ `comiss` unordered ⇒ `seta` false | returns `0` (no error code, sentinel rejection) |
| 11 | `c2CircletoCircle` / `c2CircletoAABB` | any input coordinate is `NaN` ⇒ `d2` is `NaN` ⇒ unordered compare | returns `0` |
| 12 | `c2CircletoCircle` / `c2CircletoAABB` | negative radius (`A.r < 0`): **not** rejected by C — `r2 = r*r` is positive, so a negative radius behaves like its absolute value | returns the same as `|r|`; must not be "fixed" |
| 13 | `c2AABBtoAABB` | inverted / degenerate box (`min > max`, `min == max`, `NaN` corner): no validation; result is `!(d0|d1|d2|d3)` over the four unordered-false comparisons | `1` whenever all four `<` are false (so an all-`NaN` box "collides") |

## Notes on untestable-by-design cases

Dereferencing a genuinely null pointer with a *valid* type tag
(`collided(NULL, C2_TYPE_CIRCLE, …)`) is undefined behaviour in the C — it
segfaults. It is therefore **not** part of the error surface (the C does not
"reject" it) and is deliberately not exercised as a differential test; doing so
would crash both libraries rather than compare them. Rows 6–8 cover the null
pointer cases that the C actually *defines* (rejected by the switch before any
dereference).

## Divergence found and fixed (row 9)

`err09_tag_struct_size_mismatch` sweeps `collided` over every byte offset of a
`[u8; 40]` buffer, so `A`/`B` are misaligned for most offsets. The original Rust
`collided` used plain dereferences:

```rust
c2CircletoCircle(unsafe { *(A as *const c2Circle) }, unsafe { *(B as *const c2Circle) })
```

A plain `*ptr` in Rust requires the pointer to be aligned. The C has no such
requirement — `*(c2Circle *)A` on `const void *A` compiles to ordinary `movss` /
`mov` loads that succeed at any address — so a caller may legitimately pass a
misaligned pointer and the C returns a normal result. The Rust version instead
hit `misaligned pointer dereference` and **aborted the process** under debug
assertions (and was UB in release, where LLVM is free to assume alignment).

Fixed by switching all four dereferences in `collided` to
`core::ptr::read_unaligned`, which is what actually matches the C's observable
behaviour. Verified: the driver script fails with the old code and passes with
the fix, in both `debug` and `release`.

Note the failure mode this exposed in the *harness*: because the cdylib is built
with `panic = "abort"`, the misaligned-pointer check kills the test process
before libtest can print `test result: FAILED`. `run-tests.sh` therefore judges
each run by its **exit status** (via `PIPESTATUS`), never by grepping stdout.
