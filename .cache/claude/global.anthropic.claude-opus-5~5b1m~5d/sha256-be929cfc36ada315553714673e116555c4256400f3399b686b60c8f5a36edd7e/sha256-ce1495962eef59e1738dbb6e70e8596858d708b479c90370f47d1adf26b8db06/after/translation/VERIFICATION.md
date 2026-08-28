# VERIFICATION.md — how this translation was verified

The library is `stb_ds.h`'s implementation plus two test helpers, compiled as a
single translation unit (`c_src/src/lib.c`, 956 lines) and exported as a shared
object.  The Rust crate (`src/lib.rs`) is verified **only** through its `.so`:
every test loads *both* `.so`s with `libloading` and calls the exported symbols,
so the `#[no_mangle]`/`extern "C"` wrappers are on the tested path.  No Rust
function is ever called directly.

```
c_src/build/libharvest-work-uAHqBm.so   <-- oracle
target/{debug,release}/libintput_lib.so <-- under test
```

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md`  | all 16 exported symbols, C↔Rust, plus the `static` symbols that must stay private and the assertion-string parity table |
| `ERRORS.md`   | 54-row error/rejection surface table, one row per distinct way `lib.c` rejects or faults on input, each with its passing differential test |
| `CONFIGS.md`  | 65-row configuration surface table (valid inputs), derived from the option/shape axes A1-A10 that `lib.c` actually branches on |
| `verify.sh`   | Phase D driver: every feature combination × both cargo profiles × 7 differently-optimised C builds |
| `list_features.py` | helper used by `verify.sh` to enumerate `[features]` from `Cargo.toml` |

## Test harness (`tests/common/mod.rs`)

`lib.c` exports only the *low-level* `stbds_*` functions; the ergonomic API
(`hmput`, `shget`, `arrput`, `hmdel`, `sh_new_arena`, …) lives in macros that the
`.so` cannot export.  The harness therefore **re-implements those macros on top
of the exported functions**, so the tests drive the library exactly as a real
consumer's expanded macros would:

* `Map` — reproduces `stbds_hmput`/`hmputs`/`shput`/`shputs`/`hmgeti`/`hmgeti_ts`/
  `hmdel`/`shdel`/`hmdefaults`/`hmput_default`/`sh_new_arena`/`sh_new_strdup`/
  `hmfree`, including the hash-side ↔ raw-side pointer bias (`t` vs `t-1`), the
  `t[-1]` default element, `stbds_temp(t-1)` and `stbds_temp_key(t-1)`.
* `ArrPair` — reproduces `arrgrow`/`arrput`/`arrsetlen`/`arrfree`.
* `Pair` / `MapSnap` — after **every** operation both libraries are snapshotted
  and compared structurally: header `length`/`capacity`/`temp`, `hash_table`
  null-ness, every element's bytes, and the *whole* `stbds_hash_index`
  (`slot_count`, `used_count`, both thresholds, `tombstone_count`,
  `tombstone_count_threshold`, `seed`, `slot_count_log2`, the arena's
  `remaining`/`block`/`mode`/`storage`-null-ness, **and every bucket's
  `hash[]` and `index[]` array**).  Because the two libraries keep independent
  `stbds_hash_seed` globals that advance identically under identical call
  sequences, the raw hash values in the buckets are directly comparable — that
  makes the comparison extremely tight.
* Pointer-valued fields (`char *` keys, `storage`, `temp_key`) are compared by
  *content*, never by address, since the two libraries own separate heaps.
* Every test body runs under a process-wide mutex and re-seeds both libraries
  with `stbds_rand_seed`, because `stbds_hash_seed` and `strkey`'s
  `static char buffer[256]` are shared mutable globals of each `.so`.
* Randomised inputs come from a fixed-seed SplitMix64, so every run is
  reproducible.

## Results

| phase | scope | result |
|-------|-------|--------|
| A | `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md` produced from the C source | done |
| A | `nm -D` symbol diff C vs Rust | **empty** (16/16) — no symbol needed adding, no C module was missing |
| B | all **65** `CONFIGS.md` rows, randomised, deep snapshot compare per op | **all pass** |
| C | all **54** `ERRORS.md` rows | **all pass** |
| D | 3 feature combinations × 2 cargo profiles | **all pass** |
| D | 7 C build variants (`-O0`,`-O1`,`-O2`,`-O3`,`-Os`,`-O2 -fstrict-*`, `-O0 -g -fno-strict-aliasing`) × 2 Rust profiles | **all pass** |

123 tests, 9 test binaries.  `bash verify.sh` reproduces everything.

## Divergences found and fixed (in the Rust — never the C)

1. **Profile-dependent fault mode on NULL pointers.**  `lib.c` never null-checks
   a pointer parameter, so `stbds_hash_string(NULL, s)`,
   `stbds_hash_bytes(NULL, 8, s)`, `stbds_stralloc(NULL, s)`,
   `stbds_stralloc(a, NULL)`, `stbds_strreset(NULL)`,
   `stbds_hmget_key_ts(…, temp=NULL, …)`, a NULL `key`, and a failing `realloc`
   inside `stbds_arrgrowf` all die with **SIGSEGV**.  The Rust translation used
   plain `*p`, which trips rustc's `"null pointer dereference occurred"` UB check
   and dies with **SIGABRT** whenever debug assertions are on — so the release
   `.so` matched the C but the debug `.so` did not.  Those specific
   FFI-boundary accesses now use `ptr::read_volatile` / `ptr::write_volatile`,
   which emit the same machine access without the inserted check.  Verified:
   `signal == SIGSEGV(11)` for both libraries in both profiles
   (`err_e28`, `err_e3`, `err_e47`–`err_e54`).

## C behaviours deliberately reproduced (not "fixed")

* `stbds_arrgrowf(NULL, es, 0, 0)` returns **NULL** — the `min_cap <= arrcap`
  early return fires before the floor-to-4 logic.
* `stbds_make_hash_index` never initialises `temp_key`; it is genuine
  uninitialised heap data until a string-mode `hmput_key` writes it, and every
  grow/shrink/rebuild installs a fresh uninitialised `stbds_hash_index`.  The
  harness therefore only compares `temp_key` where the C provably wrote it
  (`Pair::assert_temp_key`, rows B43/B43b/E44).
* Only the **forward** half of `hmput_key`'s probe loop updates `temp_key`
  (lib.c:733); the wrap half (lib.c:749) returns without touching it, so
  `stbds_shputs` can legitimately store a *stale* key pointer.  Reproduced
  exactly; the harness does not treat it as an error.
* `intput(9)` and `intput(11)` abort: `hmput(intmap,num,7)` is later overwritten,
  so `assert(hmget(intmap,num) == 7)` at lib.c:955 is false.  Both libraries
  print a byte-identical glibc diagnostic (same `__FILE__` path via `build.rs`,
  same line, same `__PRETTY_FUNCTION__`, same expression text) and raise SIGABRT.
* `mode == 2` (`STBDS_HM_PTR_TO_STRING`, which this TU never `#define`s) makes
  `hmdel_key` hash the *address* of the key field as a C string, so
  `assert(slot >= 0)` at lib.c:846 fires — identically in both libraries.
* `keysize == 0` collapses the map to a single entry (`memcmp(_,_,0) == 0`).
* `stbds_shmode_func` truncates its `int` mode to `unsigned char`
  (`256 → 0`, `-1 → 255`, `INT_MAX → 255`, `INT_MIN → 0`).
* The string arena's `block` field saturates at **22**, not 23
  (`512 << 11 == 1<<20 == BLOCKSIZE_MAX`, and the guard is a strict `<`).
* `assert(len <= a->remaining)` at lib.c:913 is **provably unreachable** (see
  `ERRORS.md` E31); the nearest reachable out-of-range input is a `block` field
  above 109, which wraps `512u << (block>>1)` to 0.  Reproduced with
  `wrapping_shl`, tested for `block ∈ 110..=153 ∪ 238..=255`.
