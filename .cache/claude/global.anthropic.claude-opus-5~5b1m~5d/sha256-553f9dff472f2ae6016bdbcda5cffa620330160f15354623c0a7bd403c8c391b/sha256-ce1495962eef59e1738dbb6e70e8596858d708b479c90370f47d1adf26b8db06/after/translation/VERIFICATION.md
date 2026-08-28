# VERIFICATION.md — results

Run everything with `./verify.sh` (builds the C `.so`, enumerates feature
combinations, runs all phases in both profiles, then diffs `nm -D`).

## Completion gate

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing/undefined non-libc symbols in Rust | **PASS** (1/1 symbol; diff empty in both profiles) |
| Phase B: every row in `CONFIGS.md` passes across randomized inputs | **PASS** (30/30 rows) |
| Phase C: every row in `ERRORS.md` has a passing error-path differential test | **PASS** (13/13 rows + 2 generic-boundary tests) |
| All of the above under every feature combination | **PASS** (crate declares no `[features]` → single configuration; verified in debug *and* release) |

Totals: **50 tests** (30 Phase B + 17 Phase C + 3 Phase D), green in both
`cargo test` and `cargo test --release`.

Every call in every test crosses the FFI boundary via `libloading::Library::get`
on **both** `.so`s — the Rust implementation is never invoked as a Rust
function, so the `#[unsafe(no_mangle)] extern "C"` export is itself under test.

## Divergence found and fixed

One real bug, caught by Phase C (`ERRORS.md` rows 7/8/11):

* **Symptom** — `pix == NULL, w == INT_MIN, h == 2` (and the `w < 0` family
  generally): the C is a clean no-op, the Rust aborted with
  `unsafe precondition(s) violated: ptr::offset requires the address calculation
  to not overflow`.
* **Cause** — `src/lib.rs` used `pix.offset(off)`. The C still *computes* the row
  pointers `pix + w*i` / `pix + w*(h-i-1)` before the inner `j < w` guard
  rejects, so with a negative `w` it legitimately forms an out-of-bounds address
  and then never dereferences it. `ptr::offset` declares that address
  calculation UB and traps it under debug assertions.
* **Fix** — use `wrapping_offset` for the row pointers and the `++a` / `++b`
  increments. That is the same two's-complement address arithmetic the C codegen
  emits, with no precondition, so the no-op is reproduced exactly.

## Documented profile-dependent behaviour (not a divergence)

`ERRORS.md` rows 12–13 (`img == NULL`, and `img->pix == NULL` while work is due)
are hard faults, compared by re-executing the test binary in a child process and
checking the **exact signal**:

| | C | Rust (release) | Rust (debug) |
|---|---|---|---|
| `img == NULL` | SIGSEGV (11) | SIGSEGV (11) | SIGABRT (6) |
| `img->pix == NULL`, `w=4,h=2` | SIGSEGV (11) | SIGSEGV (11) | SIGABRT (6) |

The release `.so` — the shipped artifact, and the apples-to-apples counterpart
of the optimised C — faults **identically**. In debug builds Rust's std
instruments raw place-reads with a null-dereference UB detector that panics on
the very same UB; that is a sanitiser making the failure louder, not a
behavioural difference. The test therefore pins the exact expected signal per
profile rather than accepting "both failed somehow".

## Test-suite strength (mutation testing)

The differential suite was validated by injecting plausible translation bugs
into `src/lib.rs` and confirming the tests fail. **15 of 16 mutants caught:**

caught — `flips = h/2 + 1`; mirror row `h-i-1 → h-i`; mirror row `h-i-2`;
row stride `w*i → i`; non-wrapping `ptr::offset` (the real bug above);
`h/2` via unsigned `(h as u32)/2`; inner guard `j < w → j <= w`; outer guard
`i < flips → i <= flips`; `j < w → j < w-1`; drop `*b = t` (copy not swap);
swap reversed; swap without temp; only `r,g,b` swapped (alpha preserved);
only alpha swapped; `a` advanced by `-1`; `b` never advanced.

not caught — `flips = (h+1)/2`. This is an **observationally equivalent
mutant**, not a gap: for even `h`, `(h+1)/2 == h/2` exactly; for odd `h` the one
extra iteration has `i == (h-1)/2` and mirror row `h - i - 1 == (h-1)/2 == i`,
i.e. it swaps the middle row with itself — a no-op. Brute-forced over
`h ∈ -8..199 × w ∈ 0..8`: 0 divergences. The two forms differ only at
`h == INT_MAX` (where `h+1` overflows), which needs a ≥8 GiB buffer to reach and
is already listed under `ERRORS.md`'s excluded genuine-UB cases.

## Notes on the C that were deliberately preserved

* The function is named `flip_horizontal` but swaps **rows**, i.e. it flips the
  image *vertically*. Reproduced as-is, not "fixed".
* Odd `h` leaves the middle row untouched (asserted explicitly in
  `row09_w1_h3_middle_row_preserved`).
* No validation whatsoever: no null checks, no range checks, no return value.
  Degenerate/negative dimensions are silent no-ops, and null pointers fault.
* The descriptor is never written back — `img->w`, `img->h`, `img->pix` are
  unchanged on return. Asserted for both libraries on every single test case.
* Only `w*h` pixels are touched; guard pixels around every test buffer confirm
  neither implementation writes out of bounds.
