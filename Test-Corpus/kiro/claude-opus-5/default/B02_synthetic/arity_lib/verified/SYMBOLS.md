# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-nzg71d.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/release/libarity_lib.so | awk '{print $3}' | sort
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt      # missing from Rust
```

The whole C library is a single translation unit (`c_src/src/lib.c`, 181 lines);
`c_src/CMakeLists.txt` builds exactly one target, `SHARED` from `src/lib.c`.
There is no second module and no macro-generated symbol family, so the C export
set below is the complete surface. There is no executable target (no
`add_executable`, no `[[bin]]`, no `src/main.rs`), so the "compare binary
stdout" clause of the completion gate does not apply.

## Symbol table

| # | symbol | in C `.so` | in Rust `.so` | C signature (from `src/lib.c`) | notes |
|---|--------|-----------|---------------|-------------------------------|-------|
| 1 | `apply_bitmask`       | yes | yes | `int apply_bitmask(int value, int operation)` | pure |
| 2 | `arity`               | yes | yes | `int arity(unsigned char len, int *params)` | **header/definition mismatch — see below** |
| 3 | `arity2`              | yes | yes | `int arity2(int p1, int p2)` | wrapper → `arity4(p1,p2,0,0)` |
| 4 | `arity3`              | yes | yes | `int arity3(int p1, int p2, int p3)` | wrapper → `arity4(p1,p2,p3,0)` |
| 5 | `arity4`              | yes | yes | `int arity4(int,int,int,int)` | drives every helper |
| 6 | `compare_allocations` | yes | yes | `int compare_allocations(int val1, int val2)` | **allocator-address dependent — see below** |
| 7 | `init_matrix`         | yes | yes | `void init_matrix(int matrix[3][4])` | param decays to `int (*)[4]` |
| 8 | `process_string`      | yes | yes | `int process_string(const char *str)` | |
| 9 | `shift_array`         | yes | yes | `void shift_array(int *arr, int size, int positions)` | `memmove` + zero fill |

**Symbol diff: EMPTY.** `comm -23` produces no output — 0 symbols missing from
the Rust `.so`. No symbol is stubbed or `unimplemented!()`; every one of the 9
has a real translation of the corresponding C body in `translation/src/lib.rs`.

The Rust `.so` additionally exports the standard cdylib housekeeping symbols
(`_init`, `_fini`, `rust_eh_personality`, and the allocator shims). Extra Rust
symbols are permitted by the gate; only *missing* C symbols are a failure.

## Undefined (imported) symbols

```sh
nm -D --undefined-only translation/target/release/libarity_lib.so
```

All undefined symbols resolve to libc (`malloc`, `free`, plus the unwinder /
`__libc_start_main`-family symbols the toolchain always emits). 0 missing
non-libc symbols.

## Two symbols whose ABI/behaviour needed a deliberate decision

### `arity` — the `int` vs `unsigned char` mismatch

`c_src/include/lib.h` declares:

```c
int arity(int len, int *params);
```

but `c_src/src/lib.c` *defines*:

```c
int arity(unsigned char len, int *params) { if (len < 2) return -1; ... }
```

A caller compiled against the header passes a full 32-bit `int` in `%edi`. The
callee, disassembled from the built `.so`, truncates it:

```
1634:  mov    %edi,%eax
163a:  mov    %al,-0x4(%rbp)     <-- keeps only the low 8 bits
163d:  cmpb   $0x1,-0x4(%rbp)
1641:  ja     164a               <-- unsigned comparison
```

So the observable contract at the `.so` boundary is `len & 0xFF`, compared
*unsigned*. The Rust export takes `c_int` (matching the public header) and masks
to 8 bits, reproducing this exactly — e.g. `len = 256` and `len = -256` both
truncate to 0 and return `-1`, while `len = -1` truncates to 255 and takes the
`arity4` branch. Verified in Phase C.

### `compare_allocations` — genuinely allocator-dependent

The C compares the *addresses* returned by two `malloc` calls:

```c
if (ptr1 < ptr2) result = 1; else if (ptr1 > ptr2) result = 2; else result = 3;
```

Because glibc's tcache is LIFO, `malloc,malloc,free,free` reverses the order of
the two chunks, so consecutive calls return `1,2,1,2,...` (+10 when `val1 > 0`).
This is process-global state **shared** by the C `.so` and the Rust `.so`, which
call the same libc `malloc`. The Rust translation is therefore faithful by
construction — it performs the same two `malloc`s and the same two `free`s in
the same order — but a naive "call C once, call Rust once" test is guaranteed to
diverge simply because the second call observes a flipped heap.

The tests handle this by comparing **phase-neutral call pairs**: each side is
called twice in a row and the two-element result *sequences* are compared. Two
calls restore the tcache to its entry state, so the pattern is self-resetting
and independent of the starting parity. This is strictly stronger than a single
call — it observes *both* heap phases per input. Applies to
`compare_allocations` and to everything that transitively calls it
(`arity4`, `arity3`, `arity2`, `arity`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the default (and only) feature combination is the empty set.
`cargo test --no-default-features` is equivalent to `cargo test`. Both are run
in Phase D for completeness.

## Completion gate — evidence

Run `./phase_d.sh` to reproduce all of this in one command.

| gate item | status | evidence |
|-----------|--------|----------|
| `nm -D`: 0 missing/undefined non-libc symbols in Rust | PASS | `comm -23` of the two sorted `nm -D` outputs is empty (9 C symbols, 9 matched). `ldd -r` reports 0 unresolved imports for both `.so`s |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | PASS | 43 rows ↔ 43 `row*` tests in `tests/phase_b.rs`, all green |
| Binary stdout comparison | N/A | no executable target: no `add_executable` in `c_src/CMakeLists.txt`, no `[[bin]]`, no `src/main.rs`. `phase_d.sh` asserts this rather than assuming it |
| Phase C: every `ERRORS.md` row has a passing error-path test | PASS | 24 rows ↔ 24 `row*` tests in `tests/phase_c.rs`, plus 2 `extra_*` tests for generic FFI-boundary values, all green |
| Holds under every feature combination | PASS | `Cargo.toml` declares no `[features]`; suite green under both `cargo test` and `cargo test --no-default-features`, and under both `--release` and debug (debug enables Rust overflow checks, which the wrapping arithmetic must survive) |

Stability: 20 consecutive full-suite runs (10 release + 10 debug, default
parallel threads) with 0 failing runs, plus single-threaded runs of both
profiles.

## Mutation testing — evidence that the tests can actually fail

Passing tests only mean something if they can detect a wrong translation. 33
mutants were injected into `translation/src/lib.rs`, rebuilt, and run against
the suite. **29 were killed; the 4 survivors are provably semantically
equivalent to the original**, so no mutant representing a real behavioural
change escaped.

Killed (a sample of the 29): dropping the `arity` 8-bit truncation; `len < 2`
→ `len < 1`; `positions < size` → `<= size`; `memmove` → `copy_nonoverlapping`;
each bitmask constant; `default:` arm → `0`; truncating → euclidean division;
`/100` → `/10`; `param3 != 0` → `> 0`; `param4 != 0` → `> 0`; `param1 % 4` →
`rem_euclid(4)`; `matrix[2][3]` → `matrix[2][2]`; `init_matrix` value 12 → 13;
`shift_array` shift 1 → 2 and zero-fill → one-fill; `arity` `len == 3` →
`len == 4`; `arity4` `count: 4` → `3`; `"Hello"` → `"Hell"`; `strlen`
off-by-one; `compare_allocations` swapping the `<`/`>` arms, swapping results
1/2, bonus 10 → 9, `> 0` → `>= 0`, and `uninit_ptr = ptr1` → `ptr2`.

Survivors, each equivalent by construction:

| mutant | why it cannot be detected |
|--------|---------------------------|
| `if (*str)` guard removed from `process_string` | when `*str == 0`, `strlen` returns 0 — identical to the guard's `return 0`. The C's guard is redundant |
| `arity3(a,b,c)` inlined to `arity4(a,b,c,0)` | that is the literal definition of `arity3` |
| tie branch `result = 3` → `4` | the `ptr1 == ptr2` branch is unreachable: two simultaneously-live allocations cannot share an address |
| `len1 + len2` → `len1 - len2` in `arity4` | `len2 = process_string("") = 0`, so both forms are `len1` |

Methodology note: mutants whose search pattern landed inside the Rust doc
comments (which quote the original C) are no-ops and were rejected by an
explicit guard, so they are not miscounted as survivors.

## The one genuinely environment-dependent function

`compare_allocations` compares two raw `malloc` addresses, so its result is a
function of allocator state, which is not part of the library's input. Two
distinct effects had to be separated from real divergence:

1. **tcache LIFO alternation.** `malloc,malloc,free,free` reverses the order of
   the two chunks, so consecutive calls return `1,2,1,2,…`. Measured over
   200,000 adjacent call pairs, alternation never once failed. A probe of
   "N `arity4` calls, then read the phase" confirmed C and Rust advance the
   allocator in exact lockstep for N = 0..4, i.e. the Rust does no extra
   allocation.
2. **glibc arena contention.** Under `cargo test`'s parallel threads,
   `arena_get` can fall back to a different arena when the preferred one is
   locked, so `ptr1` and `ptr2` may come from different arenas and the ordering
   stops alternating cleanly. This produced genuinely nondeterministic results
   that were *not* translation bugs.

`tests/harness/mod.rs::assert_alloc_diff` handles both: it serialises every
allocator-sensitive measurement behind a mutex, measures C and Rust
**adjacently with no intervening allocation** (so both pairs start from the same
phase and can be compared **in order**, not as a set), and re-measures the C
side afterwards, only reporting a divergence once the C side has reproduced its
own ordered pair. Ordered comparison is what makes the swapped-`<`/`>` and
swapped-`1`/`2` mutants detectable — a multiset compare would let both survive.
