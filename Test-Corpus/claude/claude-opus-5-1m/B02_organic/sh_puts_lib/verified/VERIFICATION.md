# VERIFICATION.md — differential verification of the C→Rust translation

The library under test is a trimmed `stb_ds.h` (dynamic array + open-addressing
string/binary hash map + string arena) plus the `strkey` / `sh_puts` demo
helpers.  `c_src/src/lib.c` is the ground truth; `src/lib.rs` must match it
byte-for-byte.

## Artefacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | Phase A: `nm -D` surface, C vs Rust, and the build-configuration enumeration |
| `ERRORS.md`  | Phase C: the error/rejection surface table (42 rows), one row per distinct C rejection site |
| `CONFIGS.md` | Phase B: the configuration surface table (53 rows), one row per meaningful option × input-shape combination |
| `verify.sh`  | mechanised Phase A→D driver (build, `cargo check`, symbol diff, tests × features × profiles) |
| `tests/`     | the differential test suite (95 tests) |

## How the tests work

`tests/common/mod.rs` `dlopen`s **both** shared libraries with `libloading`:

* `c_src/build/libtranslated_rust.so` (gcc, `-O0`)
* `target/{release,debug}/libsh_puts_lib.so`

and resolves all 16 exported symbols from each handle.  **No Rust function is
ever called directly** — every call goes through the `#[no_mangle]`/`extern "C"`
wrapper, exactly as an external C consumer would, so the export wrappers are
themselves part of what is verified.

Because `stb_ds` is macro-driven, the harness spells the macros out by hand
(`stbds_hmput`, `stbds_shput`, `stbds_shputs`, `stbds_hmgeti`, `stbds_hmgeti_ts`,
`stbds_hmdel`, `stbds_hmdefault`, `stbds_arrput`, `stbds_sh_new_arena`, …) and
drives the *low-level* entry points directly rather than only the one
header-declared convenience function.

After every operation `Dual::check` deep-compares the two maps:

* array header (`length`, `capacity`, `hash_table != NULL`, `temp`),
* the whole `stbds_hash_index` (`slot_count`, `used_count`, both thresholds,
  `tombstone_count`, `seed`, `slot_count_log2`, the embedded arena's
  `remaining`/`block`/`mode`),
* **every bucket's `hash[8]` and `index[8]` array**,
* every raw element's bytes (or, for string modes, the NUL-terminated key
  *content*, since key pointers are library-owned and not comparable),
* and that `table->storage` is 64-byte aligned in both.

Inputs are property-style randomised from a fixed-seed splitmix64 RNG, so
failures are reproducible.  `stbds_rand_seed` is called on both libraries before
every workload to keep their global `stbds_hash_seed` LCGs in lock-step, and a
process-wide mutex serialises tests that touch that global state, the `strkey`
static buffer, or `stdout`.

Crash/abort behaviour is compared by `fork()`ing a child per library and
comparing the terminating signal **and stderr** (`common::in_child`).
`sh_puts` is compared by `dup2`-redirecting fd 1 to a file
(`common::capture_stdout`).

## Findings and fixes

Three genuine defects were found and fixed in the Rust side (the C was never
touched):

1. **`stbds_stralloc` shift-count overflow.** `512u << (a->block >> 1)` was
   translated as `<<`.  `a->block` is an `unsigned char`, so the shift count can
   reach 127; gcc emits `shl`, which masks the count with `& 63`, whereas Rust's
   `<<` is an arithmetic-overflow error (panic in `debug`).  Fixed with
   `wrapping_shl`, which reproduces the masking exactly.  `arena.rs::
   stralloc_block_field_matrix` and `errors.rs::e26_stralloc_block_shift` now
   sweep `a->block` over 0..=255 and confirm both libraries collapse `blocksize`
   to 0 (and wrap `a->block` to 0) identically.
2. **`STBDS_ROTATE_LEFT/RIGHT` and the sip-hash block counter** used plain
   `<<`/`>>`/`+`, which panic on overflow in a `debug` build even though the C
   is well-defined (or masked) there.  Converted to `wrapping_shl`/
   `wrapping_shr`/`wrapping_add`.
3. **`STBDS_ASSERT` message text.** The C's `assert` embeds `__FILE__`, which
   CMake supplies as an absolute path; the translation hard-coded
   `"src/lib.c"`.  A `build.rs` now canonicalises `c_src/src/lib.c` and passes
   it through `env!()`.  `errors.rs::e23_hmdel_keyoffset_abort` reaches a real
   assertion (`stbds_hmdel_key` with `keyoffset != 0`) in both libraries and
   asserts their stderr is byte-identical.

Two *test* defects were also found and fixed:

* comparing `realloc`'s pointer-identity decision across two independently
  allocated heaps (only valid on `stbds_arrgrowf`'s `min_cap <= arrcap`
  early-return path), and
* dereferencing a NULL map pointer in an invariant check.

## Documented non-divergences

* **Rust `debug` profile turns null-dereference `SIGSEGV` into `SIGABRT`** —
  rustc's `-C debug-assertions` UB checks fire before the load.  The **release**
  cdylib (the shipped artefact) segfaults exactly like the C.  See the note at
  the end of `ERRORS.md`.
* **`stbds_hmdel_key` with `mode >= 2` and a `memmove` fix-up hashes raw pointer
  bytes as text**, so its outcome depends on heap addresses and is not
  reproducible for the C against itself.  `enums.rs::hmdel_mode_two` makes it
  deterministic by using `STBDS_SH_DEFAULT` (caller-owned, hence identical, key
  pointers) and shows both libraries abort with the same message.

## Completion gate

```
$ ./verify.sh
   [ok]   C library has exactly one build configuration
   [ok]   cargo check (default) / --no-default-features / --all-features
   [ok]   cargo build --release and debug for every combination
   [ok]   target/release/libsh_puts_lib.so exports all 16 C symbols (0 missing)
   [ok]   target/debug/libsh_puts_lib.so   exports all 16 C symbols (0 missing)
   [ok]   0 unresolved non-libc symbols
   [ok]   95 tests passed x 3 feature combinations x 2 profiles
ALL VERIFICATION STEPS PASSED
```

- [x] `SYMBOLS.md`: `nm -D` diff is **empty**; 0 missing/undefined non-libc symbols.
- [x] Phase B: every one of the 53 `CONFIGS.md` rows passes across randomised inputs.
- [x] Phase C: every one of the 42 `ERRORS.md` rows has a passing differential test.
- [x] All of the above hold under **every** feature combination (the crate has no
      `[features]`, so default == `--no-default-features` == `--all-features`)
      **and** under both the `release` and `debug` Rust profiles.
