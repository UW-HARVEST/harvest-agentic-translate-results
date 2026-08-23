# How to translate a fragment of PCRE2's internal_dfa_match()

You are translating one contiguous region of the giant `switch (codevalue)`
inside `internal_dfa_match()` in `c_src/src/pcre2_dfa_match.c` into a Rust
"fragment file" `src/pcre2_dfa_opsN.rs`.

Read, in this order:
1. `TRANSLATION_GUIDE.md` (general C->Rust conventions for this port)
2. `src/internal.rs` (types, structs, constants, callable library functions)
3. `src/macros.rs` (Rust versions of the C macros)
4. `src/pcre2_dfa_match.rs` (the skeleton: the prologue, the subject loop, the
   active-state loop, the local macros) and `src/pcre2_dfa_match_head.rs`
   (constants, `stateblock`, `RWS_anchor`, and the `OPX_*` constants).
5. Your region of `c_src/src/pcre2_dfa_match.c`.

## Shape of your output file

The whole file is ONE Rust block expression that is textually included inside
the `'next_active_state: { ... }` block of the active-state loop:

```rust
{
    match codevalue {
        OP_KET | OP_KETRMIN | OP_KETRMAX | OP_KETRPOS => {
            /* ... */
            break 'next_active_state;
        }

        OPX_PROP_TYPEPLUS | OPX_PROP_TYPEMINPLUS | OPX_PROP_TYPEPOSPLUS => { /* ... */ }

        _ => {}
    }
}
```

* No `use`, no `fn`, no `const`, no `static` - only that block.
* The last arm must be `_ => {}` so unhandled opcodes fall through to the next
  fragment (and finally to the C `default: return PCRE2_ERROR_DFA_UITEM;`).
* Rust patterns cannot contain arithmetic: the C case labels
  `OP_PROP_EXTRA + OP_TYPEPLUS` etc. have named constants `OPX_PROP_TYPEPLUS`,
  `OPX_EXTUNI_TYPEQUERY`, `OPX_ANYNL_TYPESTAR`, `OPX_HSPACE_TYPEEXACT`,
  `OPX_VSPACE_TYPEPOSUPTO`, ... (pattern: `OPX_<PROP|EXTUNI|ANYNL|HSPACE|VSPACE>_<TYPExxx>`)
  declared in `src/pcre2_dfa_match_head.rs`.

## Control flow

| C | Rust |
|---|---|
| `break;` that leaves the `switch(codevalue)` | `break 'next_active_state;` |
| `continue;` at switch-arm level (next active state) | `break 'next_active_state;` |
| `goto NEXT_ACTIVE_STATE;` | `break 'next_active_state;` |
| `return x;` | `return x;` |
| `ADD_ACTIVE(x,y)` / `ADD_ACTIVE_DATA(x,y,z)` / `ADD_NEW(x,y)` / `ADD_NEW_DATA(x,y,z)` | `ADD_ACTIVE!(x, y)` / `ADD_ACTIVE_DATA!(x, y, z)` / `ADD_NEW!(x, y)` / `ADD_NEW_DATA!(x, y, z)` (they can `return PCRE2_ERROR_DFA_WSSIZE`, exactly like the C macros) |
| `PCRE2_ASSERT(...)` / `PCRE2_UNREACHABLE()` | nothing (no-ops; never a panic) |

Beware: `break`/`continue` inside a nested `for`/`while` in the C code refer to
that loop; only at `switch`-arm level do they mean "next active state".
The forward gotos `goto ANYNL01/ANYNL02/ANYNL03` and `goto QS1..QS5` jump into
the *following* case's code. Reproduce them faithfully: if the target label is
inside your own region, restructure with a Rust labelled block or by duplicating
the small piece of code the label introduces - but do not change behaviour. (The
region boundaries were chosen so that every goto and its target are in the same
fragment.)

## Available locals (declared by the skeleton - do not redeclare)

Function level: `mb`, `this_start_code`, `current_subject`, `start_offset`,
`offsets`, `offsetcount`, `workspace`, `wscount`, `rlevel`, `RWS`,
`active_states`, `new_states`, `next_active_state`, `next_new_state`, `ctypes`,
`lcc`, `fcc`, `ptr`, `end_code`, `new_recursive`, `active_count`, `new_count`,
`match_count`, `start_subject`, `end_subject`, `start_code`, `utf`,
`utf_or_ucp`, `reset_could_continue`.
Subject-loop level: `i`, `j`, `clen`, `dlen`, `c`, `d`, `partial_newline`,
`could_continue`.
State-loop level: `current_state`, `caseless`, `code`, `codevalue`,
`state_offset`, `rrc`, `count`.
Tables: `coptable`, `poptable`, `toptable1`, `toptable2` (read them with
`*coptable.as_ptr().add(i)` to avoid bounds-check panics).
Helpers: `do_callout_dfa(code, offsets, current_subject, ptr, mb, extracode, &mut length) -> c_int`
and `more_workspace(&mut rws, ovecsize, mb) -> c_int` (in
`src/pcre2_dfa_match_helpers.rs`), plus recursive calls to
`internal_dfa_match(...)`.
Library functions: `_pcre2_xclass_8`, `_pcre2_eclass_8`, `_pcre2_extuni_8`,
`_pcre2_is_newline_8`, `_pcre2_was_newline_8`, `_pcre2_strncmp_8`,
`_pcre2_ord2utf_8`, `UCD_*`, `_pcre2_OP_lengths_8` etc.
`IS_NEWLINE(x)` in this file means `IS_NEWLINE!(x, mb, (*mb).end_subject, utf)`;
`WAS_NEWLINE(x)` means `WAS_NEWLINE!(x, mb, (*mb).start_subject, utf)`.

## Verification

Run:
```
cd $HARVEST_WORKDIR/translated_rust && cargo build --release 2>&1 | grep -E "^error" -A15 | head -80
```
Other agents are writing other fragments at the same time: only fix errors whose
span points at YOUR file. Iterate until your file is clean.
