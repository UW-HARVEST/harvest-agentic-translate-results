# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/lib.c`

Derived mechanically from the branches the C source actually takes.

## Axes the C code branches on

**A. Build-time configuration.**  None.  `c_src/CMakeLists.txt` compiles the
single TU `src/lib.c` with no `-D`, no `option()`, no conditional sources; and
`grep -n '#if\|#ifdef\|#ifndef' c_src/src/lib.c` returns nothing.  There is
exactly one C configuration, hence exactly one Rust feature combination
(`default` = empty, identical to `--no-default-features`).  Every row below is
run under **both** cargo invocations by `scripts/verify_all.sh`.

**B. Run-time options / modes.**  The library has no option struct, no flag
parameter that selects a mode, and no global state.  The "options" are purely
the argument values, so the axes are the value classes of the arguments:

| axis | where the C branches on it | classes |
|------|----------------------------|---------|
| `create_block.name` length | `strcpy` copy length (lib.c:43), NUL position | `0`, `1`, mid (`5`,`11`), `31` (last byte that fits), `32..35` (past the field but inside the 40-byte struct) |
| `create_block.name` bytes | `char` is **signed** on x86-64, so `name[i]` values `≥0x80` become negative `c_char` | pure ASCII / bytes with the high bit set / `0x01`+`0xFF` mixes |
| `create_block.flags` | stored verbatim (lib.c:44); `uint8_t` in a 32-bit register slot | `0x00`, `0x01`, `0x0F`, `0x55`, `0xAA`, `0xF0`, `0xFF`, random, and *wide* (`>0xFF`) values seen across the ABI |
| `create_block.id` | stored verbatim | `0`, `±small`, `±large`, `INT_MAX`, `INT_MIN` |
| `allocate_block.count` | `calloc(count,4)` success/failure (lib.c:52-56) and the `i < count` loop bound (lib.c:60) | `0`, `1`, `2`, `5..14` (the range `betagamma` can request), `4096`, overflowing / unmappable (→ `ERRORS.md` #2/#3) |
| `allocate_block.init_value` | `mb->data[i] = init_value + i` — `int`→`size_t` conversion, add, truncate back to `int` (lib.c:61) | `0`, `±small`, `±large`, `INT_MAX` (wraps **inside** the loop), `INT_MIN`, `-1` (crosses 0), random |
| `free_block.mb` | `if (mb)` / `if (mb->data)` (lib.c:68-69) | `NULL`, full block, block with `data == NULL`, block allocated by the *other* library |
| `compute_hash` pointer order | `mb1->data <=> mb2->data` (lib.c:77-81) × `mb1 <=> mb2` (lib.c:83-87) | 3 × 3, of which 7 are reachable (`mb1 == mb2` forces `data1 == data2`) |
| `betagamma.param1 % 10` | `block_size = (param1 % 10) + 5` (lib.c:126) | all 19 residues `-9..9` ⇒ `block_size ∈ {-4..14}`; `-9..-6` fail (`ERRORS.md` #6), `-5` ⇒ 0, `-4..9` ⇒ `1..14` |
| `betagamma` sum difference | `(sum1 - sum2) / 10` — C truncates **toward zero**, so the sign of the dividend is a real branch (lib.c:147) | `>0` and `≥10`, `>0` and `<10`, `== 0`, `<0` and `>-10`, `≤ -10` |
| `betagamma` overflow class | `flag_contribution*id`, `result+=`, `sum+=`, `sum1-sum2` are `int` (lib.c:123,137,141,144,147) | no overflow / overflow in the flag phase / overflow in the sum phase / overflow in the difference |
| `betagamma` `data` inequality | `mem1->data != mem2->data` (lib.c:152) and `mem1->data > NULL && mem2->data > NULL` (lib.c:156) | both always true for two live allocations, including `count == 0`; asserted for every row |

**C. Notes that constrain what may be compared.**

* **N1 — indeterminate bytes.** `create_block` starts from an *uninitialised*
  `DataBlock block;` (lib.c:41) and only `strcpy`s `strlen(name)+1` bytes, so
  `name[strlen(name)+1 .. 31]` is stack garbage in C (verified: the C library
  returns non-zero junk there for short names).  Comparisons therefore cover
  `id`, `flags`, and `name[0 ..= strlen(name)]` — everything the C code
  actually defines.
* **N2 — allocator-address dependence.** `compute_hash` (and therefore
  `betagamma`) branches on the *numeric values* of `malloc`/`calloc` results,
  so `betagamma` is not even a function of its arguments: the shipped C library
  returns 517 / 527 / 617 / 627 for `betagamma(1,2,3,4)` depending on the tcache
  state of the caller's heap.  All `betagamma` comparisons are therefore run in
  a **`fork()`ed child**: parent forks once per implementation from the *same*
  heap image, so both libraries observe byte-identical allocator state and any
  remaining difference is a genuine translation defect.
* **N3 — same allocator.** Both libraries must call the platform
  `malloc`/`calloc`/`free` (not Rust's `GlobalAlloc`), otherwise blocks are not
  interchangeable and the `compute_hash` addresses come from different arenas.
  Row 44 pins this down.

## Configuration table

Every row is exercised with **many randomized inputs** drawn from the stated
classes (`SplitMix64`, fixed seed `0x5EED_1234_ABCD_0001`, see
`tests/common/mod.rs`), not a single hand-picked value, and both libraries are
called through their `.so` exports via `libloading`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `create_block` | `name` length 0 (empty string), `id = 0`, `flags = 0` | [x] |
| 2 | `create_block` | `name` length 1, random `id`, random `flags` | [x] |
| 3 | `create_block` | `name` length 2..30 random ASCII, random `id`, random `flags` | [x] |
| 4 | `create_block` | `name` length exactly 31 (fills the field, NUL is the last byte) | [x] |
| 5 | `create_block` | `name` length 32..35 (NUL lands on `flags`/padding — `flags` is reassigned afterwards) | [x] |
| 6 | `create_block` | `name` containing bytes `0x80..0xFF` (negative `signed char`), lengths 1..31 | [x] |
| 7 | `create_block` | `flags` swept over **all** 256 values with a fixed name/id | [x] |
| 8 | `create_block` | `id ∈ {0, 1, -1, INT_MAX, INT_MIN, ±10^9}` × `flags ∈ {0x00,0xFF}` | [x] |
| 9 | `create_block` | fully random `(id, name, flags)`, 2000 iterations | [x] |
| 10 | `allocate_block` + `free_block` | `count = 0` (boundary: `calloc(0,4)` non-NULL), `init_value` random | [x] |
| 11 | `allocate_block` + `free_block` | `count = 1`, `init_value ∈ {0, ±small, INT_MAX, INT_MIN}` | [x] |
| 12 | `allocate_block` + `free_block` | `count = 2..14` (exactly the range `betagamma` requests), random `init_value` | [x] |
| 13 | `allocate_block` + `free_block` | `count = 4096` (multi-page, `calloc` may switch to `mmap`), random `init_value` | [x] |
| 14 | `allocate_block` + `free_block` | `init_value = INT_MAX` with `count ≥ 2` ⇒ `data[i]` wraps to `INT_MIN` mid-loop | [x] |
| 15 | `allocate_block` + `free_block` | `init_value = INT_MIN` / `-1` / `-count` ⇒ `data[i]` crosses zero | [x] |
| 16 | `allocate_block` + `free_block` | fully random `(count ∈ 0..64, init_value ∈ full i32)`, 1000 iterations, every element compared | [x] |
| 17 | `free_block` | `NULL` (guard `if (mb)`) | [x] |
| 18 | `free_block` | hand-built `MemoryBlock{ data: NULL, size: n }` (guard `if (mb->data)`) | [x] |
| 19 | `compute_hash` | `mb1 == mb2` (same object) ⇒ `data1 == data2`, `p1 == p2` ⇒ **0** | [x] |
| 20 | `compute_hash` | `data1 < data2`, `p1 < p2` ⇒ **110** | [x] |
| 21 | `compute_hash` | `data1 < data2`, `p1 > p2` ⇒ **120** | [x] |
| 22 | `compute_hash` | `data1 > data2`, `p1 < p2` ⇒ **210** | [x] |
| 23 | `compute_hash` | `data1 > data2`, `p1 > p2` ⇒ **220** | [x] |
| 24 | `compute_hash` | `data1 == data2` (aliased buffer), `p1 < p2` ⇒ **10** | [x] |
| 25 | `compute_hash` | `data1 == data2` (aliased buffer), `p1 > p2` ⇒ **20** | [x] |
| 26 | `compute_hash` | `data1 == data2 == NULL`, `p1 < p2` and `p1 > p2` ⇒ **10** / **20** | [x] |
| 27 | `compute_hash` | random synthetic pointer values (`0`, `1`, `usize::MAX`, `0x8000…`) — checks that the comparison is **unsigned**, as C pointer comparison is | [x] |
| 28 | `compute_hash` | fed with real `allocate_block` results (both orders), 200 iterations | [x] |
| 29 | `betagamma` | `param1 % 10 == 0` ⇒ `block_size = 5`; all params random | [x] |
| 30 | `betagamma` | `param1 % 10 == 1..9` (9 sub-rows) ⇒ `block_size = 6..14`; all params random | [x] |
| 31 | `betagamma` | `param1 % 10 == -1..-4` (4 sub-rows) ⇒ `block_size = 4..1`; all params random | [x] |
| 32 | `betagamma` | `param1 % 10 == -5` ⇒ `block_size = 0` (empty sum loops, `ERRORS.md` #14) | [x] |
| 33 | `betagamma` | all four params `0` (the all-zero shape) | [x] |
| 34 | `betagamma` | `param1 == param2` ⇒ `sum1 - sum2 == 0` ⇒ quotient exactly 0 | [x] |
| 35 | `betagamma` | `param1 > param2` with `(sum1-sum2) < 10` ⇒ positive dividend, quotient truncates to 0 | [x] |
| 36 | `betagamma` | `param1 > param2` with `(sum1-sum2) ≥ 10` ⇒ positive quotient | [x] |
| 37 | `betagamma` | `param1 < param2` with `(sum1-sum2) > -10` ⇒ **negative** dividend, quotient must truncate toward zero (not floor) | [x] |
| 38 | `betagamma` | `param1 < param2` with `(sum1-sum2) ≤ -10` ⇒ negative quotient | [x] |
| 39 | `betagamma` | `param3`/`param4` large so `flag_contribution * id` overflows `int` | [x] |
| 40 | `betagamma` | `param1 = INT_MAX` (⇒ `block_size = 12`) so `sum1` overflows inside the loop | [x] |
| 41 | `betagamma` | `param1 = INT_MIN` (⇒ `% 10 == -8` ⇒ the `-1` error path, `ERRORS.md` #15) | [x] |
| 42 | `betagamma` | each of the 16 sign patterns of `(param1..param4)` with large magnitudes | [x] |
| 43 | `betagamma` | fully random `(i32, i32, i32, i32)`, 1500 iterations, fork-isolated (covers every residue class and overflow shape) | [x] |
| 44 | `allocate_block` / `free_block` **cross-library** | block from the C `.so` freed by the Rust `.so` and vice-versa (proves both use the same platform allocator, note N3) | [x] |
| 45 | `create_block` | `flags` supplied through a *wide* register slot (`0x1FF`, `-1`, `0x7FFFFF00`) — narrow-parameter ABI truncation (`ERRORS.md` #16) | [x] |
| 46 | `betagamma` | repeated invocation in one process (heap state cycles through tcache) — asserts the C and Rust value sequences are identical, i.e. the allocation *pattern* matches, not just one call | [x] |

## How the rows are executed

| test binary | rows | technique |
|-------------|------|-----------|
| `tests/differential_create_block.rs` | 1-9, 45 | direct `.so` calls, ~5 800 inputs, defined-region field comparison (note N1) |
| `tests/differential_alloc.rs` | 10-18, 44 | direct `.so` calls; every element of every returned buffer compared; cross-library free and forced-`malloc`-failure cases run in forked children |
| `tests/differential_compute_hash.rs` | 19-28 | synthetic `MemoryBlock`s in a 2-element array (guaranteed address order) with planted `data` values, so all 7 reachable order combinations plus unsigned-comparison boundaries are hit deterministically; ~4 500 comparisons |
| `tests/differential_betagamma.rs` | 29-43, 46 | `fork_pair` isolation (note N2); ~2 900 inputs, each result additionally checked against an independent `model_without_hash` oracle that predicts everything except the 9 possible `compute_hash` values |
| `tests/symbol_parity.rs` | — | Phase D `nm -D` diff |

`scripts/traceability.sh` verifies mechanically that each row number above is
referenced by a label inside `tests/`; `scripts/verify_all.sh` runs the whole
suite for every feature combination and both cargo profiles.

### Mutation testing (proof the rows are not vacuous)

Eighteen single-line mutations were injected into `src/lib.rs`, rebuilt, and the
suite re-run (`--test-threads=1`, failure = non-zero `cargo test` status).
**All sixteen behaviour-changing mutants were caught**; the two survivors are
*provably equivalent* mutants and are analysed below the table.

| mutation | caught by |
|----------|-----------|
| `(sum1-sum2)/10` → `div_euclid` (floor instead of truncate) | `betagamma_differential` (rows 37/38) |
| signed instead of unsigned pointer comparison | `compute_hash_differential` (row 27) |
| flag mask `0b00001111` → `0b00011111` | `betagamma_differential` |
| `data[i]` truncation `as u32` → mask `0x7fffffff` | `allocate_and_free_differential` (rows 14/16) |
| `hash += 100` → `101` | `betagamma_differential` |
| `block_size` sign-extend → zero-extend | `betagamma_differential` (rows 31/32 + ERRORS #6) |
| `block.flags = flags` → `flags \| 1` | `create_block_differential` (row 7) |
| `block.id = id` → `id + 1` | `create_block_differential` (row 8) |
| `free_block` NULL guard removed | `allocate_and_free_differential` (row 17) |
| `allocate_block` rejects `count == 0` | `allocate_and_free_differential` (row 10) |
| `compute_hash` `p1 < p2` → `p1 <= p2` | `compute_hash_differential` (row 19) |
| `special.flags as c_int` → `as i8 as c_int` | `betagamma_differential` |
| `flag_contribution * id` → `… + 1` | `betagamma_differential` |
| `free_block` NULL guard removed (2nd form) | `allocate_and_free_differential` (row 17, fork-isolated signal diff) |
| `allocate_block` leaks `mb` instead of `free(mb)` on `calloc` failure | `betagamma_differential` (the leak perturbs the allocator addresses `compute_hash` observes) |
| `betagamma` error path returns `0` instead of `-1` | `betagamma_differential` (ERRORS #6) |

Two mutants survived, and **both are equivalent mutants** — no test can
distinguish them because the C code's own observable behaviour is unchanged:

| survivor | why it is equivalent |
|----------|----------------------|
| `free_block`: drop the `if (mb->data)` guard (lib.c:69) | the guard is redundant: without it the call becomes `free(NULL)`, which ISO C defines as a no-op.  Behaviour is identical by definition. |
| `strcpy_raw`: copy a fixed 32 bytes instead of stopping at the NUL | the C code `strcpy`s in three places (lib.c:43, 107, 150).  Line 107 writes `temp_name`, which is *never read*; line 150 writes `special.name`, which is *never read* (only `special.id` and `special.flags` reach the result); line 43's bytes after the NUL are *indeterminate* in C (note N1) and are therefore excluded from every comparison.  The extra bytes are unobservable through the public API. |

## Completion status

- [x] Every one of the 46 rows passes across its randomized inputs.
- [x] Every row passes for feature combination `--no-default-features`.
- [x] Every row passes for feature combination `default`.
- [x] Every row passes for the `release` profile of both combinations.
