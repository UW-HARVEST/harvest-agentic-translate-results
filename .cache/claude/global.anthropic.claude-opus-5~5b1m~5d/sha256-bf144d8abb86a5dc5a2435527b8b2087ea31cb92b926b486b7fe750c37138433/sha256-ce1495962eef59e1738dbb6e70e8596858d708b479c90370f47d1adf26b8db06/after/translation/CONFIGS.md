# CONFIGS.md — Phase A configuration-surface table

## Mechanical derivation

The public API is the whole of `c_src/include/lib.h`:

```c
char *tool_basename(char *path);
```

**Public entry points: 1.** There is no init/teardown, no options struct, no
context object, no setter, no global. `tool_basename` *is* the lowest-level
entry point as well as the only one — there is no convenience wrapper to hide
behind, so "exercise the low-level entry points, not just the wrappers" is
satisfied by construction.

**Runtime options / modes / flags: 0.** Grepping the source for the constructs
that would implement them finds nothing:

```sh
grep -nE '#if|#ifdef|switch|enum|static|extern|global' c_src/src/lib.c c_src/include/lib.h
# -> (only '#include <string.h>')
```

**Compile-time configuration: 0.** `CMakeLists.txt` defines no options and no
`target_compile_definitions`. `translation/Cargo.toml` has no `[features]`
section, so the only feature combination that exists is the default (empty) one
— see the Phase D section of `VERIFICATION.md` for the enumeration script.

So the configuration surface is entirely made of **input shapes**. These are the
axes the C actually branches on, read straight off lines 10–21 of `src/lib.c`:

| axis | values the C distinguishes | where |
|---|---|---|
| A. presence of `'/'` | absent (`s1 == NULL`) / present (`s1 != NULL`) | `strrchr(path,'/')`, lines 13/16 |
| B. presence of `'\\'` | absent (`s2 == NULL`) / present (`s2 != NULL`) | `strrchr(path,'\\')`, lines 13/18 |
| C. relative order when both present | `s1 > s2` / `s1 < s2` (equality impossible: one byte cannot be both) | ternary on line 14 |
| D. multiplicity of a separator | 0 / 1 / many (`strrchr` = **last** occurrence, not first) | `strrchr` semantics |
| E. separator position | first byte / interior / last byte (empty basename) / adjacent-run (`//`, `\\\\`, `/\`, `\/`) | pointer arithmetic `+1` |
| F. string length | 0 / 1 / small / large (1 MiB) | `strrchr` traversal |
| G. byte values | plain ASCII / bytes neighbouring the separators (`0x2E,0x30,0x5B,0x5D`) / high-bit `0x80..0xFF` (signed-`char` trap) / invalid UTF-8 | byte compare inside `strrchr` |

Rows below are the cross-product of A×B×C pruned to the combinations the code
treats differently, crossed with the shape axes D–G where they can change the
answer. Every row is driven with **many randomized inputs from a fixed seed**
(deterministic PCG in `tests/common/mod.rs`), not one hand-picked value.

## What each row asserts

For every generated input, both `.so`s are loaded via `libloading` and their
exported `tool_basename` is called through the FFI boundary. The comparison is
byte-exact:

1. returned pointer **offset** from the buffer base is equal (`c_ret - c_buf ==
   rust_ret - rust_buf`) — this is the full information content of the return
   value, since the result always aliases the input;
2. the returned C strings are byte-identical;
3. neither implementation is `NULL`;
4. the input buffer is byte-identical after each call (no mutation);
5. each implementation gets its own copy of the buffer, so cross-contamination
   cannot mask a difference.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | randomized inputs | test | [x] |
|---|----------------|--------------------------------------------|------|------|-----|
| C1 | `tool_basename` | no options (none exist); **no separator at all**, random ASCII body, random length 0–64 (axis A=absent, B=absent, F=0..64) | 2 000 | `phase_b_configs.rs::c01_no_separator` | [x] |
| C2 | `tool_basename` | `'/'` only, exactly **one** occurrence at a random interior index (A=present ×1, B=absent, D=1, E=interior) | 2 000 | `phase_b_configs.rs::c02_single_slash_interior` | [x] |
| C3 | `tool_basename` | `'\\'` only, exactly **one** occurrence at a random interior index (A=absent, B=present ×1) | 2 000 | `phase_b_configs.rs::c03_single_backslash_interior` | [x] |
| C4 | `tool_basename` | `'/'` only, **many** occurrences at random indices — checks `strrchr` returns the *last*, not the first (D=many) | 2 000 | `phase_b_configs.rs::c04_many_slashes` | [x] |
| C5 | `tool_basename` | `'\\'` only, **many** occurrences at random indices (D=many) | 2 000 | `phase_b_configs.rs::c05_many_backslashes` | [x] |
| C6 | `tool_basename` | **both** present, forced `s1 > s2` (last `'/'` strictly after last `'\\'`) → exercises the true arm of the line-14 ternary (A,B present, C=`s1>s2`) | 2 000 | `phase_b_configs.rs::c06_both_slash_after_backslash` | [x] |
| C7 | `tool_basename` | **both** present, forced `s1 < s2` (last `'\\'` strictly after last `'/'`) → false arm of the ternary (C=`s1<s2`) | 2 000 | `phase_b_configs.rs::c07_both_backslash_after_slash` | [x] |
| C8 | `tool_basename` | **both** present, **many** of each at random interleaved positions, order of the two last occurrences left to chance (C=either, D=many/many) | 4 000 | `phase_b_configs.rs::c08_both_many_random_order` | [x] |
| C9 | `tool_basename` | separator at the **first** byte (`"/rest"`, `"\\rest"`), random tail (E=first) | 2 000 | `phase_b_configs.rs::c09_separator_first_byte` | [x] |
| C10 | `tool_basename` | separator at the **last** byte → empty basename, returned pointer is the NUL terminator (E=last) | 2 000 | `phase_b_configs.rs::c10_separator_last_byte` | [x] |
| C11 | `tool_basename` | **runs of adjacent separators** (`//`, `\\\\`, `/\`, `\/`, length-2..6 runs) at a random index (E=adjacent-run) | 2 000 | `phase_b_configs.rs::c11_adjacent_separator_runs` | [x] |
| C12 | `tool_basename` | length **0** and length **1** strings across every relevant byte (`""`, `"/"`, `"\\"`, `"a"`, `0x00..0xFF` single byte) — exhaustive, not random (F=0/1) | 258 exhaustive | `phase_b_configs.rs::c12_tiny_lengths_exhaustive` | [x] |
| C13 | `tool_basename` | **large** buffers: 64 KiB–1 MiB, separators at random far-out offsets, plus one with no separator (F=large) | 60 | `phase_b_configs.rs::c13_large_buffers` | [x] |
| C14 | `tool_basename` | body drawn from the **separator-neighbour bytes** `{0x2E,0x30,0x5B,0x5D}` only, with occasional real separators mixed in (G=neighbours) | 3 000 | `phase_b_configs.rs::c14_separator_neighbour_bytes` | [x] |
| C15 | `tool_basename` | body drawn from **high-bit bytes** `0x80..=0xFF` (signed-`char` sign-extension trap, incl. `0xAF`/`0xDC`), with occasional real separators (G=high-bit) | 3 000 | `phase_b_configs.rs::c15_high_bit_bytes` | [x] |
| C16 | `tool_basename` | body of **fully random bytes `0x01..=0xFF`** (invalid UTF-8 guaranteed in practice; separators appear naturally at random) — the unrestricted fuzz row (G=arbitrary) | 5 000 | `phase_b_configs.rs::c16_arbitrary_bytes_fuzz` | [x] |
| C17 | `tool_basename` | **realistic path corpus** crossed with both separator styles and mixed styles: POSIX (`/usr/bin/tool`), Windows (`C:\dir\tool.exe`), UNC (`\\host\share\f`), mixed (`C:/dir\tool`), relative (`./a/b`), dot-files, trailing-dot names, random component counts 1–8 and random component lengths | 4 000 | `phase_b_configs.rs::c17_realistic_path_corpus` | [x] |
| C18 | `tool_basename` | **idempotence / composition**: feed each implementation's own output back into itself 3× (the composed pipeline a real consumer builds), across a randomized corpus — divergence in composition is invisible to single-call rows | 2 000 × 3 | `phase_b_configs.rs::c18_repeated_application` | [x] |
| C19 | `tool_basename` | **aliasing / non-mutation contract at scale**: same buffer passed to C then Rust then C again, asserting the buffer never changes and the offset is stable across call order | 2 000 | `phase_b_configs.rs::c19_call_order_and_no_mutation` | [x] |
| C20 | `tool_basename` | **interior-pointer arithmetic edge**: buffer where the only separator sits at index `len-2` (basename is exactly 1 byte) and at index `len-1` (basename empty), swept over lengths 2..=64 for both separators | exhaustive sweep 252 | `phase_b_configs.rs::c20_basename_length_zero_and_one` | [x] |

**20 / 20 rows pass across randomized inputs. Phase B gate satisfied.**
