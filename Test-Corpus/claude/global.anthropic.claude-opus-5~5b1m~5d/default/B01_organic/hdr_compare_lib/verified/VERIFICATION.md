# VERIFICATION.md — differential verification of the C→Rust translation

The library under test is `c_src/src/lib.c`: one exported function, `hdr_compare`, plus one
`static` helper, `hdr_valid`. It is an MPEG audio frame-header comparison routine.

Every assertion in this suite compares the **C `.so`** against the **Rust `.so`**, both loaded
with `libloading` and called through their exported `hdr_compare` symbol. The Rust crate is
never linked directly, so the `#[no_mangle] extern "C"` wrapper is itself under test.

## How to reproduce

```
# C shared object
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# Everything: both profiles x every cargo feature combination, plus nm -D parity
cd translation && ./verify.sh

# A single profile
cd translation && cargo build --release && cargo test --release

# Full-size rows in the debug profile too (debug strides the heavy sweeps by default)
HDR_STRIDE=1 cargo test

# The total 2^40 exhaustive equivalence sweep (~12 min)
HDR_SWEEP_2P40=1 cargo test --release --test exhaustive c36 -- --nocapture
```

## Artifacts

| file | phase | contents |
|---|---|---|
| `SYMBOLS.md` | A | `nm -D` surface of both `.so`s; symbol diff |
| `ERRORS.md` | A/C | error-surface table: 25 rows, one per distinct rejection in the C |
| `CONFIGS.md` | A/B | configuration-surface table: 36 rows over the axes the C branches on |
| `tests/common/mod.rs` | — | harness: dual `dlopen`, guarded pages, fork-based crash probe, seeded RNG |
| `tests/symbols.rs` | D | symbol parity, no unresolved non-libc symbols, `hdr_valid` stays internal |
| `tests/valid_paths.rs` | B | rows C1–C19, C21, C23–C33 |
| `tests/exhaustive.rs` | B | rows C20, C22, C34–C36 (the exhaustive sweeps) |
| `tests/error_paths.rs` | C | rows E1–E23 + G1 |
| `verify.sh` | D | full matrix driver |

## Results

66 tests, all passing in every configuration:

| configuration | symbols | valid_paths | exhaustive | error_paths | total |
|---|---|---|---|---|---|
| `debug`, default features | 5 ✅ | 31 ✅ | 5 ✅ | 25 ✅ | 66 ✅ |
| `debug`, `--no-default-features` | 5 ✅ | 31 ✅ | 5 ✅ | 25 ✅ | 66 ✅ |
| `release`, default features | 5 ✅ | 31 ✅ | 5 ✅ | 25 ✅ | 66 ✅ |
| `release`, `--no-default-features` | 5 ✅ | 31 ✅ | 5 ✅ | 25 ✅ | 66 ✅ |
| `debug`, `HDR_STRIDE=1` (all rows full size) | 5 ✅ | 31 ✅ | 5 ✅ | 25 ✅ | 66 ✅ |

`Cargo.toml` declares no `[features]`, so `{default}` and `{--no-default-features}` are the
complete set of feature combinations; `verify.sh` enumerates the power set mechanically so it
remains correct if features are added.

### Total exhaustive equivalence

`hdr_compare` reads exactly five bytes: `h2[0]`, `h2[1]`, `h2[2]`, `h1[1]`, `h1[2]`
(`h1[0]` is never touched — proved separately by rows C14/C33 and by the page-guard rows).
Row **C36** enumerates **all 2^40 = 1,099,511,627,776** combinations of those five bytes and
compares the C and the Rust on each:

```
c36: h2[0] = 0x00 done (0 matches, ...)
...
c36: h2[0] = 0xFF done (283584 matches, ...)
c36: 2^40 = 1,099,511,627,776 cases in 1158.960559936s, 283584 matches
test result: ok. 1 passed; 0 failed
```

(Log: `sweep2p40_fixed.log`, run against the final library. An earlier run of the same sweep
against the pre-fix library — `sweep2p40.log` — produced the identical counts, confirming the
`core::ptr::read` change altered only the invalid-pointer signal behaviour, not the logic.)

255 of the 256 `h2[0]` values produce zero matches (no sync byte), and `0xFF` produces exactly
283,584 — which equals the match count of the valid cross-product measured independently by
row C20. **Zero divergences.** The two implementations are therefore not merely
"tested-equivalent" but *exhaustively* equivalent over the whole reachable input domain.

## Divergence found and fixed

One genuine behavioural divergence was found — not in the logic, but at the FFI boundary on
invalid pointers, and only in builds with `-C debug-assertions=on` (the default for the
`dev`/`test` profiles). It was invisible to the release-profile tests.

**Symptom** (`ERRORS.md` rows E10/E12, `debug` profile):

```
DIVERGENCE [h2=NULL]: C = Signal(11), Rust = Signal(6)
```

The C returns/dies with `SIGSEGV` (11); the Rust died with `SIGABRT` (6).

**Cause.** `src/lib.rs` read header bytes with a plain raw-pointer dereference, `*h.add(i)`.
With `-C debug-assertions=on`, `rustc` emits a null-pointer *precondition* check in front of
every such dereference:

```asm
cmp    $0x0,%rax          ; is the pointer null?
sete   %al
...
call   *0x3bec9(%rip)     ; -> panic "null pointer dereference occurred"
```

That panic tries to unwind out of an `extern "C"` function, which aborts the process
(`SIGABRT`). The C has no such check: `h[i]` simply performs the load and the hardware raises
`SIGSEGV`. So for a null (or otherwise unmapped) pointer the two libraries terminated
differently — an observable difference for any caller that installs a handler, uses
`waitpid`, or runs the library under a supervisor.

**Fix.** All byte loads now go through a single helper that uses `core::ptr::read`, which
carries no such instrumentation and lowers to the identical single `movb`:

```rust
#[inline(always)]
unsafe fn byte(p: *const u8, i: usize) -> u8 {
    core::ptr::read(p.add(i))
}
```

Verified: the debug `.so` no longer contains the `"null pointer dereference occurred"` string,
`core::ptr::read::<u8>` compiles to `mov (%rdi),%al; ret` even at `-O0`, and rows E10–E17 now
report the same signal from both libraries in both profiles.

**Regression guards added:**

* `tests/symbols.rs::rust_so_has_no_null_pointer_precondition_check` fails if a raw `*ptr`
  dereference is ever reintroduced (it greps the built `.so` for the panic message).
* The harness (`tests/common/mod.rs::rust_so_path`) is now **profile-strict**: it will build
  the `cdylib` for the profile under test rather than silently falling back to another
  profile's artifact. That fallback is what let the first debug run pass while loading the
  release `.so`, hiding this bug.

## Behavioural contract established by the suite

Beyond the return value, the tests pin down the C's *memory-access* behaviour, which a
translation can easily get wrong:

| contract | rows |
|---|---|
| `h1[0]` is never read | C14, C33 |
| `h1` is not touched at all when `hdr_valid(h2)` is false | E6b |
| nothing past index 2 is read from either pointer | C18, E18 |
| `h2[1]` is not read when `h2[0] != 0xFF` | E21 |
| `h2[2]` is not read when the `h2[1]` checks fail | E22 |
| `h1[2]` is not read when the `h1[1]` check fails | E23 |
| the return value is exactly `0` or `1`, never another truthy int | C31, E20 |
| the function is **not** symmetric in its arguments | C30 |
| the function is stateless | C32 |

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols in the Rust `.so`, and **0**
      unresolved non-libc symbols. The C's entire exported ABI is `hdr_compare`; the Rust
      `.so` exports it with the exact same name. `hdr_valid` is `static` in the C and is
      correctly *not* exported by either.
- [x] Phase B: **every** row of `CONFIGS.md` (C1–C36) passes, across randomized inputs with a
      fixed seed, and the exhaustive rows cover the entire 2^40 reachable input space.
- [x] Phase C: **every** row of `ERRORS.md` (E1–E23, G1) has a passing differential test that
      asserts the *same* rejection (same `0` sentinel, or death by the same signal — never
      merely "both failed").
- [x] All of the above hold under **every** feature combination (`{default}`,
      `{--no-default-features}`) in **both** the `debug` and `release` profiles, and with
      `HDR_STRIDE=1` forcing every row to full size in `debug` as well.
