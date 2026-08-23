// Translated from c_src/src/pcre2_compile_class.c
use crate::internal::*;

#[unsafe(no_mangle)]
pub static _pcre2_posix_class_maps8: [i32; 42] = [
  160, 64, -2, 128, -1, 0,
  96, -1, 0, 160, -1, 2,
  224, 288, 0, 0, -1, 1,
  288, -1, 0, 64, -1, 0,
  192, -1, 0, 224, -1, 0,
  256, -1, 0, 0, -1, 0,
  160, -1, 0, 32, -1, 0,
];

include!("pcre2_compile_class_p1.rs"); /* c_src lines 45-750    */
include!("pcre2_compile_class_p2.rs"); /* c_src lines 751-1071  */
include!("pcre2_compile_class_p3.rs"); /* c_src lines 1072-1879 */
include!("pcre2_compile_class_p4.rs"); /* c_src lines 1880-end  */
