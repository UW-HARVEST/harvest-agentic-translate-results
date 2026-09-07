# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping every `return`, every
`if (...)` guard, every `NULL` mention, every allocation call and every
unchecked pointer use. There are **no** `assert`s, **no** error enums and **no**
`RETURN_ERROR`-style macros in this library; the entire rejection vocabulary is
`NULL` (from `allocate_block`) and `-1` (from `betagamma`), plus a set of
unchecked-pointer paths that fault.

`sizeof(MemoryBlock) == 16`, `sizeof(int) == 4`, `sizeof(DataBlock) == 40`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `allocate_block` | `malloc(16)` fails (`lib.c:50`, `if (!mb) return NULL;`) — not reachable from a test harness on this platform; documented for completeness | returns `NULL`, nothing leaked |
| 2 | `allocate_block` | `calloc(count, 4)` fails (`lib.c:53`) because `count` is huge: `count == SIZE_MAX` | returns `NULL` (and `free(mb)` first — no leak) |
| 3 | `allocate_block` | `calloc` fails because `count * 4` overflows `size_t`: `count == SIZE_MAX/4 + 1`, `count == SIZE_MAX/2` | returns `NULL` |
| 4 | `allocate_block` | `calloc` fails because the request is merely too large to satisfy: `count == 1 << 62`, `count == SIZE_MAX/8` | returns `NULL` |
| 5 | `allocate_block` | `count == 0` — **not** an error: `calloc(0, 4)` returns a unique non-`NULL` pointer, the init loop body never runs | returns non-`NULL` with `size == 0`, `data != NULL` |
| 6 | `free_block` | `mb == NULL` (`lib.c:68`, guarded) | no-op, no crash |
| 7 | `free_block` | `mb != NULL` but `mb->data == NULL` (`lib.c:69`, guarded) — inner `free` skipped, outer `free(mb)` still runs | no crash; only `mb` is freed |
| 8 | `betagamma` | `(param1 % 10) + 5 < 0`, i.e. `param1 % 10 <= -6` (C `%` truncates toward zero, so this needs `param1 <= -6` with residue in `{-6,-7,-8,-9}`): `param1 ∈ {-6,-7,-8,-9,-16,-17,-18,-19,-26,…,INT_MIN(-8)}`. The negative `int` sign-extends into `size_t`, `calloc` fails, `mem1` and `mem2` are both `NULL` | returns `-1` (`lib.c:133`) |
| 9 | `betagamma` | `param1 == INT_MIN` specifically: `INT_MIN % 10 == -8` → `block_size == (size_t)(-3)` | returns `-1` |
| 10 | `betagamma` | `(param1 % 10) + 5 == 0`, i.e. `param1 % 10 == -5` (`param1 ∈ {-5,-15,-25,…}`) — **boundary, not an error**: `block_size == 0`, both allocations succeed, `sum1 == sum2 == 0` | returns a normal result, **not** `-1` |
| 11 | `betagamma` | `(param1 % 10) + 5 == 1..4`, i.e. `param1 % 10 ∈ {-4,-3,-2,-1}` — **boundary, not an error**: one step "past" the erroring residues | returns a normal result, **not** `-1` |
| 12 | `compute_hash` | `mb1 == NULL` — `lib.c:77` dereferences `mb1->data` with **no** null check | fatal fault (`SIGSEGV`) |
| 13 | `compute_hash` | `mb2 == NULL` — `lib.c:77` dereferences `mb2->data` with no null check | fatal fault (`SIGSEGV`) |
| 14 | `compute_hash` | both `NULL` | fatal fault (`SIGSEGV`) |
| 15 | `compute_hash` | `mb1->data == mb2->data` **and** `mb1 == mb2` — both `if`/`else if` chains fall through | returns `0` (neither 100/200 nor 10/20 added) |
| 16 | `create_block` | `name == NULL` — `lib.c:43` `strcpy(block.name, name)` has no null check | fatal fault (`SIGSEGV`) |
| 17 | `create_block` | `strlen(name) > 31` — `strcpy` into `char name[32]` overflows the returned struct (`lib.c:43`); formally UB, so only the in-range boundary `strlen(name) == 31` is asserted as a hard equality | at `strlen == 31`: exact 31 bytes + NUL fills `name`; beyond that: UB, not asserted |
| 18 | `betagamma` | there is **no** input for which `betagamma` rejects other than row 8/9 — every `int` quadruple outside those residues returns a computed value, including ones where the `int` arithmetic overflows (`param2 == INT_MAX` etc.) | wrapping result, never `-1` |

## Notes on non-error-code rejections

* Rows 12–14 and 16 are the only ways to make this library fault. They are
  verified differentially by launching the driver in a child process and
  comparing the C and Rust *termination signal*, not merely "both failed".
* There is no out-of-range enum in this API: the four `betagamma` parameters and
  `create_block`'s `flags` are plain integers with no valid-variant set, so
  every bit pattern is a legal input. The equivalent "invalid enum" test here is
  passing `flags` values that satisfy none, some, or all of the four
  `0b00001111 / 0b11110000 / 0b10101010 / 0b01010101` masks — covered as axis
  **F** of `CONFIGS.md` for `create_block`, and fixed by the three hard-coded
  blocks inside `betagamma`.
* `betagamma` cannot be made to hit rows 1–4 for only *one* of its two
  allocations: both calls use the same `block_size`, so `mem1` and `mem2`
  succeed or fail together. The `!mem1 || !mem2` short-circuit's asymmetric
  halves are therefore unreachable, and `free_block(NULL)` (row 6) is what
  actually executes on the error path.

## Where each row is verified

| rows | test |
|------|------|
| 2, 3, 4 | `tests/error_paths.rs::err02/err03/err04` — `allocate_block` returns `NULL` in both |
| 5 | `err05` — `count == 0` is non-`NULL`, `size == 0` |
| 6 | `err06` — `free_block(NULL)` no-op |
| 7 | `err07` — `mb->data == NULL` skips the inner `free` |
| 8, 9 | `err08`, `err09` — `-1`, plus an exhaustive `param1 ∈ [-2000, 2000]` sweep in `err10_11` asserting C's `-1` set matches the residue rule exactly |
| 10, 11 | `err10_11` — boundary residues return a normal value, not `-1` |
| 12, 13, 14 | `tests/driver.rs` `crash_hash_null_{first,second,both}` — both libraries die with `signal:11` |
| 15 | `err15` — aliased `compute_hash(p, p)` returns `0` in both |
| 16 | `tests/driver.rs` `crash_create_null_name` — both die with `signal:11` |
| 17 | `err17` — name lengths `0..=31`, NUL lands at the same index |
| 18 | `err18` — 4000 randomized quadruples plus all 256 `flags` values; C's accept/reject decision matches the rule and Rust's matches C's |
| 1 | not reachable from a test harness; documented only |

All rows pass in both the debug and release profiles and under every feature
combination (`scripts/check_features.sh`).
