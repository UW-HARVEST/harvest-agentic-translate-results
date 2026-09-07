# Verification report

The C in `../c_src` is the ground truth. This crate is verified against it by
loading **both** shared libraries with `libloading` and comparing their results
through the FFI boundary. No Rust function is ever called directly — every call
goes through `dlopen`/`dlsym` on `libarr_del_lib.so`, so the
`#[unsafe(no_mangle)] extern "C"` export wrappers are on the tested path too.

## Run it

```sh
# one command does everything (builds the C .so, both Rust profiles, symbol
# parity, and the whole suite for every feature combination x profile)
./verify.sh
```

Or manually:

```sh
cd ../c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && cargo build --release && cargo test
```

## Artifacts

| file | what it is |
|------|------------|
| `SYMBOLS.md` | every `nm -D` symbol of the C `.so` vs the Rust `.so` (Phase A/D) |
| `ERRORS.md` | the error-surface table: every rejection/sentinel/assert in the C, one row each (Phase C) |
| `CONFIGS.md` | the configuration-surface table: every option x input-shape combination the C branches on (Phase B) |
| `verify.sh` | the driver that re-checks all of the above |
| `VERIFICATION.md` | this report |
| `tests/common/mod.rs` | the `dlopen` harness, the `#[repr(C)]` struct mirrors and the fixed-seed RNG |
| `tests/common/map.rs` | the low-level map driver (the `stbds_hmput`/`shput`/`hmgeti`/`hmdel` macro bodies from `lib.c`, reimplemented on top of the exported functions) |

## Results

* **Symbols** — 16 exported, `diff` of `nm -D` is **empty** in both profiles and
  both feature configurations. No undefined non-libc symbols.
* **Phase B** — all 46 `CONFIGS.md` rows pass, each over many randomized inputs
  from a fixed-seed xoshiro256\*\* generator.
* **Phase C** — 42 of 44 `ERRORS.md` rows have a passing differential test; the
  remaining two are genuine C undefined behaviour (unchecked `realloc`,
  `free((char*)0 - 32)`) and are documented in `ERRORS.md` rather than executed.
* **Asserts** — the C's 7 live `STBDS_ASSERT`s are reproduced; abort parity is
  checked in subprocesses (`err_asserts_abort_parity`).
* **Totals** — 87 test functions, green against the release **and** the debug
  Rust `.so` (the debug profile has overflow checks on, so it independently
  proves no arithmetic in the translation panics where the C wraps).
* **Binary** — `c_src/CMakeLists.txt` has no `add_executable` and `Cargo.toml`
  has no `[[bin]]`, so there is no driver stdout to diff.
* **Features** — `Cargo.toml` has no `[features]` table; `__default__` and
  `--no-default-features` are the same build and both are exercised.

## What is compared, not just "did it return"

For every map operation the tests compare, byte for byte:

* the whole `stbds_array_header` (`length`, `capacity`, `temp`, whether
  `hash_table` is set);
* all `length * elemsize` element bytes (for string maps: each key's *contents*,
  since the pointers legitimately differ, plus the value bytes);
* the entire `stbds_hash_index` — `slot_count`, `used_count`, all three
  thresholds, `tombstone_count`, `seed`, `slot_count_log2`, the arena
  bookkeeping, **and every bucket's `hash[8]` / `index[8]` arrays**;
* the returned index / sentinel of the operation itself.

`cfg_46_struct_layout` pins the sizes, offsets and the 64-byte `storage`
alignment; `cfg_46b_allocation_sufficiency` proves every block either library
allocates is large enough for what it writes into it.

## Optimisation-independence of the C's UB reproduction

`stbds_siphash_bytes` relies on C signed-integer behaviour that is technically
undefined (`d[3] << 24` on an `int` when `d[3] >= 0x80`), and the Rust reproduces
gcc's actual result. To confirm the Rust is not merely matching one particular
compilation, `lib.c` was additionally compiled at `-O2` and at
`-O3 -march=native` (into a scratch directory — `c_src/` is untouched) and all
five builds were compared with `ctypes`:

```
builds: C(-O0, the reference), C(-O2), C(-O3 -march=native),
        Rust(release), Rust(debug)
42008 comparisons of stbds_hash_bytes / stbds_hash_string / strkey
mismatches: 0
```

So the sign-extension, the `switch` fall-through tail, the rotate macros and the
`%d` formatting agree across every optimisation level.

## Deliberate non-goals (and why)

Four C behaviours are recorded but not executed, because they are undefined
behaviour or a live `assert` that would abort the harness identically for both
libraries — running them would prove nothing and destroy the test process:

1. `stbds_arrgrowf` never checks `realloc` (`lib.c:297`), including the
   `elemsize * min_cap + sizeof(header)` **size overflow** (e.g. `elemsize = 4`,
   `min_cap = SIZE_MAX` makes the C write a 32-byte header into a 28-byte
   allocation).
2. `stbds_arrfreef(NULL)` calls `free((char *) NULL - 32)` (`lib.c:314`).
3. `stbds_hmdel_key` with `mode >= 2` doing an **interior** delete: `lib.c:842`
   tests `mode == STBDS_HM_STRING` (exactly 1) and so hands the raw bytes of the
   stored `char *` to `stbds_hm_find_slot`, which hashes them as a string, finds
   nothing, and trips the live `STBDS_ASSERT(slot >= 0)` at `lib.c:846`.
   `err_28_hmdel_mode_two_string_table` and `err_40_mode_out_of_range` therefore
   confine `mode >= 2` deletes to the last element, where that block is skipped.

4. A string-keyed table whose `string.mode` is `STBDS_SH_NONE` — only reachable
   by explicitly calling `stbds_shmode_func(elemsize, STBDS_SH_NONE)` and then
   doing string puts. The `switch` `default:` arm at `lib.c:789` memcpy's the
   first `keysize` *characters* of the key into the element, and the next
   `stbds_is_key_equal` with `mode >= 1` (`lib.c:561`) does
   `strcmp(key, *(char **) elem)`, dereferencing those characters as a pointer.

Two fields are read only where they are defined, because the C leaves them
uninitialised:

* `stbds_hash_index::temp_key` — `stbds_make_hash_index` never initialises it,
  so it is uninitialised malloc memory after any grow / shrink / rebuild. It is
  compared only right after an insert in a string mode
  (`cfg_18b_temp_key_after_insert`) and on a table pinned below its grow
  threshold (`cfg_18c_temp_key_wraparound_quirk`, which also proves the
  wrap-around probe loop's *missing* `temp_key` store is reproduced).
* the value part of an element after `stbds_hmput_key` — the function only
  `memcpy`s `keysize` bytes, so the rest is uninitialised until the caller's
  macro writes it.

## Static audit

Independently of the differential tests, every function was read side by side
against the C. All 23 functions in `lib.c` (the 16 exported ones plus the
`static` helpers `stbds_probe_position`, `stbds_log2`, `stbds_make_hash_index`,
`stbds_siphash_bytes`, `stbds_is_key_equal`, `stbds_hm_find_slot`,
`stbds_strdup`) and every macro they expand are semantically equivalent. Points
that were checked explicitly and are correct:

* `stbds_hmput_key` uses `raw_a` (the *hash* pointer) for `stbds_is_key_equal`
  and `a` (the *array* pointer) for `stbds_temp` / the key store, and re-derives
  `raw_a` unconditionally after the conditional `arrgrowf` (`lib.c:773-776`).
* the wrap-around probe loop's **missing** `stbds_temp_key` store
  (`lib.c:745-759` vs `732-733`) is reproduced.
* `stbds_hmdel_key` applies `keyoffset` to the re-find key but **not** to the
  strdup `free` or the `memmove` (`lib.c:837`, `840`, `843/845`).
* `stbds_arrdeln`'s memmove count uses the length *before* the decrement, and
  `stbds_arrdelswap` reads `arrlast` before the store.
* `stbds_hash_string`'s `hash ^= hash ^ ROTATE_RIGHT(...)` (which is just the
  rotate) is kept literal.
* every `int`-promotion / sign-extension site in `stbds_siphash_bytes`,
  including that tail cases 7/6/5 cast to `size_t` *before* shifting while
  case 4 does not.
* all struct sizes, field offsets and the 64-byte `STBDS_ALIGN_FWD` of
  `storage`.

## The two divergences that were found and fixed

### 1. All 7 `STBDS_ASSERT`s were dropped

`src/lib.rs` had translated **all 7 `STBDS_ASSERT`s away** (four silently, three
left as comments). The C is built without `NDEBUG` — its `.so` links
`__assert_fail` — so those asserts are live and `abort()`. One of them,
`lib.c:846 STBDS_ASSERT(slot >= 0)`, is reachable through the public API:

```
interior stbds_hmdel_key with mode == 2 on a STBDS_SH_DEFAULT string map
  C (reference)         -> SIGABRT (lib.c:846: Assertion `slot >= 0' failed.)
  Rust (as translated)  -> exit 0   <-- DIVERGENCE, then a wild pointer write
  Rust (after the fix)  -> SIGABRT (stbds_assert: lib.c:846: ...)
```

(`stbds_hmdel_key` tests `mode == STBDS_HM_STRING`, i.e. exactly 1, at line 842,
while `stbds_hm_find_slot` tests `mode >= 1`. With `mode > 1` the re-find is
handed the raw bytes of the stored `char *`, hashes them as a string, and misses,
so `slot == -1` and the C trips the assert. The Rust then computed
`storage + ((-1) >> 3)`.)

**Fix:** all 7 asserts are now reproduced with `assert!` (not `debug_assert!`, so
they are live in every profile just as in the C build); a panic under an
`extern "C"` fn aborts, so the two libraries fail with the same signal.
`err_asserts_abort_parity` runs 7 scenarios (5 aborting, 2 clean) against each
library in a **subprocess** and asserts the termination statuses are identical.
Validated with a negative control: a Rust `.so` rebuilt with the assert macro
neutered makes that test fail, so it genuinely guards the behaviour.

### 2. `stbds_stralloc`'s shift count (`lib.c:888`)

`a->block` is an `unsigned char` and `stbds_stralloc` is exported, so the C's
shift count `a->block >> 1` can reach 127. gcc emits `shlq %cl`, masking it to 6
bits. The Rust's plain `<<` is fine in the release profile
(`overflow-checks = false`) but **panics/aborts in the dev profile** for any count
>= 64:

```
a->block = 128   C -> "hello", block=129, remaining=506 (exit 0)
                 Rust release -> identical
                 Rust debug   -> SIGABRT (attempt to shift left with overflow)
a->block = 255   C -> "hello", block=0, remaining=0 (exit 0)
                 Rust debug   -> SIGABRT
```

**Fix:** `wrapping_shl`, whose count is masked with `& 63` on a 64-bit target,
exactly like `shlq %cl`. The same reasoning applies to every other plain
`+`/`-` on a value a caller can control, so nine further sites were switched to
`wrapping_*`: the `sizeof(*sb)-8+len` allocations in `stbds_stralloc`,
`header->length -= 1` in `stbds_hmdel_key`, the three `header->length += 1`,
`i+1`/`i-1` in `stbds_hmput_key`'s `found_empty_slot`, `arrlen-1-1` in
`stbds_hmdel_key`, and `idx+1` in `arrpush_int`. Loop counters,
`rotate_left`/`rotate_right` (always non-zero constant counts < 64) and the
constant `len << 56` were deliberately left alone.

`err_34b_stralloc_block_full_range` pins all 29 boundary values of `a->block`,
each in a subprocess. Negative control: restoring the plain `<<` makes it fail at
`block = 128`.

For `a->block` in {63, 64, 100, 200} the C's **unchecked** `realloc` (non-goal 1
below) returns NULL and the C stores through it — C and release-Rust both
SIGSEGV, while the dev profile's null-pointer check traps the same UB one step
earlier and aborts. The test demands an exact match whenever the C returns
normally, and only "both died" on those row-4 UB inputs.

Apart from these two, `src/lib.rs` needed no changes. Every other fix during
verification was to the *test harness*, where early versions read uninitialised
C memory (`temp_key`, element padding), compared `malloc_usable_size` (not a
function of the requested size), assumed `realloc` always moves the block, or
exercised configurations on which the C itself faults.

## Global state

Both `.so`s carry a process-global `stbds_hash_seed` (advanced by an LCG on every
new hash index) and a global 256-byte `buffer` for `strkey`. The harness
serialises tests with a mutex and re-seeds both libraries to the same value
before each test, so table seeds — and therefore every hash, probe position and
bucket layout — match. `cfg_07_44_rand_seed_sequence` checks the seed *sequence*
itself, and `cfg_33b_soak_multi_seed` re-runs the full randomized pipeline under
24 different global seeds so the bucket layouts differ every time.
