//! A translation of the PCRE2 8-bit library (`PCRE2_CODE_UNIT_WIDTH == 8`,
//! `SUPPORT_UNICODE` on, `SUPPORT_JIT` off) from C to Rust.
//!
//! Every public C symbol is re-exported with its final linker name, which for
//! this library means the `_8` suffix produced by the `PCRE2_SUFFIX` macro.
#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut,
    unused_parens,
    unused_unsafe,
    unused_variables
)]

pub mod consts;
pub mod internal;
pub mod tables;

pub mod auto_possess;
pub mod chkdint;
pub mod compile;
pub mod compile_h;
pub mod compile_tables;
pub mod compile_local;
pub mod compile_parse_util;
pub mod compile_parse;
pub mod compile_branch;
pub mod compile_aux;
pub mod compile_cgroup;
pub mod compile_class;
pub mod config;
pub mod context;
pub mod convert;
pub mod dfa_match;
pub mod error;
pub mod extuni;
pub mod find_bracket;
pub mod jit;
pub mod maketables;
pub mod match_data;
pub mod match_next;
pub mod match_local;
pub mod match_util;
pub mod match_core;
pub mod pcre2_match;
pub mod newline;
pub mod ord2utf;
pub mod pattern_info;
pub mod script_run;
pub mod serialize;
pub mod string_utils;
pub mod study;
pub mod substitute;
pub mod substring;
pub mod valid_utf;
pub mod xclass;

