# Differential-test harness contract (`tests/common/mod.rs`)

Every test file starts with:

```rust
mod common;
use common::*;
```

and calls `setup()` first. **Never call a Rust function directly** — always go
through `dlsym` on the two `.so`s so the `#[no_mangle]` export wrappers are
tested too.

## Library access

| item | meaning |
|---|---|
| `c_lib() -> &'static Library` | the C reference `.so` (`c_src/build/libsodium.so`) |
| `r_lib() -> &'static Library` | the Rust `.so` (`target/debug/liblibsodium.so`) |
| `sym::<T>(lib, "name") -> T` | look one symbol up, transmuted to fn type `T` |
| `pair::<T>("name") -> (T, T)` | `(c_fn, rust_fn)` for the same symbol |
| `has_sym("name") -> bool` | present in both |

`T` is always a bare `unsafe extern "C" fn(..) -> ..` type. Map C types as:
`size_t` -> `usize`, `unsigned long long` -> `u64`, `uint64_t` -> `u64`,
`uint32_t` -> `u32`, `int` -> `i32`, `unsigned char *` -> `*mut u8`,
`const unsigned char *` -> `*const u8`, `const char *` -> `*const std::ffi::c_char`,
`size_t *` -> `*mut usize`, `unsigned long long *` -> `*mut u64`,
an opaque `*_state *` -> `*mut u8`.

## Setup

* `setup()` — idempotent. Installs a **deterministic `randombytes`
  implementation into BOTH libraries** (via `randombytes_set_implementation`)
  and calls `sodium_init()` on both.
* `reset_rngs(seed)` — rewinds *both* libraries' RNG streams to the same seed.
  The RNG state is **thread-local**, so concurrently running tests in the same
  binary cannot disturb each other's streams — no locking needed.
  Call it immediately before **each** of the two calls in a pair whose result
  depends on `randombytes_*` (keypair/keygen/`_random`):
  ```rust
  reset_rngs(s); unsafe { c_fn(a.as_mut_ptr()) };
  reset_rngs(s); unsafe { r_fn(b.as_mut_ptr()) };
  eq_bytes("...", &a, &b);
  ```

## Randomised inputs (fixed seed — reproducible)

`Rng::new(seed)` with `.next_u64()`, `.next_u32()`, `.byte()`, `.below(n)`,
`.range(lo,hi)`, `.fill(&mut [u8])`, `.bytes(n) -> Vec<u8>`, `.bool()`,
`.pick(&slice)`. Use MANY inputs per row, not one hand-picked value.

## Opaque state buffers

`State::for_sym("crypto_x_statebytes")` allocates a 64-byte-aligned buffer sized
by the library's own accessor (use this for every `*_state`). `State::new(len)`
for an explicit size. `.as_mut_ptr()`, `.as_ptr()`, `.bytes()`, `.len()`.
Allocate a **separate** state per library — never share one across the two.

`chunks(&mut rng, total, style)` -> `Vec<usize>` summing to `total`;
`style` 0 = one chunk, 1 = one byte at a time, 2 = two chunks at a random split,
3 = random walk. Use it to drive `_update` in many different chunkings.

## Comparison

`eq_bytes(what, &c, &r)`, `eq_i32(what, c, r)`, `eq_usize(what, c, r)`,
`canary(n) -> Vec<u8>` (0xA5-filled output buffer, so writes past the expected
end are visible), `hex(&[u8]) -> String`.

## `sodium_misuse()` / `abort()` / `assert()` paths — run OUT OF PROCESS

`sodium_misuse()` calls the installed handler and then `abort()`s, so those rows
must be run in a child process. Pattern (one `#[test]` child + one `#[test]`
parent per file):

```rust
const MISUSE_CASES: &[&str] = &["tag/one", "tag/two"];

#[test]
fn misuse_child() {
    let Some((tag, lib)) = child_case() else { return; }; // parent: no-op
    // `child_case()` already ran setup() and installed the observing handler
    // on `lib` (the C or the Rust library, per DIFFTEST_WHICH).
    let mut out = canary(64);
    match tag.as_str() {
        "tag/one" => {
            set_observation(out.as_ptr(), out.len()); // printed by the handler
            let f = sym::<Fn1>(lib, "crypto_x");
            let rc = unsafe { f(/* the misusing arguments */) };
            println!("OBS rc={rc} out={}", hex(&out));   // only if it returned
        }
        _ => panic!("unknown tag {tag}"),
    }
    use std::io::Write; let _ = std::io::stdout().flush();
    std::process::exit(0);           // reached only if the library did NOT abort
}

#[test]
fn misuse_paths_match() {
    if child_tag().is_some() { return; }
    setup();
    for &tag in MISUSE_CASES {
        let c = run_child("misuse_child", "c", tag);
        let r = run_child("misuse_child", "r", tag);
        eq_child(tag, &c, &r);   // same exit code + signal + same "MISUSE"/"OBS" stdout
        assert_eq!(c.status.code(), Some(MISUSE_EXIT),
            "{tag}: C did not reach sodium_misuse (stderr: {})",
            String::from_utf8_lossy(&c.stderr));
    }
}
```

`MISUSE_EXIT == 77`. `set_observation(ptr, len)` registers a byte range that the
handler prints as `MISUSE obs=<hex>` — use it to compare out-parameters that the
C writes *before* aborting. For a raw `assert()` failure (no handler involved)
the child dies with SIGABRT instead; `eq_child` compares that too, so the same
pattern works — just don't assert exit code 77 for those, assert
`c.status.signal() == Some(6)` and that C and Rust agree.

## `errno`

`std::io::Error::last_os_error().raw_os_error()`. Clear it first with a syscall
known to succeed (e.g. `let _ = std::fs::metadata("/");`). Common values on
Linux: `EINVAL=22`, `ERANGE=34`, `ENOMEM=12`, `EFBIG=27`, `ENOSYS=38`,
`EPERM=1`, `ENXIO=6`.

## Rules

* **Never modify anything in `c_src/`.** The C is ground truth.
* On any divergence, fix the **Rust** source under `src/`.
* Build/run with `timeout 600 cargo test --no-default-features --test <name>`.
  (`cargo test` rebuilds the cdylib automatically.)
* Keep any single test under ~60 s. Skip/downscale genuinely huge work
  (multi-GiB buffers, `SENSITIVE` pwhash limits) and say so in a comment.
