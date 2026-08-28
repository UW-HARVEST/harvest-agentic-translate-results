# FINDINGS.md — what the differential verification found

C ground truth: `c_src/src/lib.c` (9 functions), built by `c_src/CMakeLists.txt`
as `libharvest-work-jsRX3G.so` (no `CMAKE_BUILD_TYPE`, i.e. `-O0`).
Rust under test: `translation/src/lib.rs` → `libarity_lib.so` (`cdylib`).
Both are loaded with `libloading` and compared through their exported symbols
only; no Rust function is ever called directly.

## Divergences found in the Rust translation (all fixed)

### 1. NULL-pointer dereference aborted instead of faulting (`SIGABRT` vs `SIGSEGV`)

`process_string(NULL)`, `init_matrix(NULL)`, `arity(len>=2, NULL)` and
`shift_array(NULL, size, positions)` with a passing guard dereference an unchecked
pointer. The C library dies with `SIGSEGV`; the Rust translation used plain `*p`
derefs, and Rust inserts a language-level null check whenever the crate is built
with `-C debug-assertions` (the `dev` profile default), so it died with `SIGABRT`
after printing `null pointer dereference occurred`.

*Found by* `phase_c_errors::e7_crash_parity` (runs each implementation in a child
process and compares the terminating signal).
*Fix* — all pointer traffic now goes through unchecked accessors in `src/lib.rs`.

### 2. Misaligned pointer arguments aborted instead of working

The first version of the fix used `ptr::read_volatile`/`write_volatile`, which
have no null check but *do* carry an alignment precondition check under
`debug-assertions`. The C code reads `params[i]` with a plain `mov`, which is
happy with any address on x86-64, so a caller passing a misaligned buffer got
correct results from C and an abort from Rust.

*Found by* `phase_c_errors::e25_misaligned_pointers`.
*Fix* — the volatile access is performed through a `#[repr(C, packed)]` newtype
(`align_of == 1`), so neither the null check nor the alignment check is emitted.
Verified to compile to exactly `mov (%rdi),%eax`, the same instruction gcc emits.

### 3. Optimised builds elided the store/reload that `compare_allocations` observes

`compare_allocations` writes `val1` through `ptr1` and `val2` through `ptr2`, then
re-reads `*ptr1`. With plain derefs, LLVM knows both pointers come from `malloc`
(hence `noalias`) and that the memory is `free`d again, so at `opt-level > 0` it
deleted both stores *and* the reload and answered the `*uninit_ptr > 0` test from
`val1` in a register. gcc at `-O0` really stores and reloads, so the two
libraries disagreed whenever the two allocations alias: C reports the value
written **last** (`val2`), the optimised Rust build reported `val1`.

*Found by* `phase_c_errors::e24_pointer_order_branches` (release profile), which
uses an interposed `malloc` to return the same address twice.
*Fix* — the volatile accessors keep the memory traffic; `core::hint::black_box`
additionally hides the allocations' provenance as defence in depth. Each
mechanism alone is sufficient with the current toolchain; the mutation
`compare_allocations plain deref + no black_box` proves the test catches the
regression when both are removed.

## Bugs found in the *test harness* (fixed)

These matter because they would have produced false results.

1. **Flaky differential comparison (1 full-suite run in 5).** `compare_allocations`
   compares two `malloc` addresses, so its value depends on the process-wide
   glibc allocator state, which *both* libraries share. Comparing pairs of
   back-to-back calls (relying on the state having period two) broke down when the
   tcache bin was not in the assumed state, producing `C=(x,x)` vs `Rust=(y,x)`
   mismatches with no translation defect behind them.
   `tests/probe_alloc.rs` demonstrates the effect is environmental by loading the
   **same C `.so` twice** and showing the two C instances diverging from each
   other identically.
   *Fix* — `common::normalize_allocator(order)` canonicalises the `sizeof(int)`
   tcache bin before every measurement, forcing `ptr1 < ptr2` or `ptr1 > ptr2`.
   Comparisons became deterministic *and* stronger (the exact expected value is
   asserted, and both branches are hit deliberately). 40 consecutive suite runs
   with zero failures afterwards.
2. **Race compiling the `LD_PRELOAD` fixture.** Three tests built the shim to the
   same path on parallel threads, so a child could `dlopen` a half-written file.
   *Fix* — `OnceLock` + unique name + atomic rename.
3. **Child output parsing.** libtest prints `test child_worker ... ` without a
   newline, so the child's marker line was not at the start of a line.

## Test-power evidence (`./mutation_check.sh`)

Injecting a bug into `src/lib.rs` one at a time and requiring the suite to fail:

| profile | mutants caught | expected survivors | blind spots |
|---------|----------------|--------------------|-------------|
| `dev`     | 34 | 4 | **0** |
| `release` | 32 | 6 | **0** |

Expected survivors are provably behaviour-preserving mutants, each with its
reason recorded in the script, e.g.:

* `process_string` calling `strlen` unconditionally — `strlen("") == 0`, so the
  guard is redundant for every input, including `NULL` (which faults either way).
* signed instead of unsigned pointer comparison — no canonical x86-64 user-space
  address has bit 63 set, so `<` agrees for every pointer a caller can obtain.
  (gcc emits the unsigned `jae`/`jbe`, which is what the translation mirrors.)
* profile-sensitive mutants (`dev-only` / `release-only`), which survive in the
  profile where the corresponding check or optimisation does not exist.

## Behaviours deliberately preserved (not "fixed")

* `arity`'s parameter is `unsigned char` in the definition but `int` in
  `include/lib.h`; gcc only looks at the low 8 bits, so `arity(256, p)` is
  rejected (`-1`) while `arity(-1, p)` truncates to `255` and calls `arity4`.
* `param1 % 4` uses C truncating remainder, so a negative `param1` yields a
  negative selector that matches no `case` and falls into `default:`.
* Signed-overflow wrap-around in `result * param3` and `result + param4`
  (`-O0` gcc wraps; the translation uses `wrapping_*`).
* `compare_allocations` reading through `uninit_ptr` and comparing two unrelated
  `malloc` results.
* No bounds checks anywhere: `init_matrix` always writes 12 `int`s,
  `shift_array` trusts `size`, `arity` reads `params[0..len]`, `process_string`
  runs `strlen` past any "logical" end.

## Reproducing

```sh
cd translation
./run_all.sh                 # C build + all feature/profile combos + suites + mutation check
SKIP_MUTATION=1 ./run_all.sh # same without the (slower) mutation check
cargo test                   # single configuration
PROFILE=--release ./mutation_check.sh
```
