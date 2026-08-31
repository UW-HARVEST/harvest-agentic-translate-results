//! Translation of `c_src/src/pcre2_tables.c`: the shared data tables that the C
//! build exports as `_pcre2_*` symbols.
//!
//! The table contents live in the generated modules (`ucd`, `ucptables`,
//! `chartables`) and in `internal`; this module provides the exported C symbols.

use core::ffi::{c_char, c_int};

use crate::internal::{UcdRecord, UcpTypeTable};

/// Wrapper so that a `const char *` table can be a Rust `static`.
#[repr(transparent)]
pub struct CStrPtr(pub *const c_char);
unsafe impl Sync for CStrPtr {}

/* Table of sizes for the fixed-length opcodes. */
#[unsafe(no_mangle)]
pub static _pcre2_OP_lengths_8: [u8; 173] = crate::opcodes::OP_LENGTHS;

/* Tables of horizontal and vertical whitespace characters. */
#[unsafe(no_mangle)]
pub static _pcre2_hspace_list_8: [u32; 20] = crate::internal::HSPACE_LIST;
#[unsafe(no_mangle)]
pub static _pcre2_vspace_list_8: [u32; 8] = crate::internal::VSPACE_LIST;

/* Valid pairs of delimiters for callout string arguments. */
#[unsafe(no_mangle)]
pub static _pcre2_callout_start_delims_8: [u32; 9] = crate::internal::CALLOUT_START_DELIMS;
#[unsafe(no_mangle)]
pub static _pcre2_callout_end_delims_8: [u32; 9] = crate::internal::CALLOUT_END_DELIMS;

/* UTF-8 support tables. These are not width-suffixed in the C build. */
#[unsafe(no_mangle)]
pub static _pcre2_utf8_table1: [c_int; 6] = crate::internal::UTF8_TABLE1;
#[unsafe(no_mangle)]
pub static _pcre2_utf8_table1_size: u32 = crate::internal::UTF8_TABLE1_SIZE;
#[unsafe(no_mangle)]
pub static _pcre2_utf8_table2: [c_int; 6] = crate::internal::UTF8_TABLE2;
#[unsafe(no_mangle)]
pub static _pcre2_utf8_table3: [c_int; 6] = crate::internal::UTF8_TABLE3;
#[unsafe(no_mangle)]
pub static _pcre2_utf8_table4: [u8; 64] = crate::internal::UTF8_TABLE4;

/* Unicode property tables. */
#[unsafe(no_mangle)]
pub static _pcre2_ucp_gentype_8: [u32; 30] = crate::internal::UCP_GENTYPE;
#[unsafe(no_mangle)]
pub static _pcre2_ucp_gbtable_8: [u32; 15] = crate::internal::UCP_GBTABLE;

#[unsafe(no_mangle)]
pub static _pcre2_unicode_version_8: CStrPtr =
    CStrPtr(crate::ucd::UNICODE_VERSION.as_ptr() as *const c_char);

#[unsafe(no_mangle)]
pub static _pcre2_utt_8: [UcpTypeTable; crate::ucptables::UTT_SIZE] = crate::ucptables::UTT;
#[unsafe(no_mangle)]
pub static _pcre2_utt_names_8: [u8; 3834] = crate::ucptables::UTT_NAMES;
#[unsafe(no_mangle)]
pub static _pcre2_utt_size_8: usize = crate::ucptables::UTT_SIZE;

/* Unicode character database tables. */
#[unsafe(no_mangle)]
pub static _pcre2_ucd_records_8: [UcdRecord; 1563] = crate::ucd::UCD_RECORDS;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_stage1_8: [u16; 8704] = crate::ucd::UCD_STAGE1;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_stage2_8: [u16; 40192] = crate::ucd::UCD_STAGE2;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_caseless_sets_8: [u32; 118] = crate::ucd::UCD_CASELESS_SETS;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_nocase_ranges_8: [u32; 84] = crate::ucd::UCD_NOCASE_RANGES;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_nocase_ranges_size_8: u32 = crate::ucd::UCD_NOCASE_RANGES_SIZE;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_digit_sets_8: [u32; 78] = crate::ucd::UCD_DIGIT_SETS;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_script_sets_8: [u32; 476] = crate::ucd::UCD_SCRIPT_SETS;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_boolprop_sets_8: [u32; 382] = crate::ucd::UCD_BOOLPROP_SETS;
#[unsafe(no_mangle)]
pub static _pcre2_ucd_turkish_dotted_i_caseset_8: u32 =
    crate::ucd::UCD_TURKISH_DOTTED_I_CASESET;
