# Undefined behaviour in the C source, and how the Rust build handles it

`c_src/src/lib.c` validates nothing. Three inputs reachable through its public
API make the C read or write memory it never initialised or never owned. For
those, "byte-identical output" is not a defined target — the C has no defined
output. This file records each case, what was measured, and the deliberate
choice made in the Rust translation.

## 1. `c2GJK` with an out-of-range `C2_TYPE` (`ERRORS.md` row 36)

`c2GJK` declares `c2Proxy pA; c2Proxy pB;` as plain stack locals and
`c2MakeProxy`'s `switch` has **no `default:` label**. An invalid `type`
therefore leaves the proxy completely uninitialised, and `c2GJK` then reads
`pA.count` (feeding `c2Support`'s loop bound) and `pA.verts[...]`.

Measured (`tests/error_paths.rs::probe_c_gjk_bad_type_child`, isolated in a
child process): the C returns `dist = 0, iterations = 2` for all 24 invalid
type values tried — but only because our own harness calls `c2GJK` in a tight
loop, so the stack slot still holds the *previous* call's proxy. Any other
caller leaves something else there. Agreement with Rust: **0/24**.

**Choice:** Rust uses `c2Proxy::default()` (zeroed). This is deterministic and
memory-safe, and for every *valid* type it is overwritten in full by
`c2MakeProxy`, so it changes nothing on any defined path. Replicating the C
would require reading uninitialised Rust stack — UB in Rust, exploitable by
LLVM, and it would still only coincide for one specific call pattern.

The differential test therefore asserts:

* both libraries **return** (the C side isolated in a child process so a
  segfault cannot take the suite down), and
* the Rust result is **deterministic** across repeated identical calls.

## 2. Caller-supplied `c2GJKCache` indices (`ERRORS.md` row 26)

`c2GJK` reads `cache->iA[i]` for `i < cache->count` out of an `int[3]`, and
`pA.verts[iA]` out of a `c2v[8]`, with no validation. Three regimes:

| regime | C behaviour | Rust behaviour | tested how |
|--------|-------------|----------------|-----------|
| `count` 1..=3, `iA`/`iB` 4..=7 | in bounds of `verts[8]` but never written by `c2MakeProxy` → uninitialised stack (measured 39/272 samples differ, `tests/probe_ub.rs`) | reads the zeroed slot | Rust must return **deterministically**; values not compared |
| `iA >= 8` | reads past `c2Proxy::verts` into adjacent stack | same (raw pointer read) | isolated child process |
| `count > 3` | write loop `verts + i` overruns `c2Simplex`'s four `c2sv` slots and corrupts the caller's frame | same | isolated child process — **reliably SIGSEGV in both** |

**Choice that mattered:** the Rust translation originally used bounds-checked
indexing (`pA.verts[iA as usize]`, `cache->iA[i as usize]`, `saveA[i as usize]`).
That turned a C out-of-bounds *read* into a Rust **panic**, and because these
functions are `extern "C"` a panic cannot unwind — it calls `abort()` and kills
the host process. Aborting the caller is a far larger divergence from C than
reading an adjacent stack word. All such accesses were converted to raw pointer
arithmetic so the Rust build degrades the same way C does. See the comment at
`src/lib.rs` in `c2GJK`.

## 3. `c2Support` with `count` past the end of the array

`c2Support(verts, count, d)` reads `verts[0]` before testing `count` at all,
and loops to `count` regardless of the real array length. Both are faithfully
reproduced (`err_support_nonpositive_count`, `err_support_count_past_end`); the
tests over-allocate the vertex buffer so the over-read stays inside memory the
test owns and the comparison is meaningful.

## Summary

Everything on a **defined** C path is compared bit-for-bit. The only
relaxations are the three regimes above, where the C program has no defined
value to match, plus the NaN payload question documented separately in
`NOTES-nan.md`. Each relaxation is measured and isolated rather than assumed.
