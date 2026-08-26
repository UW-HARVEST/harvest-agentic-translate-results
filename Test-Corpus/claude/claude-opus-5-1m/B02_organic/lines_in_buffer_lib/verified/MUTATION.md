# MUTATION.md — proof that the differential suite is not vacuous

Passing tests only mean something if they *can* fail. Every mutant below was
applied to `src/lib.rs`, the suite was re-run via `./run_tests.sh`, and the
original file was restored afterwards (verified with `diff -q`).

## The vacuousness bug this exercise found

The first mutation run reported **all mutants passing**. Root cause:

> `Cargo.toml` sets `crate-type = ["cdylib"]`. With no `rlib`, no integration
> test ever links the lib target, so **`cargo test` does not build
> `target/<profile>/libdriver.so` at all**. The suite was `dlopen`ing a stale
> `.so` left over from an earlier `cargo build`, and passed unconditionally
> regardless of what `src/lib.rs` said.

Two fixes, both in place:

1. `run_tests.sh` runs an explicit `cargo build` before `cargo test` for each
   feature combination and passes the resulting path in `RUST_SO`.
2. `tests/common/mod.rs::assert_so_fresh` refuses to run if the `.so` is older
   than any `.rs` file under `src/`, so the failure mode cannot recur silently.
   Verified: after `touch src/lib.rs`, a bare `cargo test` now exits 101 with
   *"STALE RUST .so — refusing to run vacuous tests."*

## Results after the fix

| mutant | change to `src/lib.rs` | detected? | failing tests |
|--------|------------------------|-----------|---------------|
| M3 | under-run check `line_index != num_lines` → `>` | **YES** | 13 |
| M5 | reject when `alloc_size == 0` (treat `malloc(0)` as failure) | **YES** | 6 |
| M7 | line start `buffer.add(pos)` → `buffer.add(pos + 1)` | **YES** | 21 |
| M8 | outer guard `&&` → `\|\|` | **YES** | crash / exit 1 |
| M9 | terminator test `!= 0` → `== 0` | **YES** | 16 |
| M12 | never skip the NUL (`if pos < buffer_size` → `if false`) | **YES** | 16 |
| M14 | outer guard `pos < buffer_size` → `pos <= buffer_size` | **YES** | crash / exit 1 |
| M16 | `size_of::<*const *const c_char>()` → `16` | **YES** | 1 (`cfg_25` only) |

M16 is detected *only* by `cfg_25_allocation_size_parity`, which is why that row
was added.

## Surviving mutants — each provably unobservable, not a coverage gap

| mutant | change | why no test can detect it |
|--------|--------|---------------------------|
| M1 | `if pos < buffer_size { pos += 1 }` → `if true { … }` | The inner loop's invariant gives `pos <= buffer_size` always, so the guard can only be false when `pos == buffer_size`; in that case the outer loop exits whether `pos` ends as `buffer_size` or `buffer_size + 1`. **Semantically equivalent.** |
| M4 | scan bound `pos + len < buffer_size` → `pos + len + 1 < buffer_size` | The *index* `buffer[pos + len]` is unchanged, so only the stop length changes. If a NUL exists at index `k <= buffer_size - 1` both stop at `len = k - pos`. If no NUL exists, the original ends with `pos = buffer_size` (no skip) and the mutant with `pos = buffer_size - 1` then `+1` = `buffer_size`. Line starts and final `pos` are identical. **Semantically equivalent.** |
| M15 | `*p != 0` on `c_char` (`i8`) → `(*p as u8) != 0u8` | `i8 != 0` ⟺ `i8 as u8 != 0` for every bit pattern. **Semantically equivalent.** (Confirms the signed-vs-unsigned `char` question is a non-issue for a `!= 0` test.) |
| M6 | delete the `free(buffer_ptrs)` on the rejection path | A leak is not observable through the return value. Both still return `NULL`. The Rust does keep the `free`, matching the C. |
| M2 | `wrapping_mul` → `saturating_mul` | Provably unobservable — see the "note on rows 8–11" in `ERRORS.md`: every `numLines >= 2^61` returns `NULL` under both, because returning non-NULL would require `bufferSize >= 2^61`. The Rust uses `wrapping_mul`, matching the C exactly. |

## Reproducing

```bash
./run_tests.sh                 # debug, all feature combos
./run_tests.sh --release       # release profile (panic = "abort")
```
