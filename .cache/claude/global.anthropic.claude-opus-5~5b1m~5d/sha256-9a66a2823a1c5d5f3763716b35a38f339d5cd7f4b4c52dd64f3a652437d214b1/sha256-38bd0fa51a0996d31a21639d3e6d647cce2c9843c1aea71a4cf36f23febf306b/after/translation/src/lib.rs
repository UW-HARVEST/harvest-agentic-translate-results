//! A translation of the PCRE2 library (10.48-DEV, 8-bit mode, LINK_SIZE 2,
//! SUPPORT_UNICODE, no JIT) from C to Rust.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]
#![allow(unused_imports, unused_variables, unused_mut, unused_parens, dead_code)]
#![allow(unused_unsafe, unused_assignments, unused_labels)]

#[macro_use]
pub mod macros;
pub mod consts;
pub mod types;

pub mod auto_possess;
pub mod chartables;
pub mod chkdint;
pub mod compile;
pub mod match_engine;
pub mod dfa_internal;
pub mod compile_main;
pub mod compile_aux;
pub mod compile_branch;
pub mod compile_parse;
pub mod compile_tables;
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
pub mod matcher;
pub mod newline;
pub mod ord2utf;
pub mod pattern_info;
pub mod script_run;
pub mod serialize;
pub mod string_utils;
pub mod study;
pub mod substitute;
pub mod substring;
pub mod tables;
pub mod ucd;
pub mod valid_utf;
pub mod xclass;

