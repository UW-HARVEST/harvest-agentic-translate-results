# ERRORS.md — Error-surface table

## How this was derived

Mechanical grep over the whole C source (`c_src/src/lib.c`, 126 lines;
`c_src/include/lib.h`, 1 line):

```sh
grep -nE 'return[[:space:]]+(-|NULL|0)|assert|RETURN_ERROR|errno|abort|exit\(|goto' src/lib.c include/lib.h
#   -> no matches
grep -cE '\bif\b|\bassert\b' src/lib.c
#   -> 0
grep -nE 'return' src/lib.c
#   src/lib.c:107:  return v0 ^ v1 ^ v2 ^ v3;
#   src/lib.c:111:  return stbds_siphash_bytes(p, len, seed);
```

**Result: this library has an empty error surface.** There is not one
`if`, `assert`, range check, null check, `errno` write, error enum, sentinel
return, `goto`, `abort`, or `exit` anywhere in it. Both `return` statements
return an ordinary computed value. `stbds_hash_bytes` is a *total* function
over its input domain: every `size_t` return value is a legitimate hash, so
there is no in-band value it could use to signal an error even if it wanted to.
`siphash` returns `void`.

The only control-flow branch in the entire file is the
`switch (len - i)` at `src/lib.c:48`, and that is a *dispatch* on valid input
(the 0..7-byte tail), not a rejection.

Consequently the table below has **no rows of the form "C rejects X"** — there
are none to have. What it does contain is every generic C-API boundary
condition mandated by the verification protocol, each with the behaviour the C
*actually* exhibits (verified by disassembly and by differential test), because
"the C accepts this and returns a value" is itself a contract the Rust must
match exactly.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `stbds_hash_bytes` | any input whatsoever — there is no error return path (`src/lib.c:110-112`, only `return stbds_siphash_bytes(...)`) | never signals an error; always returns a `size_t` hash | `err_no_error_return_path_exists` | [x] |
| 2 | `stbds_hash_bytes` | `p == NULL`, `len == 0` | **no dereference**: `len==0` ⇒ main loop body never runs, `switch(0)` falls to `case 0: break`. Returns the fixed len-0 hash for that seed. Well-defined, no crash. | `err_null_ptr_zero_len` | [x] |
| 3 | `stbds_hash_bytes` | `p == NULL`, `len == 0`, `seed` swept over edge values (`0`, `1`, `SIZE_MAX`, `SIZE_MAX/2`, …) | same as #2 per seed; `~seed` is computed on `SIZE_MAX` without incident | `err_null_ptr_zero_len_seed_sweep` | [x] |
| 4 | `stbds_hash_bytes` | `len == 0` with a **valid** non-null `p` | identical result to #2 (pointer is never read) | `err_zero_len_valid_ptr_equals_null_ptr` | [x] |
| 5 | `stbds_hash_bytes` | `len == 0` with a **dangling/garbage** non-null `p` (e.g. `0x1`, `usize::MAX`) | identical result to #2; still no dereference, so no fault | `err_zero_len_garbage_ptr` | [x] |
| 6 | `stbds_hash_bytes` | `seed == SIZE_MAX` (so `~seed == 0`) — the max/min constant boundary of the only scalar knob | no rejection; folds `0` into `v1`/`v3` normally | `err_seed_boundary_values` | [x] |
| 7 | `stbds_hash_bytes` | misaligned `p` (odd address, 1..7-byte offsets into a buffer) | no alignment check exists; all reads are `movzbl` single-byte loads, so any alignment is accepted and gives the same answer as the aligned copy | `err_misaligned_pointer_accepted` | [x] |
| 8 | `stbds_hash_bytes` | `len` one step past each internal block boundary (`7/8/9`, `15/16/17`, `63/64/65`) — "one past a documented valid range" | no rejection; selects the next `switch (len - i)` arm. All 8 tail arms 0..7 reachable and none error. | `err_len_one_past_block_boundaries` | [x] |
| 9 | `stbds_hash_bytes` | oversized `len` (1 MiB) against a buffer that is actually that large | no length cap or rejection exists; hashes all of it | `err_oversized_len_within_allocation` | [x] |
| 10 | `stbds_hash_bytes` | `switch (len - i)` `default` / `len - i > 7` (GCC emits a `cmp $0x7 / ja` guard skipping all arms) | **unreachable** by construction: the loop exit condition `i + 8 > len` plus `i <= len` forces `len - i ∈ [0,7]`. The Rust mirrors the guard (`rem <= 7` on every arm) so both take the no-op path if it were ever reached. | `err_switch_default_arm_unreachable` (documents + asserts the invariant) | [x] |
| 11 | `siphash` | any `int init` — there is no error path and no return value (`void`) | never signals an error | `siphash_stdout_differential_all_rows` / sub-case `err11 no-error-path` (also asserts exactly 64 lines for every init tested) | [x] |
| 12 | `siphash` | out-of-range / extreme `int` passed across FFI: `INT_MIN`, `INT_MAX`, `-1`, `0`. (`init` is a plain `int`, **not** an enum — see note below — so the whole `int` range is in-domain.) | no rejection; `mem[i] = (unsigned char)(init + i)` truncates, and at `INT_MAX` the `z++` wraps to `INT_MIN`. Prints the same 64 lines. | `siphash_stdout_differential_all_rows` / sub-case `err12 ffi-int-boundary` | [x] |
| 13 | `siphash` | `init` values chosen so the generated `mem` bytes straddle the `0x7f -> 0x80` high-bit boundary, which is what triggers the `int`-overflow sign-extension (`cltq`) inside the hash | no rejection; the sign-extension is part of the observable result | `siphash_stdout_differential_all_rows` / sub-cases `err13 crossing-position`, `err13 negative-crossing` | [x] |

### Out-of-range enum values

There are **no `enum` types** in the public API (`grep -n enum src/lib.c
include/lib.h` → no matches). The single scalar parameter, `siphash`'s
`int init`, already accepts the full `int` range with no valid/invalid
partition, so the "C enums accept any int" class of bug cannot arise here.
Row 12 nevertheless drives the full `int` range boundaries across the FFI
boundary to prove it.

### Deliberately excluded (undefined behaviour, not an error path)

| condition | why excluded |
|-----------|--------------|
| `p == NULL` with `len > 0` | Unconditional dereference ⇒ SIGSEGV in *both* C and Rust. Not an error *path*; it is UB. Comparing "both crash the test process" is not a meaningful differential assertion, and it would abort the harness. |
| `len` larger than the real allocation | Same: out-of-bounds read, UB. Row 9 covers large `len` with a genuinely large buffer instead. |
| `len` near `SIZE_MAX` | `i + sizeof(size_t)` overflows, so the loop guard wraps and the code reads unbounded memory ⇒ UB/SIGSEGV in both. Not testable. |

---

## Finding: the `seed` parameter provably CANCELS OUT (discovered during Phase C)

This was not visible from reading the happy path and is worth recording because
it changes what the seed-related rows above actually prove.

`c_src/src/lib.c:10-17` mixes `seed` into each state word **twice**:

```c
v0 = (C0) ^  seed;      /* :10 */
v1 = (C1) ^ ~seed;      /* :11 */
v2 = (C2) ^  seed;      /* :12 */
v3 = (C3) ^ ~seed;      /* :13 */
v0 ^= K0 ^  seed;       /* :14  -> seed ^ seed  == 0 */
v1 ^= K1 ^ ~seed;       /* :15  -> ~seed ^ ~seed == 0 */
v2 ^= K2 ^  seed;       /* :16 */
v3 ^= K3 ^ ~seed;       /* :17 */
```

Since `x ^ x == 0`, every occurrence of `seed` and `~seed` annihilates, leaving
`v0 = C0 ^ K0`, `v1 = C1 ^ K1`, etc. **`stbds_hash_bytes` is therefore
completely seed-independent**, verified empirically against the C `.so`:

```
len=  0 C hashes across 6 seeds: ALL IDENTICAL = 0x726fdb47dd0e0e31
len=  3 C hashes across 6 seeds: ALL IDENTICAL = 0x59299698de423050
len=  8 C hashes across 6 seeds: ALL IDENTICAL = 0x6ea8a07ad97b542d
len= 16 C hashes across 6 seeds: ALL IDENTICAL = 0xd0e1b06875efa716
len= 33 C hashes across 6 seeds: ALL IDENTICAL = 0x5fc35cefd84de0c6
```

The C is ground truth, so this is **not** a defect to repair — the Rust must be
seed-independent too, and it is.

Consequences for this table:

* Rows 3 and 6 still hold, but they prove "no crash / no rejection on extreme
  seeds", **not** "the seed is mixed in correctly". No test could prove the
  latter, because the C does not do it.
* A change that made the Rust genuinely honour the seed would be a **silent
  divergence** that no amount of seed sweeping could detect on its own — every
  seed-swept comparison would still pass on the C side while the Rust drifted.
  `quirk_seed_cancels_out_identically_in_both_libraries`
  (`tests/phase_c_errors.rs`) therefore asserts all three facts explicitly:
  C is seed-independent, Rust is seed-independent, and the two agree.

| # | function | trigger | expected C result | test | [x] |
|---|----------|---------|-------------------|------|-----|
| 14 | `stbds_hash_bytes` | any two different seeds, same bytes/len | **identical** hashes (seed cancels); C == Rust | `quirk_seed_cancels_out_identically_in_both_libraries` (81 lens x 11 seeds + 5 000 random quadruples) | [x] |
| 15 | `stbds_hash_bytes` | the two provably-unobservable code shapes (high-half sign- vs zero-extension; `rem == 7` vs `rem >= 7`) | indistinguishable by construction — proven as properties, not assumed | `quirk_equivalent_mutant_properties` | [x] |
