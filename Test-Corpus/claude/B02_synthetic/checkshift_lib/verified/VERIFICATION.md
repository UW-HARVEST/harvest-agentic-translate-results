# VERIFICATION.md — C ↔ Rust differential verification report

The C in `c_src/` is the ground truth. Every test loads **both** shared objects
with `libloading` and calls **only** through `dlsym`, so the Rust
`#[no_mangle]`/`extern "C"` export wrappers are exercised exactly as an external
C consumer would exercise them. The Rust crate is never called directly.

Nothing in `c_src/` was modified (only `c_src/build/`, a build output directory,
was created — as the task's own build instructions require).

## Reproduce

```sh
./verify.sh            # Phases A→D: C build, feature combos, symbol parity,
                       # Phase B + C suites in debug AND release, row-coverage gate
./mutation_check.sh    # validates that the suite actually discriminates
./row_coverage.sh      # every CONFIGS.md / ERRORS.md row -> a passing test
./symbol_parity.sh     # nm -D diff
DIFF_ITERS=20000 cargo test --release -- --test-threads=1   # soak
```

## Build-time configuration surface

| source | result |
|---|---|
| `Cargo.toml` `[features]` | **absent** ⇒ the only valid combination is the empty set |
| `c_src/CMakeLists.txt` | no `option()` / `add_definitions`; `lib.c` has no `#ifdef` |

`./check_all_features.sh` derives this mechanically (prints `features found: 0`)
and `cargo check --no-default-features --features ''` passes. Because a single
feature combination could still hide profile-dependent bugs, every phase is run
in **both `debug` and `release`** — which is exactly how the one real bug was
found.

## Results

| gate | result |
|---|---|
| `SYMBOLS.md` — all 10 C exports present in the Rust `.so` | ✅ 0 missing |
| `SYMBOLS.md` — undefined non-libc symbols in the Rust `.so` | ✅ 0 |
| `CONFIGS.md` — Phase B rows passing (randomized) | ✅ 26 / 26 |
| `ERRORS.md` — Phase C rows with a passing error-path test | ✅ 25 / 25 (E1–E17 + G1–G8) |
| tests | ✅ 51 (6 harness + 26 Phase B + 19 Phase C) |
| every feature combination × {debug, release} | ✅ all pass |
| mutation validation of the harness | ✅ 47 / 47 caught, 0 blind spots |
| soak — 20 000 randomized inputs per row | ✅ pass |

Randomized rows use SplitMix64 with the fixed seed `0x5EED_1234_ABCD_F00D`
(reproducible); `DIFF_ITERS` overrides the per-row iteration count.

Comparison is **byte-for-byte on both the return value and captured stdout**.
stdout matters here: the library's `printf` diagnostics *are* its error report,
and gcc rewrites the specifier-free `printf`s into `puts` (hence `puts` in the C
`.so`'s imports) — identical bytes either way. Capture works by flushing all
streams, swapping fd 1 for a temp file under a global lock, and restoring.

## The bug found and fixed

**`checkshift`: the allocation-failure branch was optimized away in release
builds.** (`src/checkshift.rs`, mirroring `lib.c:148-153`)

At `-O2` LLVM recognised the `malloc`/`free` pair in `checkshift` as
non-escaping and deleted the allocation outright (heap-to-stack promotion), so
the release Rust `.so` contained **no call to `malloc` at all** inside
`checkshift`:

```
$ objdump -d --disassemble=checkshift target/release/libcheckshift_lib.so | grep malloc
(nothing)
```

That made `if state.is_null()` dead code. Under allocation failure the C prints
`Error: Failed to allocate memory for state` and returns `-1`, while the
optimized Rust sailed on and returned a normal result — a genuine behavioural
divergence, invisible in debug builds and invisible to any test that does not
actually starve the allocator.

Fix: pass the `malloc` result through `core::hint::black_box` so the allocation
stays observable and the failure branch remains reachable. Verified:

```
$ objdump -d --disassemble=checkshift target/release/libcheckshift_lib.so | grep -E 'malloc|free'
call *0x3be49(%rip)   # <malloc@GLIBC_2.2.5>
call *0x3bc4f(%rip)   # <free@GLIBC_2.2.5>
```

This was caught by `e17_checkshift_malloc_failure`, which does not reason about
the branch — it **executes** it, using the `LD_PRELOAD` fault injector
`tests/fixtures/failmalloc.c`. The shim interposes `malloc` and fails
allocations of exactly `sizeof(ComputeState)` (12 bytes), but only inside an
explicitly armed window around the single `checkshift` call, so `dlopen`, the
Rust runtime and stdio buffers are never disturbed. The test re-execs the test
binary as a child under `LD_PRELOAD`, runs the same scenario against each `.so`,
and diffs the two transcripts.

## Harness self-validation

Passing tests prove nothing if the harness is not discriminating, so this is
checked explicitly rather than assumed:

* `tests/phase_a_harness.rs` asserts the two `.so` files are distinct, that all
  ten symbols resolve to **different** addresses per library (no accidental
  global-symbol interposition, which would make every comparison vacuous), that
  capture returns non-empty transcripts and restores fd 1, and that the
  comparison logic detects a deliberately differing call.
* `./mutation_check.sh` injects 47 deliberate divergences into the Rust source
  one at a time, rebuilds, and asserts the specific test **fails**. 47/47 caught.

Two candidate mutations are deliberately **excluded as provably equivalent**
(a suite "missing" them is correct, not a blind spot):

* `(checksum << 1)` → `checksum.rotate_left(1)` — at most 16 bytes are folded, so
  the accumulator never exceeds `0x007FFFFF` and bit 31 is never shifted out.
* `!values.is_null() && count > 0` → `count > 0 && !values.is_null()` — both
  operands are side-effect free.

### A second, procedural bug the suite caught

`mutation_check.sh` originally restored the pristine source with `cp -a`, which
**preserves mtimes**. The restored files therefore looked *older* than the `.so`
cargo had just built from the mutated source, so cargo skipped the rebuild and
left a mutated `.so` on disk — and `c23`/`c24` promptly failed on the next run
with `Result of Xor:` instead of `Result of XOR:`. Fixed by bumping mtimes after
restore; `verify.sh` now also defensively touches sources so a stale artifact can
never silently pass.

## Verified C behaviours worth calling out

These are places where the C does something the Rust must replicate rather than
"fix", all confirmed by differential test:

* `shift_with_static` — `a << 2` is a **wrapping** shift and `b >> 2` an
  **arithmetic** (sign-extending) shift; rows C7–C10 pin both signs.
* `multiply_with_static` / `add_with_static` — signed overflow **wraps** (the C is
  built at `-O0`, so no UB exploitation); rows C1–C4 include `INT_MIN`/`INT_MAX`.
* `compute_checksum` — folds the **raw object representation**, so the result is
  host-byte-order dependent; `count` is **clamped** to 4, not rejected (E12), and
  `count <= 0` / `NULL` skip the `MAGIC_NUMBER` mix-in entirely and return `0`
  (E8–E11).
* `checkshift`'s final fold converts the signed sum to `unsigned` for the XOR and
  back to `int` on assignment.
* `apply_operation` checks `state` **before** `func`, so a doubly-null call emits
  only the *state* message (E16).
* `execute_operation` forwards `op_name` as a `%s` **argument**, so a `NULL` name
  renders as glibc's `(null)` (E5) and a name containing `%d` is never
  reinterpreted as a format (C15).
* `get_operation`'s lazily-filled `static` table is idempotent and returns the
  addresses of the library's **own exported** symbols — checked against `dlsym`
  (C11) and stable across 1000 interleaved calls (C12).
