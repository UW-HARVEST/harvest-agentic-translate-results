# ERRORS.md — ERROR-SURFACE TABLE (Phase A / Phase C)

Every distinct way `c_src/src/lib.c` rejects, skips, or errors on input.
Derived mechanically by grepping **all** `return`, `if`, `switch`/`default`,
`NULL`, `assert`, and guard statements in the C source (there are no `assert`s
and no error enums in this library; rejection is expressed as sentinel returns
`-1` / `0`, as silent no-ops, and as a `switch` `default` fall-through).

Test file: `tests/error_paths.rs`. `[x]` = differential test written AND passing
against BOTH `.so`s.

| # | function | trigger (exact invalid input/condition) | expected C result | [x] |
|---|----------|------------------------------------------|-------------------|-----|
| E1 | `shift_array` (`lib.c:36`) | `positions <= 0` (e.g. `0`, `-1`, `INT_MIN`) — guard `positions > 0` fails | silent no-op: array left **completely unmodified**, no memmove, no zero-fill | [x] |
| E2 | `shift_array` (`lib.c:36`) | `positions >= size` (e.g. `size=4, positions=4`, `positions=INT_MAX`) — guard `positions < size` fails | silent no-op: array unmodified | [x] |
| E3 | `shift_array` (`lib.c:36`) | `size <= 0` (e.g. `size=0` or `size=-4`) with any `positions>0` — `positions < size` cannot hold | silent no-op: array unmodified | [x] |
| E4 | `process_string` (`lib.c:45,48`) | empty string `""` (first byte is `NUL`) — `if (*str)` false | returns `0` (does **not** call `strlen`) | [x] |
| E5 | `apply_bitmask` (`lib.c:66`) | `operation` outside the handled set `{0,1,2,3}` — `switch` `default`. Includes out-of-range "enum" ints across FFI: `4`, `5`, `-1`, `-4`, `INT_MIN`, `INT_MAX` | returns `value` **unchanged** (no mask applied) | [x] |
| E6 | `compare_allocations` (`lib.c:91-95`) | `malloc` returns `NULL` for either allocation | `free(ptr1); free(ptr2); return -1` | [x] (documented / not inducible — see note A) |
| E7 | `arity` (`lib.c:172-173`) | `len < 2` after **8-bit truncation**: `len ∈ {0, 1}` | returns `-1`; `params` is **never dereferenced** (safe with `NULL`) | [x] |
| E8 | `arity` (`lib.c:172-173`) | `len` whose low byte is `< 2` although the `int` passed is large/positive: `256`, `257`, `512`, `65536`, `65537` | returns `-1` (truncation ⇒ low byte `0`/`1`) | [x] |
| E9 | `arity` (`lib.c:171-173`) | **negative** `len` passed through the `int` public prototype: `-1`, `-2`, `-256`, `-255`, `INT_MIN` | low byte reinterpreted **unsigned**: `-1`→`255`, `-2`→`254` ⇒ **not** `< 2` ⇒ dispatches to `arity4` (reads 4 params). `-256`→`0`, `INT_MIN`→`0` ⇒ returns `-1` | [x] |
| E10 | `arity` (`lib.c:174-179`) | `len` ≥ 4 (incl. `255`) — the `else` branch reads **exactly 4** `params`, never `len` of them | dispatches to `arity4(params[0..3])`; elements past index 3 ignored | [x] |
| E11 | `process_string` (`lib.c:45`) | `str == NULL` — C has **no** null check, dereferences immediately | UB ⇒ `SIGSEGV` (crash) | [x] (crash-signal parity, forked child) — **found a real divergence, see Note C** |
| E12 | `arity` (`lib.c:175-179`) | `params == NULL` with `len >= 2` — no null check before `params[0]` | UB ⇒ `SIGSEGV` (crash) | [x] (crash-signal parity, forked child) |
| E13 | `shift_array` / `init_matrix` | `arr`/`matrix == NULL` with a size that makes the guard pass (`shift_array(NULL,4,1)`), `init_matrix(NULL)` — no null checks | UB ⇒ `SIGSEGV` (crash) | [x] (crash-signal parity, forked child) |

## Boundary cases additionally covered in `tests/error_paths.rs`

These are not distinct C rejection branches, but are the generic C-API
boundaries the task requires (zero/oversized lengths, one-past-range values,
out-of-range enum ints across FFI):

| case | covered by |
|------|-----------|
| out-of-range enum ints for `apply_bitmask`'s `operation` (`4`, `-1`, `INT_MIN`, `INT_MAX`, plus 2 000 random `i32`) | E5 test + `tests/valid_paths.rs` row C7 |
| one step past each valid `arity` length: `1`/`2` (lower edge), `3`/`4` (wrapper edge), `255`/`256` (truncation edge) | E7–E10 tests |
| `shift_array` with `size = 0`, `size = 1`, `positions = size-1`, `positions = size`, `positions = INT_MAX`, `size = INT_MIN` | E1–E3 tests |
| `process_string` on zero-length, 1-byte, 4 096-byte, and high-bit (`0x80..0xFF`, i.e. **negative** `c_char`) buffers | E4 test + row C5 |
| integer-overflow shapes in `arity4` (`INT_MAX`/`INT_MIN` params, wrapping `result * param3`, `/100` truncation toward zero, `param1 % 4` with negative `param1`) | `tests/valid_paths.rs` rows C13–C19 |
| `arity` with `len` covering **all 256** low-byte values | E7–E10 test `arity_all_256_low_byte_values` |

## Note A — why E6 is not inducible in-process

`compare_allocations` allocates `sizeof(int)` (4 bytes) twice. Forcing `malloc`
to fail for a 4-byte request cannot be done without replacing the allocator
(e.g. `LD_PRELOAD`/`malloc` interposition), which would perturb the C and Rust
sides differently and is outside the FFI surface under test. The Rust code is
verified by **inspection** to be a structurally identical translation of the
C branch — same null test (`||`), same `free` of *both* pointers (including the
possibly-`NULL` one, which is legal for `free`), same `-1` sentinel:

```c   if (ptr1 == NULL || ptr2 == NULL) { free(ptr1); free(ptr2); return -1; }
```
```rust if ptr1.is_null() || ptr2.is_null() { free(..ptr1); free(..ptr2); return -1; }
```

The *success* path of `compare_allocations` is fully differentially tested
(rows C8–C9), including its allocator-state dependence — see Note B.

## Note B — `compare_allocations` is allocator-state dependent (verified, by design)

`compare_allocations` compares the **addresses** returned by two consecutive
`malloc` calls (`ptr1 < ptr2` → 1, `>` → 2, `==` → 3). Under glibc this is
**not** a pure function of its arguments: the tcache free-list is LIFO, so each
call returns the previous call's two chunks in swapped order. Measured, on both
libraries:

```
C  compare_allocations(5,7) x8 : 11 12 11 12 11 12 11 12
R  compare_allocations(5,7) x8 : 11 12 11 12 11 12 11 12
```

Both implementations alternate **identically** because both call the real libc
`malloc`/`free` (the Rust translation deliberately links `extern "C" malloc`
rather than using Rust's allocator API). The consequence for testing: a naive
`assert_eq!(c_fn(x), rust_fn(x))` interleaving one C call with one Rust call
compares two *different* allocator states and reports a **false** divergence
(`C=12, R=11`). Every test touching `compare_allocations`/`arity*` therefore
uses parity-neutral **2-call batches** (`tests/common/mod.rs::batch2`): two
consecutive calls restore the allocator to its prior state, so the C pair and
the Rust pair observe the same two states and must match element-wise. This is
a property of the C code that the translation must (and does) reproduce — not a
bug to "fix".

## Note C — real divergence found and FIXED: NULL deref crashed differently

The E11–E13 crash-parity tests caught a genuine behavioural difference at the
FFI boundary:

```
NULL-pointer behaviour differs for `process_string_null`: C=Signal(11) Rust=Signal(6)
```

* C `.so`: dereferences `NULL` and dies with **SIGSEGV (11)**.
* Rust `.so` (as originally built): the dev profile enables debug assertions,
  which inject a UB check that panics with `null pointer dereference occurred`
  at `src/lib.rs:69`. Because the exported functions are `extern "C"`, that panic
  is non-unwinding, so the process **aborts with SIGABRT (6)**.

An external C consumer can observe that difference, so it is a translation
defect, not a test artefact. Fixed by making the cdylib behave like the C in
every profile (`Cargo.toml`):

```toml
[profile.dev]
debug-assertions = false   # no Rust-only NULL/UB traps ...
overflow-checks  = false   # ... and hardware wrapping for signed overflow
```

(The release profile already had both off, which is why only debug builds
diverged.) After the fix all seven NULL cases report
`C and Rust both -> Signal(11)`.

## Note D — comparing long call SEQUENCES needs separate processes

Note B's parity-neutral 2-call batching is sufficient for per-call rows, but it
is **not** sufficient for a long sequence run inside one process: after the C
sequence finishes, the tcache holds a different set of chunks than it did at the
start (allocations of the same size class from unrelated code move in and out),
so the Rust sequence does not see the same allocator evolution. Two weaker
approaches were tried and rejected because both produced off-by-one *false*
divergences:

1. counting the calls that reach `compare_allocations` and adding one corrective
   call when the count is odd;
2. measuring the state in-process with a probe and re-aligning it.

Row C32 therefore runs each library's sequence in its **own freshly spawned
process** (`SEQ_ENV`), where the allocator is pristine and evolves identically
for both sides. 2 000 calls per process compare exactly.
