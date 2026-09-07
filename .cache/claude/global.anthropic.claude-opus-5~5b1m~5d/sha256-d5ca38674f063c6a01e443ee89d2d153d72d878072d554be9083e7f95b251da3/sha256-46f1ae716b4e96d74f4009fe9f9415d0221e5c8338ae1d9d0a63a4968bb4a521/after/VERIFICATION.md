# Verification of the SPHINCS+ C-to-Rust translation

`c_src/` is the ground truth and was **not modified**. Everything below compares
the Rust `cdylib` against the C `.so` **through `dlopen`/`dlsym` only** -- no Rust
library function is ever called directly, so the `#[no_mangle]` export wrappers
are under test too.

## Phase artifacts

| file | contents |
|---|---|
| `translation/SYMBOLS.md` | all 87 exported symbols, per backend, C vs Rust |
| `translation/ERRORS.md`  | 64-row error-surface table, all ticked |
| `translation/CONFIGS.md` | 74-row configuration-surface table, all ticked |

## Scripts

| script | what it does |
|---|---|
| `build_c.sh <backend> <thash> <secpar>` | CMake-configures + builds the C for one configuration into `cbuild/<cfg>/` |
| `link_cref.sh <backend> <thash> <secpar>` | links `cbuild/<cfg>/libcref.so`: the *unmodified* CMake objects for `sphincs_obj` + `rng.c`, with `lib<backend>.so` as a `DT_NEEDED` dependency. One self-contained handle for the whole C API, loadable with `RTLD_LOCAL` so its symbol names cannot collide with the identically-named Rust ones |
| `symdiff.sh` | dumps + diffs `nm -D` for C and Rust, per backend |
| `run_tests.sh` | Phase B + C differential suite over all 48 feature combinations |
| `run_drivers.sh` | `CONFIGS.md` row 73: C vs Rust `driver` stdout, and both vs `reference_outputs.txt` |

Reproduce from scratch:

```bash
# 1. Build the C for all 48 configurations and link the differential .so
for b in blake haraka sha2 shake; do for t in robust simple; do
  for s in 128s 128f 192s 192f 256s 256f; do
    ./build_c.sh $b $t $s && ./link_cref.sh $b $t $s
done; done; done

# 2. Symbol parity
./symdiff.sh

# 3. Differential tests (per (backend,thash) chunk keeps each run short)
for b in blake haraka sha2 shake; do for t in robust simple; do
  BACKENDS=$b THASHES=$t ./run_tests.sh -- --test-threads=4
done; done

# 4. Driver stdout parity
./run_drivers.sh
```

### OpenSSL note

`c_src/app/src/rng.c` includes `<openssl/{conf,evp,err}.h>`. This machine has no
OpenSSL *headers* on the default include path but does have
`/usr/lib64/libcrypto.so.3`. `build_c.sh` therefore compiles with headers from a
nix OpenSSL 3.6.3 `-dev` output and links against the system `libcrypto.so.3`
via the `osslstub/libcrypto.so` symlink. The C source itself is untouched, and
the resulting C drivers reproduce `reference_outputs.txt` exactly for all 48
configurations -- which is the proof that this C build is the intended one.

## Results

| gate | result |
|---|---|
| `cargo check` for all 48 feature combinations (+ the `shake256` alias) | **0 failures** |
| `nm -D`: symbols the C exports but the Rust does not | **0**, for all 4 backends |
| `nm -D --undefined-only` on the Rust `.so`, non-libc | **0** |
| `CONFIGS.md`: 74 rows x 48 configurations | **all pass** (5064 test cases, 0 failures) |
| `ERRORS.md`: 64 rows x 48 configurations | **all pass** |
| C vs Rust `driver` stdout, 48 configurations | **byte-identical**, and equal to `reference_outputs.txt` |

## Bugs found and fixed in the Rust (never in the C)

1. **`sizeof(spx_ctx)` ABI mismatch (sha2 / `128s` / `128f`).**
   `context.h` declares `state_seeded_512[72]` only under `# if SPX_SHA512`, so
   the C context is **72** bytes for the sha2-128 sets. The Rust struct always
   included the field, giving 144 bytes -- every `spx_ctx` crossing the FFI
   boundary was double the size the C expects, so a C caller's adjacent memory
   would be overwritten. Fixed in `src/context.rs` by sizing the field
   `if SPX_SHA512 == 1 { 72 } else { 0 }`. `tests/hash_core.rs::row17b` now guards
   this with a marker tail (no output comparison could have caught it).

2. **`SPX_treehash` under-sized the `auth_path` slice.** The wrapper built a
   `tree_height * SPX_N` slice, but the C writes at
   `auth_path + heights[offset-1] * SPX_N` where `heights[offset-1]` can reach
   `tree_height` itself -- so the C touches up to `(tree_height + 1) * SPX_N`
   bytes. The Rust panicked exactly where the C writes (reachable with
   `tree_height == 0`, or a `leaf_idx` outside `0..2^tree_height`). Fixed in
   `src/utils.rs`.

3. **Three missing exports** (`AES256_ECB`, `DRBG_ctx`, `cst`) -- see
   `SYMBOLS.md`. All three had real implementations already; only the
   `#[no_mangle]`/`extern "C"` surface was missing. `DRBG_ctx` in particular had
   to be the *same* object the internal `randombytes` code mutates, so that a
   caller writing through the exported global is observed -- `tests/errors.rs`
   rows 40 and 41 drive the DRBG that way.

No `unimplemented!()`, stub or fake was introduced.

## Two C behaviours faithfully replicated (not "fixed")

* **`blake*_update` is chunking-dependent.** The guard
  `if (left && (((datalen >> 3) & 0x3F) >= fill))` masks the byte count with
  `0x3F`, so an update whose length is a multiple of the block size never merges
  with the buffered prefix. Consequence: feeding the same message in different
  chunk sizes gives different digests *in the C*, and `hash_message` is only
  partially sensitive to the message body -- `crypto_sign_verify` therefore
  **accepts** many single-byte message modifications. `ERRORS.md` rows 23 and 31
  record this, and the tests assert C == Rust rather than asserting a rejection.
* **`gen_message_random` overruns `R` on blake with `SPX_N >= 24`.**
  `blake512_final` writes 64 bytes into a buffer the caller sizes at `SPX_N`.
  `tests/hash_core.rs::row25` gives `R` a 64-byte tail and compares all of it, so
  the Rust must reproduce the same over-write byte for byte.

## Test-harness pitfall

`target/release/libsphincs_core_det.so` is one path shared by all 48 feature
sets, so a concurrent `cargo build` for a different `SECPAR` silently swaps the
artifact under test and produces dozens of bogus "divergences". Guarded two ways:
the runners copy each artifact to a per-configuration path and select it with
`SPHINCS_RUST_SO` (each using its own `CARGO_TARGET_DIR`), and
`tests/common/mod.rs::libs()` calls `crypto_sign_bytes()` on both `.so`s at load
time and hard-fails on a mismatch with the compiled-in `SPX_BYTES`.
