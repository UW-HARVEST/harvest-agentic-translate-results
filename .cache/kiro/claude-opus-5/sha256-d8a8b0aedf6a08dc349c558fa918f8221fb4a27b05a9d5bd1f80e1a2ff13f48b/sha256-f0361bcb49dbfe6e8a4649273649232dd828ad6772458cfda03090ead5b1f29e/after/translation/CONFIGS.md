# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`: every
runtime branch, every mask constant, every value whose *shape* the code
special-cases. There are no `#ifdef`s, no global mode flags and no setup/option
API in this library — the "options" are the argument values themselves, so the
axes below are the argument shapes each entry point branches on.

## Axes the C code actually branches on

| axis | where | distinct values the C distinguishes |
|------|-------|--------------------------------------|
| **A** `count` shape | `allocate_block` | `0` (empty, `calloc(0,4)` non-NULL, loop skipped); `1` (single); small `2..14` (the range `betagamma` itself uses); large (`1000`, `65536`, many); huge (fails → `ERRORS.md` 2–4) |
| **B** `init_value` shape | `allocate_block` `mb->data[i] = init_value + i` | `0`; positive; negative; `INT_MAX` (the `int→size_t→int` round trip wraps mid-array); `INT_MIN`; values within `count` of `INT_MAX` so only *part* of the array wraps |
| **C** `data`/`mb` state | `free_block` | `NULL`; non-NULL with `data != NULL`; non-NULL with `data == NULL` |
| **D** data-pointer order | `compute_hash` `mb1->data` vs `mb2->data` | `<` (+100), `>` (+200), `==` (+0) |
| **E** struct-pointer order | `compute_hash` `mb1` vs `mb2` | `<` (+10), `>` (+20), `==` (+0) |
| **F** `flags` mask coverage | `create_block` (free) / `betagamma` (fixed at `0xAA`,`0xCC`,`0xF0`) | which of `0b00001111`, `0b11110000`, `0b10101010`, `0b01010101` are hit: none (`0x00`), low-only, high-only, odd-only, even-only, all (`0xFF`), plus every one of the 256 values |
| **G** `name` length | `create_block` / `strcpy` | `0` (empty); `1`; `11` (`"Block_Alpha"`, the length `betagamma` uses); `31` (exactly fills `char[32]`); `>31` = UB (`ERRORS.md` 17) |
| **H** heap-address relation | `betagamma` → `compute_hash`, and `mem1->data != mem2->data`, and `mem1->data > NULL` | **not** an argument — an emergent property of the allocator. `betagamma`'s return value is a function of allocator state, so C-vs-Rust equality is only well-defined when both run from the *same* heap state. Verified by running identical call sequences in **separate fresh processes** and diffing stdout byte-for-byte (see `tests/driver.rs`); in-process interleaving is provably invalid here. |
| **I** `param1 % 10` residue | `betagamma` `block_size = (param1 % 10) + 5` | all 10 non-negative residues (`block_size` 5..14); residue `-5` → `block_size 0`; residues `-1..-4` → `block_size 1..4`; residues `-6..-9` → huge → `-1` |
| **J** `param2..param4` shape | `betagamma` `flag_contribution` accumulation, `sum2`, `(sum1-sum2)/10` | zero; positive; negative; mixed sign; `INT_MAX`/`INT_MIN` (signed wrap in `result +=` and `flag_contribution *`); values making `sum1-sum2` negative so `/10` truncates **toward zero** |
| **K** entry-point level | public surface | lowest level (`create_block`, `allocate_block`, `free_block`, `compute_hash`) called directly; and the composed top-level `betagamma` which drives all of them |

## Configuration rows

Every row is exercised through both `.so`s via `libloading` with many
randomized inputs from a fixed-seed PRNG (`SEED = 0x5DEECE66D`), not a single
hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `create_block` | G=0 (empty name), F=`0x00` (no mask hit), id `0` | [x] |
| 2 | `create_block` | G=1, F=`0xFF` (all four masks), id negative | [x] |
| 3 | `create_block` | G=11 (`betagamma`'s own name length), F=`0xAA`/`0xCC`/`0xF0` (the three literal block flags) | [x] |
| 4 | `create_block` | G=31 (name exactly fills `char[32]`), F random | [x] |
| 5 | `create_block` | G random 0..31 × F all 256 values × id ∈ {0, ±random, INT_MAX, INT_MIN} — full cross-product sweep | [x] |
| 6 | `allocate_block`+`free_block` | A=0 (empty) × B=0 — `calloc(0,4)` non-NULL, `size==0`, no element writes | [x] |
| 7 | `allocate_block`+`free_block` | A=1 (single) × B ∈ {0, +, −} | [x] |
| 8 | `allocate_block`+`free_block` | A=5..14 (exactly the sizes `betagamma` requests) × B random signed | [x] |
| 9 | `allocate_block`+`free_block` | A=large (`1000`, `65536`) × B random signed — checks the whole `init_value + i` ramp | [x] |
| 10 | `allocate_block`+`free_block` | A=large × B=`INT_MAX` — `init_value + i` wraps *mid-array* (`int→size_t→int` truncation) | [x] |
| 11 | `allocate_block`+`free_block` | A=large × B=`INT_MIN` | [x] |
| 12 | `allocate_block`+`free_block` | A random × B ∈ `[INT_MAX-count, INT_MAX]` so only the tail of the array wraps | [x] |
| 13 | `free_block` | C=non-NULL with `data` manually set to `NULL` (heap struct built by the test, `data` cleared) | [x] |
| 14 | `compute_hash` | D=`<` × E=`<` → 110 | [x] |
| 15 | `compute_hash` | D=`<` × E=`>` → 120 | [x] |
| 16 | `compute_hash` | D=`<` × E=`==` → 100 — **unreachable by construction**: `E ==` means `mb1 == mb2`, which forces `mb1->data == mb2->data`, i.e. `D ==`. Asserted unreachable rather than exercised. | [x] |
| 17 | `compute_hash` | D=`>` × E=`<` → 210 | [x] |
| 18 | `compute_hash` | D=`>` × E=`>` → 220 | [x] |
| 19 | `compute_hash` | D=`>` × E=`==` → 200 — **unreachable by construction**, same aliasing argument as row 16 | [x] |
| 20 | `compute_hash` | D=`==` × E=`<` → 10 | [x] |
| 21 | `compute_hash` | D=`==` × E=`>` → 20 | [x] |
| 22 | `compute_hash` | D=`==` × E=`==` → 0 | [x] |
| 23 | `compute_hash` | D driven by randomized *synthetic* `data` pointer values incl. `0`, `usize::MAX` and values `> 2^63` (so a signed-vs-unsigned comparison bug shows up). `compute_hash` never dereferences `data`, so arbitrary values are legal inputs here. The **struct** pointers `mb1`/`mb2` *are* dereferenced, so they must be real addresses; on x86-64 Linux every dereferenceable address has the sign bit clear, which makes axis E's comparison signed-vs-unsigned indistinguishable by construction (verified as an equivalent mutant, see below) | [x] |
| 24 | `compute_hash` | inputs produced by real `allocate_block` calls (both orders of the two blocks passed to `compute_hash`) | [x] |
| 25 | `betagamma` | I=residues 0..9 (`block_size` 5..14) × J=all params small positive | [x] |
| 26 | `betagamma` | I=residue `-5` (`block_size == 0`) — `param1 ∈ {-5,-15,-25,-35}` | [x] |
| 27 | `betagamma` | I=residues `-1..-4` (`block_size` 1..4) | [x] |
| 28 | `betagamma` | J: all four params `0` | [x] |
| 29 | `betagamma` | J: all four params negative, so `sum1-sum2 < 0` and `/10` truncates toward zero | [x] |
| 30 | `betagamma` | J: mixed signs, randomized over the full `int` range | [x] |
| 31 | `betagamma` | J: `param2 = INT_MAX`, `param3 = INT_MIN` etc. — signed overflow in `flag_contribution`, `* id` and `result +=` | [x] |
| 32 | `betagamma` | J: `param1` fixed per residue × `param2..4` swept so `(sum1-sum2)` straddles `0` and multiples of `10` | [x] |
| 33 | `betagamma` | I=erroring residues `-6..-9` and `INT_MIN` → `-1` (also `ERRORS.md` 8–9) | [x] |
| 34 | `betagamma` (composed, H) | identical randomized call *sequence* (600 calls, fixed seed) replayed in two fresh processes, one per `.so`; full stdout compared byte-for-byte — the only sound way to compare the address-dependent result | [x] |
| 35 | all five entry points (composed, H) | full mixed-workload sequence: `create_block`/`allocate_block`/`compute_hash`/`free_block`/`betagamma` interleaved in a randomized order, replayed per-`.so` in fresh processes, stdout diffed byte-for-byte | [x] |
| 36 | `betagamma` | repeated identical call, 8 iterations — verifies the C and Rust allocator-state *cycle* (period 2/4) matches, not just one sample | [x] |

## Why `betagamma` cannot be compared in-process (axis H, established empirically)

`betagamma` returns `... + compute_hash(mem1, mem2) + ...`, and `compute_hash`
compares raw heap addresses. So `betagamma` is a function of its arguments *and
of glibc's allocator state*, and each call mutates that state (the four
allocations are freed in an order that reshuffles the tcache bins). Measured on
the C library alone, repeating the *same* call cycles through several values:

```
p1=0 -> 497 597 607 507 497 597 607 507 ...      (period 4)
p1=3 -> 517 607 517 607 ...                      (period 2)
```

Calling the C `.so` and then the Rust `.so` in one process therefore compares
two *different* heap states and reports spurious differences: 48 of 60 inputs
"mismatched", every single one by exactly one of {±10, ±90, ±100, ±110} — i.e.
purely a `compute_hash` term, never an arithmetic difference. Both libraries
produce the *same cycle* of values, just at a different phase.

Hence the split used here:

* the address-**independent** arithmetic is compared in-process
  (`tests/valid_paths.rs`), asserting that any residual difference is one of the
  four legal `compute_hash` deltas — so a real arithmetic divergence still fails;
* the **exact** value, `compute_hash` term included, is compared by replaying an
  identical call sequence in two fresh processes, one per `.so`, and diffing
  stdout byte-for-byte (`tests/driver.rs`, rows 34–36). ~307 KB of stdout across
  five scenarios matches exactly, in both the debug and release profiles.

## Negative controls (does the suite actually have teeth?)

Every row above passing on the first attempt is only meaningful if the suite can
detect a wrong translation. 22 mutations were injected into `src/lib.rs` one at a
time, rebuilt, and run against the full suite. **18 were caught**; the 4 that
were not are provably unobservable through the public ABI:

| mutation | result | caught by |
|----------|--------|-----------|
| `compute_hash`: `+100` → `+200` | caught | driver stdout diff |
| `compute_hash`: `+10` → `+11` | caught | driver stdout diff |
| `compute_hash`: `data` compared as `isize` | caught | row 23 |
| `(sum1-sum2)/10` → `div_euclid` (floor) | caught | driver stdout diff |
| `block_size`: `%` → `rem_euclid` | caught | driver stdout diff |
| `block_size`: `+5` → `+6` | caught | driver stdout diff |
| block literal `id: 3` → `4` | caught | driver stdout diff |
| `special.id` `99` → `98` | caught | driver stdout diff |
| `+= special.flags` → `+= 254` | caught | driver stdout diff |
| `mem1->data != mem2->data` → `==` | caught | driver stdout diff |
| `allocate_block`: `size = count` → `count+1` | caught | driver stdout diff |
| `allocate_block`: drop the `calloc` null check | caught | driver status mismatch |
| `betagamma`: drop the `!mem1 \|\| !mem2` check | caught | driver status mismatch |
| `free_block`: drop the outer `if (mb)` guard | caught | driver status mismatch |
| `create_block`: `flags` → `flags ^ 1` | caught | driver stdout diff (`mixed`) |
| `create_block`: bounded copy instead of `strcpy` | caught | `rust_so_uses_the_platform_allocator` |
| `compute_hash`: `mb1`/`mb2` compared as `isize` | **not caught — equivalent** | on x86-64 Linux every dereferenceable address has bit 63 clear, and `compute_hash` *dereferences* `mb1`/`mb2`, so no legal input can distinguish the two comparisons |
| `free_block`: drop the inner `if (mb->data)` guard | **not caught — equivalent** | `free(NULL)` is a defined no-op in C, so the C guard is redundant |
| `init_value + i` computed in `i32` instead of `size_t` | **not caught — equivalent** | the two agree mod 2^32 for every `i < 2^31`; distinguishing them needs an ≥8 GB allocation |
| flag mask `0b10101010` → `0b10101011` | **not caught — equivalent** | bit 0 is clear in all three literal `flags` (`0xAA`, `0xCC`, `0xF0`) and no public entry point feeds other values into these masks |
