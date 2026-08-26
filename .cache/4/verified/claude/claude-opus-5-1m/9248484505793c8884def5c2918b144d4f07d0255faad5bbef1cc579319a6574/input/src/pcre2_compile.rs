// Translated from c_src/src/pcre2_compile.c
//
// The C file is very large, so the translation is split over several files that
// are textually included here (they all live in the same Rust module, exactly as
// they all live in the same C file).
use crate::internal::*;

/* Other debugging code can be enabled by these defines. */
/* (DEBUG_SHOW_PARSED / DEBUG_SHOW_OPS are not defined) */

/* There are a few things that vary with different code unit sizes. Handle them
by defining macros in order to minimize #if usage. */

pub const MAX_LABEL_TARGET: u32 = 0xffff;

/* Function definitions and tables */

include!("pcre2_compile_tables.rs"); /* c_src lines 95-835   */
include!("pcre2_compile_p1.rs"); /* c_src lines 1131-1258 */
include!("pcre2_compile_p2.rs"); /* c_src lines 1259-1488 */
include!("pcre2_compile_p3.rs"); /* c_src lines 1489-2257 */
include!("pcre2_compile_p4.rs"); /* c_src lines 2258-2730 */
include!("pcre2_compile_p5.rs"); /* c_src lines 2731-3111 */
include!("pcre2_compile_parse_regex.rs"); /* c_src lines 3112-5966 */
include!("pcre2_compile_p6.rs"); /* c_src lines 5967-6066 */
include!("pcre2_compile_branch.rs"); /* c_src lines 6067-8573 */
include!("pcre2_compile_p7.rs"); /* c_src lines 8574-8894 */
include!("pcre2_compile_p8.rs"); /* c_src lines 8895-9392 */
include!("pcre2_compile_p9.rs"); /* c_src lines 9393-10278 */
include!("pcre2_compile_p10.rs"); /* c_src lines 10279-end  */
