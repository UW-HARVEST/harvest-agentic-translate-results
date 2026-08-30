# Mutant shared objects — suite self-validation

These are deliberately WRONG re-implementations of `driver`, used to prove the
differential suite has real detection power. `verify.sh` compiles each one into a
`.so`, points the suite at it via `DRIVER_RUST_SO`, and asserts the suite
**fails**. A suite that passes against a mutant is not testing anything.

| mutant | injected bug | must be caught by |
|--------|--------------|-------------------|
| `mutant_floor.rs` | floor/Euclidean division instead of C's truncate-toward-zero | Phase B (`CONFIGS.md` rows 2, 4, 6, 11–19) |
| `mutant_nontrap.rs` | `checked_div` swallows the divide-error instead of raising `SIGFPE` | Phase C (`ERRORS.md` rows 1, 2, 3) |

Note that `mutant_floor.rs` is correctly NOT flagged by rows 1, 3, 5, 7–10: floor
and truncating division coincide when the operands share a sign, when the
division is exact, or when `|y| == 1`. And `mutant_nontrap.rs` passes all of
Phase B — it is only wrong on the error paths, which is precisely why Phase C
exists as a separate phase.
