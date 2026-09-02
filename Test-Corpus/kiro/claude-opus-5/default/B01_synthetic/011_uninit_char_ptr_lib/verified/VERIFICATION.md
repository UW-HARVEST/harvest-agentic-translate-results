# VERIFICATION.md — results

Ground truth: `c_src/src/driver.c` built by CMake into `c_src/build/libdriver.so`.
Under test: `translation/src/lib.rs` built into `translation/target/{release,debug}/libdriver.so`.

Reproduce everything with:

```sh
translation/scripts/verify.sh          # build both, symbol parity, full suite, all combos
translation/scripts/mutation-check.sh  # negative control: prove the suite can fail
```

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0 missing** symbols: the C `.so` exports
      `bad driver good printLine` and the Rust `.so` exports all four under the
      exact same names. 0 unresolvable imports (`dlopen` with `RTLD_NOW`
      succeeds; every undefined symbol is glibc or the libgcc unwinder).
      Nothing is stubbed; `src/driver.c` is translated in full — it is the
      library's only translation unit.
- [x] **Phase B** — all **26 rows** of `CONFIGS.md` pass, with randomized inputs
      per row from a fixed seed (`0x5EED_D1FF_2025_0901`, split-mix64);
      ~40 000 differential comparisons total.
- [x] **Phase C** — all **15 rows** of `ERRORS.md` have a passing error-path
      differential test, each asserting the same *specific* rejection
      (byte-identical stdout, and identical fatal signal where the C faults).
- [x] **Every configuration** — `Cargo.toml` declares no `[features]`, so the
      only feature combinations are the default and empty sets; both are run.
      Beyond that, the suite is run against **both cargo profiles** (release and
      dev), and the non-x86-64 fallback is type-checked for
      `aarch64-unknown-linux-gnu`.

38 tests, 0 failures, in each configuration.

## How the CWE-457 read is made comparable

`bad()` reads an uninitialized `char *`, so its output is a function of stack
residue — with an uncontrolled stack it is a coin flip driven by ASLR (measured:
~50/50 between "prints nothing" and "prints library text", for *both* libraries).
Comparing that directly proves nothing.

`tests/differential.rs` therefore pins the residue: inline asm writes 64 chosen
words to `[rsp-512 .. rsp-8]` immediately before an indirect call, and the C and
the Rust symbol are then called from one identical call site at one identical
stack depth. Two patterns are used:

* **uniform** — every word gets the same value, putting the *content* `bad()`
  emits under test;
* **indexed** — word *i* points at the distinct string `slotNNNN`, so the bytes
  that come out name the exact stack word the callee read.

Under the indexed pattern the two libraries name the same word every time:
`bad()` reads `slot0060`, `driver(0)` reads `slot0056`. That is the check that
makes a frame-layout mismatch a failure rather than a coincidence.

## Fixes made to the Rust side

**The four functions were rewritten as `#[naked]` transcriptions of the C
build's disassembly.** The previous implementation used `MaybeUninit` slots plus
volatile accesses; that matched the C at `--release` but *not* in the `dev`
profile, where `printLine`/`bad`/`good` grow from `sub $0x10,%rsp` to
`0x90`/`0x70`/`0x80` and the slot `bad` reads moves ten words — a different
string reaches stdout. Since the frame geometry is observable through the
uninitialized read, leaving it to the code generator makes the byte-identical
property depend on the optimization level.

After the rewrite the Rust `.text` for all four functions is
instruction-for-instruction identical to the C's (same opcodes and lengths; only
rip-relative displacements and PLT slot addresses differ), and both profiles pass
the whole suite. `driver`'s `mov eax, 0` before each call and the trailing `nop`
in every function are included for the same reason: with a stale pointer aimed
into the library's own text, the instruction stream is itself part of the output.

Also confirmed as load-bearing and left in place:

* `build.rs` passing `-Wl,-z,lazy` — the C reaches `good`/`bad`/`printLine`/`puts`
  through lazily-bound PLT slots, so the first call from `driver` runs
  `_dl_runtime_resolve`, which overwrites the very word `bad` is about to read.
  Removing it is detected (mutant `nonlazy`).
* `.cargo/config.toml`'s `-Cforce-frame-pointers=yes` — now only relevant to the
  portable non-x86-64 fallback, since the naked functions carry their own
  prologue. The comment in that file was updated to say so.

## Negative control

`scripts/mutation-check.sh` builds eight deliberately-wrong copies of the library
in a scratch directory and checks the suite rejects each. All eight are rejected:

| mutant | change | rejected by |
|--------|--------|-------------|
| `nonullcheck` | drop `if (line != NULL)` | ERRORS rows 1/3, CONFIGS 13/17 |
| `lowbyte` | `cmp byte` instead of `cmp dword` on `useGood` | CONFIGS 21/22, ERRORS 6–11 |
| `driver_frame32` | `driver` frame 32 bytes instead of 16 | CONFIGS 20/23/24/26 |
| `printline_frame32` | `printLine` frame 32 bytes instead of 16 | CONFIGS 17 |
| `badslot` | `bad` reads `[rbp-16]` instead of `[rbp-8]` | CONFIGS 16/17/26 + 6 more |
| `goodslot` | `good` stores to `[rbp-16]` | CONFIGS 16/23/24 + 6 more |
| `offbyone` | `printLine` emits `line+1` | 33 tests |
| `nonlazy` | link with `-z now` | CONFIGS 18/20 |

## Known limits

* **Same-process, same-depth comparison.** The differential holds the caller
  identical and pins the residue. It does *not* claim the two `.so` files behave
  identically when the residue is left to the dynamic loader: a 16 KiB C object
  and a 400 KiB Rust object make the loader touch different amounts of stack, and
  that difference is not a property of the translation. (Confirmed by sweeping an
  `alloca` offset before the call under `setarch -R`: with the pre-call stack
  controlled, the two agree at every offset.)
* **Stale pointer into the library's own text.** If the residue happens to point
  into the `.so`'s text, `printLine` prints machine code. Both libraries read the
  same address, but equality of the *bytes* there would need byte-identical
  codegen for the whole object, not identical behaviour. The naked-asm rewrite
  narrows this to the code outside these four functions.
* **Non-x86-64.** The portable fallback is type-checked for aarch64 but not
  differentially tested — there is no aarch64 C build or runtime here. Its frame
  layout comes from the code generator, so byte-identical residue is not claimed
  for it.
* **Run the suite single-threaded.** It redirects fd 1 and mutates a global
  residue pattern. `tests/differential.rs` takes a process-wide lock in every
  test so this is enforced, and the scripts also pass `--test-threads=1`.
