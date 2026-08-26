# How to translate a fragment of PCRE2's match() interpreter

You are translating one contiguous region of the giant `switch` inside
`match()` in `c_src/src/pcre2_match.c` into a Rust "fragment file"
`src/pcre2_match_opsN.rs`.

Read, in this order:
1. `TRANSLATION_GUIDE.md` (general C->Rust conventions for this port)
2. `src/internal.rs` (types, structs, constants, callable library functions)
3. `src/macros.rs` (Rust versions of the C macros: `GET!`, `GET2!`,
   `GETCHARINC!`, `GETCHARINCTEST!(c, p, utf)`, `GETCHARLEN!`, `BACKCHAR!`,
   `FORWARDCHAR!`, `ACROSSCHAR!`, `TABLE_GET!`, `MAX_255!`, `CHMAX_255!`,
   `IS_NEWLINE!(p, blk, psend, utf)`, `WAS_NEWLINE!(p, blk, psstart, utf)`, ...)
4. `src/pcre2_match.rs` - the skeleton. READ IT ALL. It contains the state
   machine, all the local variables, the state constants and the local macros
   `CHECK_PARTIAL!()`, `SCHECK_PARTIAL!()`, `RMATCH!(code, RMn)`,
   `RRETURN!(rc)`, `Fov!(i)`.
5. Your region of `c_src/src/pcre2_match.c`.

## Shape of your output file

The whole file is ONE Rust block expression that is textually included inside
the `'sm: loop { ... }` of `match()`:

```rust
{
    match state {
        OP_ANY => {
            /* ... */
            state = ST_TOP;
            continue 'sm;
        }

        OP_CHAR => { /* ... */ }

        ST_L_RM7 => { /* code that in C follows RMATCH(..., RM7) */ }

        _ => {}
    }
}
```

* No `use` statements, no `fn`, no `const`, no `static` - only that block.
* Every `match` arm you write must end by transferring control (see below); the
  final arm must be `_ => {}` so that unhandled states fall through to the next
  fragment.
* Do not touch any other file.

## Control-flow translation

| C | Rust |
|---|---|
| `case OP_X:` (start of a case) | `OP_X => { ... }` |
| several `case`s sharing code | `OP_A \| OP_B => { ... }` |
| `break;` leaving the switch (repeat main loop) | `state = ST_TOP; continue 'sm;` |
| `continue;` at switch-arm level (repeat main loop) | `state = ST_TOP; continue 'sm;` |
| `RMATCH(code, RMn);` | `RMATCH!(code, RMn);` **and** the C code following it goes into a new arm `ST_L_RMn => { ... }` |
| `RRETURN(x);` | `RRETURN!(x);` |
| `return x;` | `return x;` |
| `goto LABEL;` | `state = ST_LABEL; continue 'sm;` |
| a label `LABEL:` inside the switch | a new arm `ST_LABEL => { ... }`; code that falls into it sequentially must end with `state = ST_LABEL; continue 'sm;` |
| `CHECK_PARTIAL();` | `CHECK_PARTIAL!();` |
| `SCHECK_PARTIAL();` | `SCHECK_PARTIAL!();` |
| `PCRE2_ASSERT(x)` / `PCRE2_DEBUG_UNREACHABLE()` / `PCRE2_UNREACHABLE()` | nothing (they are no-ops in this build; NEVER a panic) |

**Beware of `break` and `continue`:** in the C source a `break`/`continue` at
switch-arm level leaves the switch / repeats the main loop, but inside a nested
`for`/`while`/`do` it applies to that loop. Check the nesting carefully. Inside
your Rust arm, `break`/`continue` refer to the nearest Rust loop **you** wrote;
a bare `break` would break out of `'sm` - never do that.

**Loops containing an RMATCH** must become explicit state transitions, because
the resumption point (`ST_L_RMn`) is a separate arm. Example:

```c
for (;;)
  {
  if (Feptr <= Lstart_eptr) break;
  RMATCH(Fecode, RM203);
  if (rrc != MATCH_NOMATCH) RRETURN(rrc);
  Feptr--;
  }
break;    /* leaves the switch */
```
becomes (using one of the spare states allocated to your fragment):

```rust
ST_Ck_1 => {                      /* top of the C for(;;) loop */
    if (*F).eptr <= Lstart_eptr { state = ST_TOP; continue 'sm; }  /* the C break */
    RMATCH!((*F).ecode, RM203);
}

ST_L_RM203 => {
    if rrc != MATCH_NOMATCH { RRETURN!(rrc); }
    (*F).eptr = (*F).eptr.sub(1);
    state = ST_Ck_1; continue 'sm;         /* back to the top of the loop */
}
```
and the code that entered the loop ends with `state = ST_Ck_1; continue 'sm;`.
The spare state constants `ST_Ck_1 .. ST_Ck_8` (k = your fragment number) are
already declared in the skeleton. This is safe because PCRE2 keeps everything
that must survive an RMATCH in the frame (the `F...`/`L...` fields), never in a
local variable - see the comment in the C source.

## Field accessors

The C source uses macros for the current frame `F`:

| C | Rust |
|---|---|
| `Fback_frame` | `(*F).back_frame` (usize) |
| `Fcapture_last` | `(*F).capture_last` (u32) |
| `Fcurrent_recurse` | `(*F).current_recurse` (u32) |
| `Fecode` | `(*F).ecode` (`*const u8`) |
| `Feptr` | `(*F).eptr` (`*const u8`) |
| `Fgroup_frame_type` | `(*F).group_frame_type` (u32) |
| `Flast_group_offset` | `(*F).last_group_offset` (usize) |
| `Fmark` | `(*F).mark` (`*const u8`) |
| `Frdepth` | `(*F).rdepth` (u32) |
| `Fstart_match` | `(*F).start_match` (`*const u8`) |
| `Foffset_top` | `(*F).offset_top` (usize) |
| `Fop` | `(*F).op` (u8 - cast with `as u32` when comparing with `OP_xxx`) |
| `Fovector[i]` | `Fov!(i)` |
| `Freturn_id` | `(*F).return_id` (u8) |
| `F->recurse_last_used` | `(*F).recurse_last_used` |

The `Lxxx` macros are `#define`d immediately before each opcode group in the C
source and `#undef`ed after it. **Look at the `#define`s that are in force in
your region** and expand them by hand, e.g. inside the char-repeat group
`Lmin` is `(*F).fields.char_repeat.min` while inside the type-repeat group it is
`(*F).fields.type_repeat.min`. Union field access is fine inside `unsafe`.
`Llength`/`Loclength`/`Lcaseless`/`Lcaseopts`/`Lmatched_once`/`Lzero_allowed`/
`Lpositive` map to `(*F).byte1` / `(*F).byte2` (u8). `Loccu` is
`(*F).fields.char_repeat.oc.occu` (`[u8; 4]`, use `.as_mut_ptr()` where C uses
it as a pointer); `Loc` is `...oc.oc` (u32).
For other frames the C code writes e.g. `P->ovector[i]`: use
`*(*P).ovector.as_mut_ptr().add(i)`.

## Types and helpers

* `(*mb)` is a `match_block` (see `internal.rs` for exact field types:
  `hitend`/`hasthen`/`hasbsk`/`allowemptypartial`/`allowlookaroundbsk` are
  `BOOL` = i32, `partial`/`bsr_convention`/`name_count`/`name_entry_size` are
  u16, `moptions`/`poptions`/`nltype`/`nllen` are u32).
* Locals already declared in the skeleton (do not redeclare): `F`, `N`, `P`,
  `frames_top`, `assert_accept_frame`, `frame_copy_size`, `branch_end`,
  `branch_start`, `bracode`, `offset`, `length`, `rrc`, `proptype`, `i`, `fc`,
  `number`, `reptype`, `group_frame_type`, `condition`, `cur_is_word`,
  `prev_is_word`, `start_ecode`, `utf`, `ucp`, `state`, `frame_size`,
  `top_bracket`, `match_data`, `mb`. Block-local C variables (`uint32_t
  othercase;` etc.) become ordinary `let mut` inside your arm.
* File-local helper functions (defined in `src/pcre2_match_helpers.rs`):
  * `do_callout(F, mb, &mut length) -> c_int`
  * `match_ref(offset, caseless, caseopts, F, mb, &mut length) -> c_int`
  * `recurse_update_offsets(F, P)`
* Library functions: call them by their linker names, e.g. `_pcre2_ord2utf_8`,
  `_pcre2_extuni_8`, `_pcre2_is_newline_8`, `_pcre2_was_newline_8`,
  `_pcre2_xclass_8`, `_pcre2_eclass_8`, `_pcre2_script_run_8`,
  `_pcre2_strncmp_8`, `_pcre2_OP_lengths_8`, `UCD_*` accessors.
* `IS_NEWLINE(x)` in this file means `IS_NEWLINE!(x, mb, (*mb).end_subject, utf)`
  and `WAS_NEWLINE(x)` means `WAS_NEWLINE!(x, mb, (*mb).start_subject, utf)`.
* Pointer arithmetic: `Feptr++` -> `(*F).eptr = (*F).eptr.add(1)`,
  `Feptr[-1]` -> `*(*F).eptr.offset(-1)`, `Feptr - mb->start_subject` ->
  `(*F).eptr.offset_from((*mb).start_subject)` (i64; cast as needed).
* Table lookups that could be indexed out of range must go through
  `*TABLE.as_ptr().add(i)` (no bounds-check panic).

## Verification

Run:
```
cd $HARVEST_WORKDIR/translated_rust && cargo build --release 2>&1 | grep -E "^error" -A15 | head -80
```
Other agents are writing the other fragments at the same time, so **only fix
errors whose span points at your own file**. Iterate until your file is clean.
