# FINDINGS — divergences found and fixed

Two real behavioural divergences were found by the differential suite. Both were
fixed in `src/lib.rs` (the C was never modified). Neither was visible from
symbol parity or from happy-path testing: the return values and stdout of the
normal success path matched byte-for-byte from the very first run.

---

## #1 — `checkshift`: the allocation-failure branch was optimised away

**ERRORS.md row E16.** `tests/phase_c_malloc_failure.rs`

The C allocates its state and handles failure:

```c
ComputeState* state = (ComputeState*)malloc(sizeof(ComputeState));
if (state == NULL) {
    printf("Error: Failed to allocate memory for state\n");
    return -1;
}
```

The Rust *source* faithfully mirrored this, but the optimiser deleted it. LLVM
recognises the libc `malloc`/`free` pair, saw that the pointer never escaped, and
elided the allocation entirely — the release `.so` contained **no `malloc` call
at all** inside `checkshift`:

```
$ objdump -d libcheckshift_lib.so | sed -n '/<checkshift>:/,+250p' | grep malloc
(nothing)
```

Consequences: the `malloc(sizeof(ComputeState))` request was no longer
observable, and the `state == NULL` check became dead code, so the C's
allocation-failure path was **reachable in C but unreachable in Rust**. Under
allocation failure, C prints the diagnostic and returns `-1`; Rust returned a
normally-computed result.

**How it was caught.** The failure branch cannot be reached by choosing
arguments, so `tests/phase_c_malloc_failure.rs` builds an `LD_PRELOAD` `malloc`
interposer (`tests/helpers/malloc_fail.c`) that fails allocations of exactly
`sizeof(ComputeState)` bytes, arming it only *after* `dlopen` so the loader and
library initialisers are unaffected, and drives both `.so`s out-of-process
through a common driver (`tests/helpers/driver.c`). The C took the failure path;
the Rust ran to completion. A size-logging interposer confirmed the diagnosis:
C requested `malloc(12)`, the Rust build requested no 12-byte allocation at all.

**Fix.** Force the pointer to escape so the call — and with it the NULL check —
survives optimisation:

```rust
let state =
    core::hint::black_box(unsafe { malloc(core::mem::size_of::<ComputeState>()) })
        as *mut ComputeState;
```

Verified: the release `.so` now emits `mov $0xc,%edi; call *…<malloc>` plus the
matching `free`, exactly like the C, and the E16 differential test passes.

---

## #2 — Unaligned `ComputeState*` aborted the process in debug builds

**CONFIGS.md row H4.** `tests/phase_d_hardening.rs::h4_unaligned_state_pointer`

`init_state` and `apply_operation` take a caller-supplied `ComputeState*`. C
callers may legitimately point that at a misaligned address (a byte buffer, a
packed struct, an offset into a larger allocation); gcc compiles
`state->accumulator` to a plain x86 load that works at any alignment, so the C
accepts it and prints the expected value.

The Rust used ordinary field accesses:

```rust
printf!("State initialized with accumulator = %d\n", (*state).accumulator);
...
(*state).accumulator = func((*state).accumulator, value);
(*state).operation_count = (*state).operation_count.wrapping_add(1);
```

`(*state).field` is an *aligned* access. On a misaligned pointer it is UB, and
with `debug_assertions` enabled Rust's misaligned-pointer check **aborts the
process**:

```
thread '<unnamed>' panicked at src/lib.rs:266:13:
misaligned pointer dereference: address must be a multiple of 0x4 but is 0x55fe1f206b81
thread caused non-unwinding panic. aborting.
... (signal: 6, SIGABRT)
```

So for the same input, C returned normally while Rust killed the process — a
crash-versus-works divergence. Note this was invisible in `--release` (no
alignment check, and x86 tolerates unaligned loads); it only appeared once the
suite was run under the `debug` profile too, which is why the completion gate
requires every configuration rather than just the default.

**Fix.** Route every `ComputeState` field access through unaligned-safe helpers
(`get_accumulator` / `set_accumulator` / `get_operation_count` /
`set_operation_count` / `get_checksum` / `set_checksum`) built on
`core::ptr::read_unaligned` / `write_unaligned` over `addr_of!` / `addr_of_mut!`,
matching gcc's codegen at every alignment. This also removes the UB.

---

## Verified as NOT divergences

Investigated and confirmed harmless:

- **`printf` → `puts`.** LLVM rewrites the Rust `printf("literal\n")` calls to
  `puts("literal")`. `puts` appends the newline, so the byte stream is identical;
  gcc performs the same rewrite at `-O2`. Confirmed by byte-exact transcript
  comparison over thousands of randomized `checkshift` calls.
- **Uninitialised `buffer` in `compute_checksum`.** The C declares
  `unsigned char buffer[16]` uninitialised but only ever reads
  `[0, 4*copy_count)`, which `memcpy` has just written. The Rust zero-initialises
  it, which is unobservable.
- **`int ^ unsigned int` in the final result.** C promotes both to `unsigned int`
  and converts back to `int`; the Rust does the same cast chain. Verified over
  the full boundary grid and 3000 random tuples.
- **Signed overflow in `(a*b)*3`, `(a+b)+100`, `a << 2`.** Formally UB in C; gcc
  wraps, and the Rust uses `wrapping_*` / bit-pattern shifts. Verified over
  `INT_MIN`/`INT_MAX` grids.
- **Unaligned `int*` to `compute_checksum` (H3).** Reached only via `memcpy` in
  both implementations, so alignment is irrelevant — matched at all 8 offsets.
- **`static_multiplier` / `static_addend` / `static_shift_amount`.** File-scope
  `static`s, never written after initialisation and not exported, so modelling
  them as Rust `const`s is equivalent. Confirmed absent from both dynamic symbol
  tables.
