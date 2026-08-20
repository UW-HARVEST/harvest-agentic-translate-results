# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically from the C source.  Every `return NULL`, `return -1`,
every guard `if`, every unchecked dereference / unbounded copy, and every
boundary constant in `c_src/src/lib.c` gets its own row.

Greps used:

```sh
grep -n 'return' c_src/src/lib.c        # 6 return statements that can reject
grep -n 'if *(!\|if *(mb\|== *NULL\|!= *NULL\|> *NULL' c_src/src/lib.c
grep -n 'assert\|abort\|exit\|errno\|enum' c_src/src/lib.c   # -> no matches
grep -n 'strcpy\|malloc\|calloc\|free\|%' c_src/src/lib.c
```

There are **no** `assert`s, **no** error enums, **no** `errno` use, **no**
`abort`/`exit`, and **no** named min/max constants in the C source.  The only
numeric boundary constants that change control flow are `10` and `5` in
`block_size = (param1 % 10) + 5` (line 126) and the divisor `10` on line 147.

Legend for "expected C result": what the *compiled* C library actually does
(built with `-fPIC`, no optimisation, x86-64 glibc — the configuration produced
by `c_src/CMakeLists.txt`).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `allocate_block` (lib.c:49-50 `if (!mb) return NULL;`) | `malloc(sizeof(MemoryBlock))` returns `NULL` — process heap exhausted (forced in a forked child with `RLIMIT_AS` clamped and the heap drained) | returns `NULL`; `calloc` is never called; nothing is freed | `differential_alloc.rs` / `allocate_and_free_differential`, section `ERRORS #1` | [x] |
| 2 | `allocate_block` (lib.c:52-56 `if (!mb->data) { free(mb); return NULL; }`) | `count > SIZE_MAX / sizeof(int)` so `calloc(count, 4)` trips glibc's `nmemb*size` overflow check — e.g. `count = SIZE_MAX`, `SIZE_MAX/2`, `SIZE_MAX/4 + 1` | returns `NULL`, having released the `MemoryBlock` (no leak, no write to `mb->data`) | `differential_alloc.rs`, section `ERRORS #2` | [x] |
| 3 | `allocate_block` (same branch as #2) | `count` large but `count*4` does **not** overflow, yet cannot be mapped — e.g. `count = 1<<60`, `1<<55` | returns `NULL` (allocation refused before any element is written) | `differential_alloc.rs`, section `ERRORS #3` | [x] |
| 4 | `free_block` (lib.c:68 `if (mb)`) | `mb == NULL` | no-op, returns normally (must **not** call `free(NULL)`-adjacent garbage or crash) | `differential_alloc.rs`, section `row 17 / ERRORS #4` | [x] |
| 5 | `free_block` (lib.c:69 `if (mb->data) free(mb->data);`) | `mb != NULL` but `mb->data == NULL` (hand-built `MemoryBlock`) | inner `free` skipped, outer `free(mb)` performed, returns normally | `differential_alloc.rs`, section `row 18 / ERRORS #5` | [x] |
| 6 | `betagamma` (lib.c:130-134 `if (!mem1 \|\| !mem2) … return -1;`) | `param1 % 10 ∈ {-6,-7,-8,-9}` ⇒ `(param1%10)+5 ∈ {-1,-2,-3,-4}` ⇒ `block_size = (size_t)(−1..−4)` ⇒ `calloc` overflow ⇒ `mem1 == NULL` (left operand of `\|\|`) | returns `-1` exactly (no partial result, both blocks freed) | `differential_betagamma.rs` / `betagamma_differential`, residue classes `-9..-6` (assertion `ERRORS#6`) | [x] |
| 7 | `betagamma` (same branch, right operand) | `mem1` allocated but `mem2 == NULL`. Both calls use the *same* `block_size`, so via the public API the two `allocate_block` calls always fail together; the right operand is nevertheless exercised for every `param1` in row 6 and, in isolation, through `allocate_block` row 2 | returns `-1` | `differential_alloc.rs`, section `ERRORS #7` (reproduces the exact `free_block(mem1); free_block(mem2); return -1;` cleanup with `mem1` non-NULL and `mem2` NULL) + `differential_betagamma.rs` residue classes `-9..-6` | [x] |
| 8 | `compute_hash` (lib.c:77 `mb1->data`) | `mb1 == NULL` — the function has **no** null check | UB: reads address `0x0` ⇒ process dies with `SIGSEGV` | `differential_compute_hash.rs`, section `ERRORS #8` (fork-isolated, compares death signal) | [x] |
| 9 | `compute_hash` (lib.c:77 `mb2->data`) | `mb2 == NULL` | UB: `SIGSEGV` | `differential_compute_hash.rs`, section `ERRORS #9` (fork-isolated) | [x] |
| 10 | `create_block` (lib.c:43 `strcpy(block.name, name)`) | `name == NULL` — no null check | UB: `strcpy` reads `0x0` ⇒ `SIGSEGV` | `differential_create_block.rs`, section `ERRORS #10` (fork-isolated) | [x] |
| 11 | `create_block` (lib.c:43) | `strlen(name) > 31` — `strcpy` into `char name[32]` has **no** bounds check. `strlen ∈ 32..35` still lands inside the 40-byte `DataBlock` (`name` at offset 4, `flags` at 36, tail padding 37..39); `flags` is assigned *after* the `strcpy`, so it survives | writes past `name`; for `strlen ≤ 35` the returned struct still has the requested `id`/`flags` and the first 32 name bytes | `differential_create_block.rs`, section `row 5 / ERRORS #11` (lengths 32..35) | [x] |
| 12 | `betagamma` (lib.c:123 `flag_contribution * current->id`, 137 `result += hash`, 141/144 `sum += data[i]`, 147 `sum1 - sum2`) | signed-`int` overflow — `INT_MAX`/`INT_MIN` parameters | UB per ISO C, but the shipped build (`-fPIC`, no `-O`) wraps two's-complement; Rust must reproduce the wrap | `differential_betagamma.rs`, sections `row39/flag-overflow`, `row40/sum-overflow`, `row42/signs`, `row43/random-chunk*` | [x] |
| 13 | `allocate_block` (lib.c:52, boundary) | `count == 0` ⇒ `calloc(0, 4)`; glibc returns a **unique non-NULL** pointer, so the `!mb->data` branch is *not* taken | returns non-`NULL` block with `size == 0`; init loop body never runs | `differential_alloc.rs`, section `row 10 / ERRORS #13` | [x] |
| 14 | `betagamma` (lib.c:126, boundary) | `param1 % 10 == -5` ⇒ `block_size == 0` — one step away from the row-6 failure region | succeeds: `sum1 == sum2 == 0`, `(0-0)/10 == 0`, `+99`, `+255` still added | `differential_betagamma.rs`, section `row 32 / ERRORS #14` | [x] |
| 15 | `betagamma` (lib.c:126, boundary) | `param1 == INT_MIN` ⇒ `INT_MIN % 10 == -8` ⇒ row-6 failure; `param1 == INT_MAX` ⇒ `% 10 == 7` ⇒ `block_size == 12` | `-1` for `INT_MIN`; a normal (wrapped) result for `INT_MAX` | `differential_betagamma.rs`, section `row 41 / ERRORS #15` | [x] |
| 16 | `create_block` — out-of-range value in a narrow parameter across the FFI boundary | `flags` is `uint8_t`; the ABI passes it in a 32-bit register slot. A caller declaring the symbol as `fn(c_int, *const c_char, c_int)` and passing `0x1FF`/`-1`/`0x7FFFFF00` puts non-zero bits above bit 7 into that slot | callee reads only the low 8 bits (`%dl`); result is `flags = value & 0xFF` | `differential_create_block.rs`, section `row 45 / ERRORS #16` | [x] |
| 17 | out-of-range **enum** value across the FFI boundary | The public API declares **no** `enum` type at all (`grep -n enum c_src/**` → no matches). Every parameter is `int` / `size_t` / `uint8_t`, i.e. its whole machine range is a valid input. The equivalent "no valid variant" test is therefore row 16 (narrow type) plus the full-range `i32`/`usize` sweeps | n/a — no enum to violate; full ranges are swept instead | `differential_alloc.rs`, section `ERRORS #17` (`size_t` bit patterns outside the normal domain) + `differential_create_block.rs` section `ERRORS #16` + `differential_betagamma.rs` full-`i32` sweep | [x] |
| 18 | `free_block` used across libraries | a `MemoryBlock` produced by the **C** `allocate_block` released by the **Rust** `free_block` and vice-versa (both must use the *same* platform allocator, otherwise `free` aborts with `free(): invalid pointer`) | clean release, no allocator abort | `differential_alloc.rs`, section `row 44 / ERRORS #18` | [x] |

## Not reachable / deliberately not tested

* `strcpy(temp_name, current->name)` (lib.c:107) — source is always one of the
  three ≤11-byte literals, so it can never overflow `char temp_name[32]`.
  No rejection path.
* `(sum1 - sum2) / 10` (lib.c:147) — the divisor is the literal `10`, so
  division by zero is impossible.
* `create_block` with `strlen(name) >= 36` — writes beyond the returned
  `DataBlock`, i.e. corrupts the caller's stack frame in a way that has no
  defined result in *either* language; excluded on purpose (row 11 covers the
  largest still-inside-the-object length).
* Double-`free_block` of the same pointer, or `free_block` of a non-`malloc`
  pointer — glibc aborts the whole process; not a library-defined rejection.
* `name` array bytes **after** the NUL terminator returned by `create_block`
  are *indeterminate* in C (`DataBlock block;` is uninitialised, lib.c:41), so
  they are excluded from every comparison — see `CONFIGS.md` note N1.

## Divergences found by these tests and fixed in `src/lib.rs`

Both were on error paths that a happy-path test suite cannot see.

| # | row | symptom | root cause | fix |
|---|-----|---------|-----------|-----|
| D1 | 8, 9 | `compute_hash(NULL, p)`: C dies with **SIGSEGV (11)**, Rust died with **SIGABRT (6)** and the message *"null pointer dereference occurred"* | `(*mb1).data` in Rust is instrumented with a null check whenever `debug-assertions` are on, turning C's hardware fault into a Rust panic/abort | read the field with an uninstrumented machine load (`load_data_field`, a libc `memcpy` of `offset_of!(MemoryBlock, data)`), matching C in both the `dev` and `release` profiles |
| D2 | 10 | `create_block(id, NULL, flags)`: C dies with **SIGSEGV (11)**, the hand-written Rust copy loop would abort with **SIGABRT (6)** | same instrumentation, on `*src.add(i)` inside the reimplemented `strcpy` | call libc `strcpy` directly — which is what the C source does (it imports `strcpy@GLIBC_2.2.5`), so the Rust `.so` now imports the identical symbol |

## Completion status

- [x] Every row above has a differential test that constructs the exact
      condition, calls **both** `.so`s through their exported symbols, and
      asserts the *same* error value / sentinel / death signal — not merely
      "both failed".
- [x] `scripts/traceability.sh` mechanically verifies that no row is checked
      off without a matching label in `tests/`.
- [x] All rows pass under both feature combinations and both cargo profiles
      (`scripts/verify_all.sh`).
