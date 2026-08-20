# VERIFICATION.md — completion record

Differential verification of the Rust translation in `src/` against the C ground
truth in `c_src/`. Everything is driven through the two shared libraries'
exported C symbols (`libloading` + `dlopen`), never by calling a Rust function
directly, so the `#[no_mangle]` export wrappers are part of what is measured.

Reproduce with `./run_all.sh` (or `./run_all.sh quick` to skip the fork-heavy
suites).

## Artifacts

| file | phase | content |
|------|-------|---------|
| `SYMBOLS.md`  | A | every `nm -D` symbol of the C `.so` and its Rust counterpart |
| `ERRORS.md`   | A | error-surface table: every way the C rejects/mishandles input |
| `CONFIGS.md`  | A | configuration-surface table: every valid option × input-shape combination |
| `check_symbols.sh` | D | regenerates and re-checks the symbol diff |
| `run_all.sh`  | A–D | every feature combination × both profiles × every suite |

## Test suites

| file | tests | what it gates |
|------|-------|---------------|
| `tests/common/mod.rs` | — | harness: dual `dlopen`, fd-1 capture, `fork`+`waitpid` disposition compare (2 s `alarm` so "spins forever" is a comparable outcome; `SIGSEGV`/`SIGBUS`/`SIGABRT` reset to `SIG_DFL` in the child so the *test binary's* own Rust runtime cannot reclassify one library's fault and not the other's), guard-page buffers, SplitMix64 PRNG, independent reference model |
| `tests/phase_b_configs.rs` | 25 | one per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 25 | one per `ERRORS.md` row 1–25 |
| `tests/negative_len_analysis.rs` | 4 | `ERRORS.md` rows 14–16 (negative `len`) |
| `tests/oob_read_analysis.rs` | 4 | `ERRORS.md` rows 26–27 (`len` past the caller's buffer) |
| `tests/phase_d_symbols.rs` | 5 | symbol parity, `inner` staying private, callability |

## Build-time configurations

`Cargo.toml` has **no `[features]` section** and `c_src/CMakeLists.txt` has no
`option()`/`#ifdef`/build-type branches, so the powerset of valid feature
combinations has exactly one element (the empty set). To make sure "one
configuration" is not an excuse for one code path, the suites are additionally
run against **both** Rust profiles:

| configuration | `cargo check` | Phase B | Phase C | Phase D |
|---------------|---------------|---------|---------|---------|
| `--no-default-features`, dev profile `.so` | clean | pass | pass | pass |
| default features, dev profile `.so`       | clean | pass | pass | pass |
| `--no-default-features`, release profile `.so` | clean | pass | pass | pass |
| default features, release profile `.so`   | clean | pass | pass | pass |

## Divergences found and fixed (all fixes in the Rust, none in `c_src/`)

| # | found by | symptom | root cause | fix |
|---|----------|---------|------------|-----|
| 1 | `err_05..08` (NULL pointer rows) | C died with `SIGSEGV`, Rust with `SIGABRT` + `"null pointer dereference occurred"` | the unoptimised profiles compiled Rust's debug-only UB checks *into the shared library*; the C reference has no such instrumentation | `debug-assertions = false` / `overflow-checks = false` for `dev`/`test`/`release` in `Cargo.toml`, with the reason documented there. (Signed-overflow wrapping was already explicit via `wrapping_mul`/`wrapping_add`, so the arithmetic is unaffected.) |
| 2 | `err_17_driver_len_int_max` | `driver(data, INT_MAX)`: C `SIGSEGV`, Rust `SIGABRT` | the C VLA `int out[len]` is a bare `rsp -= size` with no probe, so an oversized length faults on first use; the Rust used a heap `Vec`, which either succeeded or aborted through `handle_alloc_error` | `src/driver.rs` now computes the VLA address with gcc's exact formula (`(len*4 + 15)/16*16`, then `align4`) and `read_volatile`s its first byte, so an unbackable VLA faults with the same signal at the same address, before anything is printed |
| 3 | `err_24_wild_int_len_sweep` at `len = -512` | C `SIGSEGV`, Rust `exited(0)` | for a negative `len` the C's `memcpy` destination is `rsp + round16(4·\|len\|)` — inside the caller's frame — not a local; writing elsewhere clobbers different memory | the negative branch now performs the copy at that very address (`vla_base`) |
| 4 | `err_24_wild_int_len_sweep` at `len = 8192` over a 4096-element buffer | outputs diverged 3 elements past the end of the buffer | the C VLA lives on the **stack**, so it leaves the malloc heap untouched; the Rust's `Vec` came from the same heap as the caller's `data` and perturbed the bytes immediately after it | the VLA stand-in is now a fresh anonymous `mmap` (`src/driver.rs`), so the malloc heap is untouched exactly as in the C |
| 5 | `err_20_driver_int_max_overflow` | — | the *test's* expected string was wrong: `INT_MAX*INT_MAX + INT_MAX` wraps to `INT_MIN`, so the C prints `-2147483648` | test expectation and `ERRORS.md` row 20 corrected against the C `.so`; the Rust already matched |

## Two conditions with no single C answer (documented, not papered over)

Both were established by *measuring the C shared library against itself*, not by
assumption, and for both, everything the C source does specify is still compared
byte-for-byte:

1. **`driver` with a negative `len`** (`ERRORS.md` rows 14–16). The VLA moves
   `%rsp` upwards into the caller's frame, so the outcome is decided by the
   caller. `d_neg_01` calls the same C `.so` with the same arguments from four
   call sites differing only in caller stack usage and gets both `exited(0)` and
   `SIGSEGV` for `len = -512`.
   Still gated: identical (empty) stdout from both libraries for every negative
   length; the Rust faults deterministically (same `SIGSEGV` from all four caller
   frames, repeated runs) instead of silently "succeeding"; `fma_array` — which
   has no VLA — is compared exactly.
2. **`len` past the end of the caller's buffer** (`ERRORS.md` rows 26–27). The
   unvalidated `memcpy` copies and prints unspecified process memory.
   `d_oob_01` gets **3 distinct outputs** from the same C `.so` by varying only an
   unrelated earlier `malloc`; `d_oob_04` flips the same C `.so` between
   `SIGSEGV` and `exited(0)` for the same 4096 input values and the same
   `len = 5120`, changing only whether a guard page or readable memory follows the
   buffer.
   Still gated: the in-bounds prefix of whatever each library prints must equal
   the independent reference model and must be identical between the two, and a
   library that survives must have printed exactly `len` lines.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` exports exactly `driver` and
      `fma_array`; the Rust `.so` exports both, under both profiles, and nothing
      extra. `inner` is `static` in the C and stays private in the Rust. The
      symbol diff is **empty** in both directions. 0 missing/undefined non-libc
      symbols (`dlopen(RTLD_NOW)` succeeds; `ldd -r` reports nothing). No C
      translation unit is untranslated — `CMakeLists.txt` compiles one file,
      `src/driver.rs` covers it.
- [x] **Phase B** — all 25 `CONFIGS.md` rows pass across their randomized inputs
      (fixed-seed SplitMix64), including the low-level `fma_array` entry point
      driven directly with all 9 aliasing/overlap layouts, not just the `driver`
      convenience wrapper.
- [x] **Phase C** — all 27 `ERRORS.md` rows have a passing differential test that
      constructs the exact condition and compares the same error/sentinel (same
      terminating signal number, or the same normal return plus byte-identical
      output and buffer). Includes NULL in all 16 pointer subsets, zero and
      oversized lengths, one-past-the-end with guard pages, and the full hostile
      `int` sweep through `len` — the API has no `enum` parameters, so an
      out-of-range `int` in `len` is the corresponding FFI-boundary input class.
- [x] **All of the above under every feature combination** — the single valid
      combination, spelled both ways, against both the dev-profile and the
      release-profile shared library.
