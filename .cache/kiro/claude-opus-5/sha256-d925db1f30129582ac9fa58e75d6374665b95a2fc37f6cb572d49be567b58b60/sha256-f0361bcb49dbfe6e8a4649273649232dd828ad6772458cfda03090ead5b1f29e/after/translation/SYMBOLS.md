# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C `.so`:   `c_src/build/libharvest-work-SfXVz9.so`
- Rust `.so`: `translation/target/debug/libfindrep_lib.so`

Commands used:

```sh
nm -D --defined-only c_src/build/libharvest-work-SfXVz9.so   | awk '{print $3}' | sort
nm -D --defined-only translation/target/debug/libfindrep_lib.so | awk '{print $3}' | sort
comm -23 <c-list> <rust-list>   # must be empty
```

## Public symbols defined by the C `.so`

| # | symbol | C signature | exported by Rust `.so`? |
|---|--------|-------------|-------------------------|
| 1 | `add_to_accumulator` | `int add_to_accumulator(int, int)` | YES |
| 2 | `multiply_with_multiplier` | `int multiply_with_multiplier(int, int)` | YES |
| 3 | `subtract_from_accumulator` | `int subtract_from_accumulator(int, int)` | YES |
| 4 | `divide_multiplier` | `int divide_multiplier(int, int)` | YES |
| 5 | `process_octal_string` | `void process_octal_string(char*, int)` | YES |
| 6 | `find_and_replace_char` | `void find_and_replace_char(char*, int)` | YES |
| 7 | `validate_and_normalize` | `int validate_and_normalize(int)` | YES |
| 8 | `findrep` | `int findrep(int, int, int, int)` | YES (declared in `include/lib.h`) |

`comm -23` output (C-defined symbols missing from Rust): **empty** — 0 missing.

## Non-exported C internals (no symbol required)

These are `static` in the C translation unit, therefore not part of the ABI, and
correctly have no exported counterpart in Rust:

| C entity | kind | Rust counterpart |
|----------|------|------------------|
| `accumulator` | `static int` | `static mut ACCUMULATOR` (private) |
| `multiplier` | `static int` (init `1`) | `static mut MULTIPLIER` (private) |
| `operation_count` | `static int` | `static mut OPERATION_COUNT` (private) |
| `operations[4]` | `static operation_func[4]` | `static OPERATIONS: [OperationFunc; 4]` (private) |
| `operation_func` | typedef | `type OperationFunc` |
| `string_processor` | typedef (declared, never used) | `type StringProcessor` (`#[allow(dead_code)]`) |

## Undefined (imported) symbols

C imports: `memchr`, `sprintf`, `strcpy`, `strlen` (all glibc) plus the standard
weak CRT symbols.

Rust imports: glibc (`malloc`, `memcpy`, `strlen`, `write`, …) and the
`_Unwind_*` family from `libgcc`. **0 undefined non-libc / non-runtime
symbols** — nothing unresolved that belongs to the translated library itself.

## Verdict

- [x] Every symbol exported by the C `.so` is exported by the Rust `.so` with
      the exact same name.
- [x] No stubs / `unimplemented!()` — every export has a real translated body.
- [x] 0 missing, 0 undefined non-libc symbols in the Rust `.so`.

## Phase D results

`scripts/phase_d.sh` re-derives the symbol diff and re-runs the whole suite for
every feature combination:

```
=== Phase D.1: feature enumeration ===
declared features: <none> (count=0)
combinations to verify: 2

=== combo: <default> ===
  symbol parity: OK (0 missing)
  test result: ok. 35 passed; 0 failed      (phase_b_valid)
  test result: ok. 32 passed; 0 failed      (phase_c_errors)

=== combo: --no-default-features ===
  symbol parity: OK (0 missing)
  test result: ok. 35 passed; 0 failed
  test result: ok. 32 passed; 0 failed

=== Phase D.2: binary/driver check ===
  no Rust [[bin]] target
  no C add_executable target
  => project builds no driver binary; stdout comparison N/A

PHASE D: PASS (all 2 combinations: symbols parity + tests green)
```

`Cargo.toml` has no `[features]` section and `lib.c` has zero `#if`/`#ifdef`
directives, so the powerset of feature combinations is exactly
`{default, --no-default-features}`; the script enumerates it rather than
assuming it, and would expand automatically if features were added.

The `release` profile (`panic = "abort"`, optimised) was verified separately and
also shows an empty symbol diff with all 67 tests passing — worth checking on
its own because the C is compiled without optimisation (no `CMAKE_BUILD_TYPE`),
where its signed-overflow UB wraps; the optimised Rust `wrapping_*` operations
still match.
