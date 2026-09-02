//! Translation of PART 1 of `pcre2_match.c` (C lines 45-683).
//!
//! This module contains:
//!   * the two file-`static` helpers `do_callout` (C line 268) and
//!     `match_ref` (C line 357), translated as `pub(crate) unsafe fn`s
//!     (they are `static` in C, so no `#[no_mangle]` / `extern "C"`);
//!   * the frame-field accessor macros (`Fxxx`, C lines 165-179) that give
//!     short names for fields of the current frame `F` (a `*mut heapframe`);
//!   * the localized `fields` union accessor macros (`Lxxx`, defined at their
//!     points of use in the big `match()` switch, C lines 1294-6231);
//!   * the partial-match macros `CHECK_PARTIAL` / `SCHECK_PARTIAL`
//!     (C lines 614-632).
//!
//! Everything here is `pub(crate)` so that `src/match_core.rs` (the big
//! `match()` function) can call it.

use crate::internal::*;
use core::ffi::c_int;

// ===========================================================================
// Frame-field accessor macros (C lines 165-179)
// ===========================================================================
//
// C defines, e.g. `#define Fecode F->ecode`.  These are used both for reading
// and for writing the field, so each is exposed here as a raw-pointer accessor
// (`&raw mut (*F).field`) and callers deref it to read or write:
//     *Fecode(F)            // read
//     *Fecode(F) = value;   // write
//
// The union-member (`fields.*`) shorthands (`Lxxx`) follow further below.

/// `#define Fback_frame F->back_frame`
#[inline(always)]
pub(crate) unsafe fn Fback_frame(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).back_frame }
}

/// `#define Fcapture_last F->capture_last`
#[inline(always)]
pub(crate) unsafe fn Fcapture_last(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).capture_last }
}

/// `#define Fcurrent_recurse F->current_recurse`
#[inline(always)]
pub(crate) unsafe fn Fcurrent_recurse(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).current_recurse }
}

/// `#define Fecode F->ecode`
#[inline(always)]
pub(crate) unsafe fn Fecode(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).ecode }
}

/// `#define Feptr F->eptr`
#[inline(always)]
pub(crate) unsafe fn Feptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).eptr }
}

/// `#define Fgroup_frame_type F->group_frame_type`
#[inline(always)]
pub(crate) unsafe fn Fgroup_frame_type(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).group_frame_type }
}

/// `#define Flast_group_offset F->last_group_offset`
#[inline(always)]
pub(crate) unsafe fn Flast_group_offset(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).last_group_offset }
}

/// `#define Fmark F->mark`
#[inline(always)]
pub(crate) unsafe fn Fmark(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).mark }
}

/// `#define Frdepth F->rdepth`
#[inline(always)]
pub(crate) unsafe fn Frdepth(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).rdepth }
}

/// `#define Fstart_match F->start_match`
#[inline(always)]
pub(crate) unsafe fn Fstart_match(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).start_match }
}

/// `#define Foffset_top F->offset_top`
#[inline(always)]
pub(crate) unsafe fn Foffset_top(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).offset_top }
}

/// `#define Fop F->op`
#[inline(always)]
pub(crate) unsafe fn Fop(F: *mut heapframe) -> *mut u8 {
    unsafe { &raw mut (*F).op }
}

/// `#define Fovector F->ovector`
///
/// In C this is the flexible array member `F->ovector`; the macro decays to a
/// `PCRE2_SIZE *` pointing at the first element.  We return that pointer.
#[inline(always)]
pub(crate) unsafe fn Fovector(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { (*F).ovector.as_mut_ptr() }
}

/// `#define Freturn_id F->return_id`
#[inline(always)]
pub(crate) unsafe fn Freturn_id(F: *mut heapframe) -> *mut u8 {
    unsafe { &raw mut (*F).return_id }
}

// The two spare temporary bytes in the frame are aliased by several `Lxxx`
// macros (e.g. `Lcaseless == F->byte1`, `Lcaseopts == F->byte2`).  Expose them
// directly so match_core.rs can reproduce those definitions.

/// `F->byte1` (aliased by `Llength` @5243 via byte1 in some ops, `Lcaseless`
/// @5246, `Lmatched_once` @5531, `Lpositive` @6006, ...).
#[inline(always)]
pub(crate) unsafe fn Fbyte1(F: *mut heapframe) -> *mut u8 {
    unsafe { &raw mut (*F).byte1 }
}

/// `F->byte2` (aliased by `Loclength` @1295, `Lcaseopts` @5247,
/// `Lzero_allowed` @5532, ...).
#[inline(always)]
pub(crate) unsafe fn Fbyte2(F: *mut heapframe) -> *mut u8 {
    unsafe { &raw mut (*F).byte2 }
}

// ===========================================================================
// Localized `fields` union accessor macros (Lxxx)
// ===========================================================================
//
// In C the same short name (e.g. `Lmin`) is `#define`d and `#undef`d many
// times through the big switch, each time aliasing a *different* union member
// at a *different* offset (see the table below).  A single global `Lmin`
// cannot be sound, so we expose one raw-pointer accessor per (union member,
// field).  match_core.rs selects the correct one for the opcode it is
// handling.  Each returns `&raw mut` of the field so callers can read or
// write.
//
//   union member        C `Lxxx` names (line)
//   ------------------   ---------------------------------------------------
//   char_repeat          Lstart_eptr/Lcharptr/Lmin/Lmax/Lc/Loc/Loccu (1296-)
//   charnot_repeat       Lstart_eptr/Lmin/Lmax/Lc/Loc                (1654-)
//   class_repeat         Lbyte_map_address/Lstart_eptr/Lmin/Lmax     (2045-)
//   xclass_repeat        Lstart_eptr/Lxclass_data/Lmin/Lmax          (2288-)
//   eclass_repeat        Lstart_eptr/Leclass_data/Leclass_len/..     (2429-)
//   type_repeat          Lstart_eptr/Lmin/Lmax/Lctype/Lpropvalue     (2913-)
//   ref_repeat           Lstart/Loffset/Llength/Lmin/Lmax            (5241-)
//   op_brapos            Lstart_eptr/Lstart_group/Lframe_type        (5528-)
//   op_bra               Lframe_type                                 (5620-)
//   op_recurse           Lstart_branch/Lframe_type                   (5704-)
//   op_assert_scs        Lsaved_end_subject/Lsaved_eptr/..           (5861-)
//   op_cond              Lstart_branch/Llength                       (6004-)
//   op_vreverse          Lmin/Lmax                                   (6230-)

// --- char_repeat (C 1296-1302) ---------------------------------------------

/// `Lstart_eptr` @1296 = `F->fields.char_repeat.start_eptr`.
#[inline(always)]
pub(crate) unsafe fn Lchar_start_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.char_repeat.start_eptr }
}
/// `Lcharptr` @1297 = `F->fields.char_repeat.charptr`.
#[inline(always)]
pub(crate) unsafe fn Lcharptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.char_repeat.charptr }
}
/// `Lmin` @1298 = `F->fields.char_repeat.min`.
#[inline(always)]
pub(crate) unsafe fn Lchar_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.char_repeat.min }
}
/// `Lmax` @1299 = `F->fields.char_repeat.max`.
#[inline(always)]
pub(crate) unsafe fn Lchar_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.char_repeat.max }
}
/// `Lc` @1300 = `F->fields.char_repeat.c`.
#[inline(always)]
pub(crate) unsafe fn Lchar_c(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.char_repeat.c }
}
/// `Loc` @1301 = `F->fields.char_repeat.oc.oc`.
#[inline(always)]
pub(crate) unsafe fn Lchar_oc(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.char_repeat.oc.oc }
}
/// `Loccu` @1302 = `F->fields.char_repeat.oc.occu` (a.k.a. `Locchars`, the
/// other-case code-unit buffer). Returns a pointer to the first element.
#[inline(always)]
pub(crate) unsafe fn Loccu(F: *mut heapframe) -> *mut PCRE2_UCHAR {
    unsafe { (*F).fields.char_repeat.oc.occu.as_mut_ptr() }
}

// --- charnot_repeat (C 1654-1658) ------------------------------------------

/// `Lstart_eptr` @1654 = `F->fields.charnot_repeat.start_eptr`.
#[inline(always)]
pub(crate) unsafe fn Lcharnot_start_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.charnot_repeat.start_eptr }
}
/// `Lmin` @1655 = `F->fields.charnot_repeat.min`.
#[inline(always)]
pub(crate) unsafe fn Lcharnot_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.charnot_repeat.min }
}
/// `Lmax` @1656 = `F->fields.charnot_repeat.max`.
#[inline(always)]
pub(crate) unsafe fn Lcharnot_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.charnot_repeat.max }
}
/// `Lc` @1657 = `F->fields.charnot_repeat.c`.
#[inline(always)]
pub(crate) unsafe fn Lcharnot_c(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.charnot_repeat.c }
}
/// `Loc` @1658 = `F->fields.charnot_repeat.oc`.
#[inline(always)]
pub(crate) unsafe fn Lcharnot_oc(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.charnot_repeat.oc }
}

// --- class_repeat (C 2045-2049) --------------------------------------------

/// `Lbyte_map_address` @2045 = `F->fields.class_repeat.byte_map_address`.
#[inline(always)]
pub(crate) unsafe fn Lbyte_map_address(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.class_repeat.byte_map_address }
}
/// `Lstart_eptr` @2047 = `F->fields.class_repeat.start_eptr`.
#[inline(always)]
pub(crate) unsafe fn Lclass_start_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.class_repeat.start_eptr }
}
/// `Lmin` @2048 = `F->fields.class_repeat.min`.
#[inline(always)]
pub(crate) unsafe fn Lclass_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.class_repeat.min }
}
/// `Lmax` @2049 = `F->fields.class_repeat.max`.
#[inline(always)]
pub(crate) unsafe fn Lclass_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.class_repeat.max }
}

// --- xclass_repeat (C 2288-2291) -------------------------------------------

/// `Lstart_eptr` @2288 = `F->fields.xclass_repeat.start_eptr`.
#[inline(always)]
pub(crate) unsafe fn Lxclass_start_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.xclass_repeat.start_eptr }
}
/// `Lxclass_data` @2289 = `F->fields.xclass_repeat.xclass_data`.
#[inline(always)]
pub(crate) unsafe fn Lxclass_data(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.xclass_repeat.xclass_data }
}
/// `Lmin` @2290 = `F->fields.xclass_repeat.min`.
#[inline(always)]
pub(crate) unsafe fn Lxclass_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.xclass_repeat.min }
}
/// `Lmax` @2291 = `F->fields.xclass_repeat.max`.
#[inline(always)]
pub(crate) unsafe fn Lxclass_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.xclass_repeat.max }
}

// --- eclass_repeat (C 2429-2433) -------------------------------------------

/// `Lstart_eptr` @2429 = `F->fields.eclass_repeat.start_eptr`.
#[inline(always)]
pub(crate) unsafe fn Leclass_start_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.eclass_repeat.start_eptr }
}
/// `Leclass_data` @2430 = `F->fields.eclass_repeat.eclass_data`.
#[inline(always)]
pub(crate) unsafe fn Leclass_data(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.eclass_repeat.eclass_data }
}
/// `Leclass_len` @2431 = `F->fields.eclass_repeat.eclass_len`.
#[inline(always)]
pub(crate) unsafe fn Leclass_len(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).fields.eclass_repeat.eclass_len }
}
/// `Lmin` @2432 = `F->fields.eclass_repeat.min`.
#[inline(always)]
pub(crate) unsafe fn Leclass_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.eclass_repeat.min }
}
/// `Lmax` @2433 = `F->fields.eclass_repeat.max`.
#[inline(always)]
pub(crate) unsafe fn Leclass_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.eclass_repeat.max }
}

// --- type_repeat (C 2913-2917) ---------------------------------------------

/// `Lstart_eptr` @2913 = `F->fields.type_repeat.start_eptr`.
#[inline(always)]
pub(crate) unsafe fn Ltype_start_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.type_repeat.start_eptr }
}
/// `Lmin` @2914 = `F->fields.type_repeat.min`.
#[inline(always)]
pub(crate) unsafe fn Ltype_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.type_repeat.min }
}
/// `Lmax` @2915 = `F->fields.type_repeat.max`.
#[inline(always)]
pub(crate) unsafe fn Ltype_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.type_repeat.max }
}
/// `Lctype` @2916 = `F->fields.type_repeat.ctype`.
#[inline(always)]
pub(crate) unsafe fn Lctype(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.type_repeat.ctype }
}
/// `Lpropvalue` @2917 = `F->fields.type_repeat.propvalue`.
#[inline(always)]
pub(crate) unsafe fn Lpropvalue(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.type_repeat.propvalue }
}

// --- ref_repeat (C 5241-5245) ----------------------------------------------

/// `Lstart` @5241 = `F->fields.ref_repeat.start`.
#[inline(always)]
pub(crate) unsafe fn Lstart(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.ref_repeat.start }
}
/// `Loffset` @5242 = `F->fields.ref_repeat.offset`.
#[inline(always)]
pub(crate) unsafe fn Loffset(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).fields.ref_repeat.offset }
}
/// `Llength` @5243 = `F->fields.ref_repeat.length`.
#[inline(always)]
pub(crate) unsafe fn Lref_length(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).fields.ref_repeat.length }
}
/// `Lmin` @5244 = `F->fields.ref_repeat.min`.
#[inline(always)]
pub(crate) unsafe fn Lref_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.ref_repeat.min }
}
/// `Lmax` @5245 = `F->fields.ref_repeat.max`.
#[inline(always)]
pub(crate) unsafe fn Lref_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.ref_repeat.max }
}

// --- op_brapos (C 5528-5530) -----------------------------------------------

/// `Lstart_eptr` @5528 = `F->fields.op_brapos.start_eptr`.
#[inline(always)]
pub(crate) unsafe fn Lbrapos_start_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.op_brapos.start_eptr }
}
/// `Lstart_group` @5529 = `F->fields.op_brapos.start_group`.
#[inline(always)]
pub(crate) unsafe fn Lstart_group(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.op_brapos.start_group }
}
/// `Lframe_type` @5530 = `F->fields.op_brapos.frame_type`.
#[inline(always)]
pub(crate) unsafe fn Lbrapos_frame_type(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.op_brapos.frame_type }
}

// --- op_bra (C 5620) -------------------------------------------------------

/// `Lframe_type` @5620 = `F->fields.op_bra.frame_type`.
#[inline(always)]
pub(crate) unsafe fn Lbra_frame_type(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.op_bra.frame_type }
}

// --- op_recurse (C 5704-5705) ----------------------------------------------

/// `Lstart_branch` @5704 = `F->fields.op_recurse.start_branch`.
#[inline(always)]
pub(crate) unsafe fn Lrecurse_start_branch(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.op_recurse.start_branch }
}
/// `Lframe_type` @5705 = `F->fields.op_recurse.frame_type`.
#[inline(always)]
pub(crate) unsafe fn Lrecurse_frame_type(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.op_recurse.frame_type }
}

// --- op_assert_scs (C 5861-5865) -------------------------------------------

/// `Lsaved_end_subject` @5861 = `F->fields.op_assert_scs.saved_end_subject`.
#[inline(always)]
pub(crate) unsafe fn Lsaved_end_subject(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.op_assert_scs.saved_end_subject }
}
/// `Lsaved_eptr` @5862 = `F->fields.op_assert_scs.saved_eptr`.
#[inline(always)]
pub(crate) unsafe fn Lsaved_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.op_assert_scs.saved_eptr }
}
/// `Ltrue_end_extra` @5863 = `F->fields.op_assert_scs.true_end_extra`.
#[inline(always)]
pub(crate) unsafe fn Ltrue_end_extra(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).fields.op_assert_scs.true_end_extra }
}
/// `Lsaved_moptions` @5865 = `F->fields.op_assert_scs.saved_moptions`.
#[inline(always)]
pub(crate) unsafe fn Lsaved_moptions(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.op_assert_scs.saved_moptions }
}

// --- op_cond (C 6004-6005) -------------------------------------------------

/// `Lstart_branch` @6004 = `F->fields.op_cond.start_branch`.
#[inline(always)]
pub(crate) unsafe fn Lcond_start_branch(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { &raw mut (*F).fields.op_cond.start_branch }
}
/// `Llength` @6005 = `F->fields.op_cond.length`.
#[inline(always)]
pub(crate) unsafe fn Lcond_length(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { &raw mut (*F).fields.op_cond.length }
}

// --- op_vreverse (C 6230-6231) ---------------------------------------------

/// `Lmin` @6230 = `F->fields.op_vreverse.min`.
#[inline(always)]
pub(crate) unsafe fn Lvreverse_min(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.op_vreverse.min }
}
/// `Lmax` @6231 = `F->fields.op_vreverse.max`.
#[inline(always)]
pub(crate) unsafe fn Lvreverse_max(F: *mut heapframe) -> *mut u32 {
    unsafe { &raw mut (*F).fields.op_vreverse.max }
}

// ---------------------------------------------------------------------------
// Convenience aliases for the un-suffixed `Lxxx` names requested in the task.
//
// These are the "primary" spellings; where a C name is overloaded across
// several union members the alias points at the first (char/ref) member and
// the member-suffixed accessors above cover the rest.
// ---------------------------------------------------------------------------

/// `Lstart_eptr` — alias for the `char_repeat` member (the first user).
#[inline(always)]
pub(crate) unsafe fn Lstart_eptr(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { Lchar_start_eptr(F) }
}
/// `Lmin` — alias for the `char_repeat` member.
#[inline(always)]
pub(crate) unsafe fn Lmin(F: *mut heapframe) -> *mut u32 {
    unsafe { Lchar_min(F) }
}
/// `Lmax` — alias for the `char_repeat` member.
#[inline(always)]
pub(crate) unsafe fn Lmax(F: *mut heapframe) -> *mut u32 {
    unsafe { Lchar_max(F) }
}
/// `Lc` — alias for the `char_repeat` member.
#[inline(always)]
pub(crate) unsafe fn Lc(F: *mut heapframe) -> *mut u32 {
    unsafe { Lchar_c(F) }
}
/// `Loc` — alias for the `char_repeat` member.
#[inline(always)]
pub(crate) unsafe fn Loc(F: *mut heapframe) -> *mut u32 {
    unsafe { Lchar_oc(F) }
}
/// `Locchars` — the other-case code-unit buffer (C `Loccu` / `occu`).
#[inline(always)]
pub(crate) unsafe fn Locchars(F: *mut heapframe) -> *mut PCRE2_UCHAR {
    unsafe { Loccu(F) }
}
/// `Lframe_type` — alias for the `op_bra` member.
#[inline(always)]
pub(crate) unsafe fn Lframe_type(F: *mut heapframe) -> *mut u32 {
    unsafe { Lbra_frame_type(F) }
}
/// `Lstart_branch` — alias for the `op_recurse` member.
#[inline(always)]
pub(crate) unsafe fn Lstart_branch(F: *mut heapframe) -> *mut PCRE2_SPTR {
    unsafe { Lrecurse_start_branch(F) }
}
/// `Llength` — alias for the `ref_repeat` member.
#[inline(always)]
pub(crate) unsafe fn Llength(F: *mut heapframe) -> *mut PCRE2_SIZE {
    unsafe { Lref_length(F) }
}

// ===========================================================================
// Partial-match macros (C lines 614-632)
// ===========================================================================

/// `SCHECK_PARTIAL()` (C lines 623-632).
///
/// Used when we already know we are at/past the end of the subject. Sets
/// `mb->hitend = TRUE` when partial matching is enabled and (the pointer is
/// past the earliest inspected character or empty partials are allowed); for
/// hard partial matching (`mb->partial > 1`) it causes the caller to
/// `return PCRE2_ERROR_PARTIAL`.
///
/// Returns `Some(PCRE2_ERROR_PARTIAL)` when the caller must return that error,
/// otherwise `None`. Intended use:
/// ```ignore
/// if let Some(r) = SCHECK_PARTIAL(F, mb) { return r; }
/// ```
#[inline(always)]
pub(crate) unsafe fn SCHECK_PARTIAL(F: *mut heapframe, mb: *mut match_block) -> Option<c_int> {
    unsafe {
        if (*mb).partial != 0
            && (*Feptr(F) > (*mb).start_used_ptr || (*mb).allowemptypartial != 0)
        {
            (*mb).hitend = TRUE;
            if (*mb).partial > 1 {
                return Some(PCRE2_ERROR_PARTIAL as c_int);
            }
        }
        None
    }
}

/// `CHECK_PARTIAL()` (C lines 614-621).
///
/// If `Feptr >= mb->end_subject`, defer to [`SCHECK_PARTIAL`]. Returns
/// `Some(PCRE2_ERROR_PARTIAL)` when the caller must return that error,
/// otherwise `None`. Intended use:
/// ```ignore
/// if let Some(r) = CHECK_PARTIAL(F, mb) { return r; }
/// ```
#[inline(always)]
pub(crate) unsafe fn CHECK_PARTIAL(F: *mut heapframe, mb: *mut match_block) -> Option<c_int> {
    unsafe {
        if *Feptr(F) >= (*mb).end_subject {
            return SCHECK_PARTIAL(F, mb);
        }
        None
    }
}

// ===========================================================================
// do_callout  (C lines 268-327)
// ===========================================================================

/// Process a callout (C `static int do_callout(...)`, line 268).
///
/// Called for all callouts, whether "standalone" or at the start of a
/// conditional group. `Feptr` points to either `OP_CALLOUT` or
/// `OP_CALLOUT_STR`. A callout block is allocated in `pcre2_match()` and
/// initialized with fixed values.
///
/// Returns the value returned by the user callout, or 0 if no callout function
/// exists.
pub(crate) unsafe fn do_callout(
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let rc: c_int;
        let save0: PCRE2_SIZE;
        let save1: PCRE2_SIZE;

        let fecode = *Fecode(F);

        *lengthptr = if *fecode as u32 == OP_CALLOUT {
            crate::tables::_pcre2_OP_lengths_8[OP_CALLOUT as usize] as PCRE2_SIZE
        } else {
            GET(fecode, (1 + 2 * LINK_SIZE) as usize) as PCRE2_SIZE
        };

        if (*mb).callout.is_none() {
            return 0; /* No callout function provided */
        }

        /* The working ovector is in the backtracking frame; for backward
        compatibility we pass capture_top and offset_vector to the callout as
        if for the extended ovector, ensuring the first two slots are unset by
        preserving and restoring their current contents. Fovector[-2]. */

        let callout_ovector: *mut PCRE2_SIZE = (Fovector(F) as *mut PCRE2_SIZE).sub(2);

        let cb: *mut pcre2_callout_block = (*mb).cb;
        (*cb).capture_top = (*Foffset_top(F) as u32) / 2 + 1;
        (*cb).capture_last = *Fcapture_last(F);
        (*cb).offset_vector = callout_ovector;
        (*cb).mark = (*mb).nomatch_mark;
        (*cb).current_position = (*Feptr(F)).offset_from((*mb).start_subject) as PCRE2_SIZE;
        (*cb).pattern_position = GET(fecode, 1) as PCRE2_SIZE;
        (*cb).next_item_length = GET(fecode, (1 + LINK_SIZE) as usize) as PCRE2_SIZE;

        if *fecode as u32 == OP_CALLOUT
        /* Numerical callout */
        {
            (*cb).callout_number = *fecode.add((1 + 2 * LINK_SIZE) as usize) as u32;
            (*cb).callout_string_offset = 0;
            (*cb).callout_string = core::ptr::null();
            (*cb).callout_string_length = 0;
        } else
        /* String callout */
        {
            (*cb).callout_number = 0;
            (*cb).callout_string_offset = GET(fecode, (1 + 3 * LINK_SIZE) as usize) as PCRE2_SIZE;
            (*cb).callout_string = fecode.add((1 + 4 * LINK_SIZE) as usize + 1);
            (*cb).callout_string_length =
                *lengthptr - (1 + 4 * LINK_SIZE) as PCRE2_SIZE - 2;
        }

        save0 = *callout_ovector.add(0);
        save1 = *callout_ovector.add(1);
        *callout_ovector.add(0) = PCRE2_UNSET;
        *callout_ovector.add(1) = PCRE2_UNSET;
        rc = ((*mb).callout.unwrap())(cb, (*mb).callout_data);
        *callout_ovector.add(0) = save0;
        *callout_ovector.add(1) = save1;
        (*cb).callout_flags = 0;
        rc
    }
}

// ===========================================================================
// match_ref  (C lines 357-501)
// ===========================================================================

/// Match a back-reference (C `static int match_ref(...)`, line 357).
///
/// Called only when it is known that the offset lies within the offsets used
/// so far in the match. In caseless UTF-8 mode the number of subject bytes
/// matched may differ from the number of reference bytes.
///
/// Arguments:
///   * `offset`    — index into the offset vector
///   * `caseless`  — `TRUE` if caseless
///   * `caseopts`  — bitmask of `REFI_FLAG_XYZ` values
///   * `F`         — current backtracking frame pointer
///   * `mb`        — match block
///   * `lengthptr` — out: number of code units matched
///
/// Returns: `0` = successful match (length set); `< 0` = no match;
/// `> 0` = partial match.
pub(crate) unsafe fn match_ref(
    offset: PCRE2_SIZE,
    caseless: BOOL,
    caseopts: c_int,
    F: *mut heapframe,
    mb: *mut match_block,
    lengthptr: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut p: PCRE2_SPTR;
        let mut length: PCRE2_SIZE;
        let mut eptr: PCRE2_SPTR;
        let eptr_start: PCRE2_SPTR;

        let ovector = Fovector(F);

        /* Deal with an unset group. Default is no match, unless the option to
        match an empty string is set. */
        if offset >= *Foffset_top(F) || *ovector.add(offset) == PCRE2_UNSET {
            if ((*mb).poptions & PCRE2_MATCH_UNSET_BACKREF as u32) != 0 {
                *lengthptr = 0;
                return 0; /* Match */
            } else {
                return -1; /* No match */
            }
        }

        /* Separate the caseless and UTF cases for speed. */

        eptr = *Feptr(F);
        eptr_start = eptr;
        p = (*mb).start_subject.add(*ovector.add(offset));
        length = *ovector.add(offset + 1) - *ovector.add(offset);
        debug_assert!(eptr <= (*mb).end_subject);

        if caseless != 0 {
            // SUPPORT_UNICODE branch.
            let utf = ((*mb).poptions & PCRE2_UTF as u32) != 0;
            let caseless_restrict = (caseopts & REFI_FLAG_CASELESS_RESTRICT as c_int) != 0;
            let turkish_casing =
                !caseless_restrict && (caseopts & REFI_FLAG_TURKISH_CASING as c_int) != 0;

            if utf || ((*mb).poptions & PCRE2_UCP as u32) != 0 {
                let endptr: PCRE2_SPTR = p.add(length);

                /* Match characters up to the end of the reference. NOTE: the
                number of code units matched may differ. It is important to
                check the length along the reference, not along the subject. */
                while p < endptr {
                    let c: u32;
                    let d: u32;
                    if eptr >= (*mb).end_subject {
                        return 1; /* Partial match */
                    }

                    if utf {
                        c = GETCHARINC(&mut eptr);
                        d = GETCHARINC(&mut p);
                    } else {
                        c = *eptr as u32;
                        eptr = eptr.add(1);
                        d = *p as u32;
                        p = p.add(1);
                    }

                    if turkish_casing && UCD_ANY_I(d) {
                        let c2 = UCD_FOLD_I_TURKISH(c);
                        let d2 = UCD_FOLD_I_TURKISH(d);
                        if c2 != d2 {
                            return -1; /* No match */
                        }
                    } else {
                        let ur = GET_UCD(d);
                        if c != d && c != ((d as i32 + ur.other_case) as u32) {
                            let mut pp: *const u32 = crate::tables::_pcre2_ucd_caseless_sets_8
                                .as_ptr()
                                .add(ur.caseset as usize);

                            /* When PCRE2_EXTRA_CASELESS_RESTRICT is set, ignore
                            any caseless sets that start with an ASCII char. */
                            if caseless_restrict && *pp < 128 {
                                return -1; /* No match */
                            }

                            loop {
                                if c < *pp {
                                    return -1; /* No match */
                                }
                                let cur = *pp;
                                pp = pp.add(1);
                                if c == cur {
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                /* Not in UTF or UCP mode */
                while length > 0 {
                    let cc: u32;
                    let cp: u32;
                    if eptr >= (*mb).end_subject {
                        return 1; /* Partial match */
                    }
                    cc = *eptr as u32;
                    cp = *p as u32;
                    // TABLE_GET(x, mb->lcc, x) == mb->lcc[x]
                    if *(*mb).lcc.add(cp as usize) != *(*mb).lcc.add(cc as usize) {
                        return -1; /* No match */
                    }
                    p = p.add(1);
                    eptr = eptr.add(1);
                    length -= 1;
                }
            }
        } else {
            /* Caseful: compare code units directly. When partial matching,
            unit by unit. */
            if (*mb).partial != 0 {
                while length > 0 {
                    if eptr >= (*mb).end_subject {
                        return 1; /* Partial match */
                    }
                    let pc = *p;
                    p = p.add(1);
                    let ec = *eptr;
                    eptr = eptr.add(1);
                    if pc != ec {
                        return -1; /* No match */
                    }
                    length -= 1;
                }
            } else {
                /* Not partial matching */
                // CU2BYTES(length) == length for 8-bit code units.
                if ((*mb).end_subject.offset_from(eptr) as PCRE2_SIZE) < length
                    || libc_memcmp(p, eptr, length) != 0
                {
                    return -1; /* No match */
                }
                eptr = eptr.add(length);
            }
        }

        *lengthptr = eptr.offset_from(eptr_start) as PCRE2_SIZE;
        0 /* Match */
    }
}

// ---------------------------------------------------------------------------
// Small private helpers used above.
// ---------------------------------------------------------------------------

/// `UCD_ANY_I(ch)` (pcre2_internal.h): true for 'i', 'I', U+0130, U+0131.
#[inline(always)]
fn UCD_ANY_I(ch: u32) -> bool {
    (ch | 0x20u32) == 0x69u32 || (ch | 1u32) == 0x0131u32
}

/// `UCD_FOLD_I_TURKISH(ch)` (pcre2_internal.h).
#[inline(always)]
fn UCD_FOLD_I_TURKISH(ch: u32) -> u32 {
    if ch == 0x0130u32 {
        0x69u32
    } else if ch == 0x49u32 {
        0x0131u32
    } else {
        ch
    }
}

/// `memcmp(a, b, n)` over `n` code units (bytes in 8-bit mode). Returns 0 when
/// equal, matching C `memcmp`'s zero/non-zero semantics as used by `match_ref`.
#[inline(always)]
unsafe fn libc_memcmp(a: PCRE2_SPTR, b: PCRE2_SPTR, n: PCRE2_SIZE) -> c_int {
    unsafe {
        let sa = core::slice::from_raw_parts(a, n);
        let sb = core::slice::from_raw_parts(b, n);
        if sa == sb {
            0
        } else {
            1
        }
    }
}
