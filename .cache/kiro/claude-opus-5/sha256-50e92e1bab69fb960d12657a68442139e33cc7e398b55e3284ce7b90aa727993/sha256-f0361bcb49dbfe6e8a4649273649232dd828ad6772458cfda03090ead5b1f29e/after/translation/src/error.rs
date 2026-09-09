//! Translation of `pcre2_error.c`.
//!
//! This module contains `pcre2_get_error_message()`, which copies an error
//! message into a caller-supplied buffer.

use crate::internal::*;
use core::ffi::c_int;

// The texts of compile-time error messages. Compile-time error numbers start
// at COMPILE_ERROR_BASE (100).
//
// This is one long string. Each substring ends with `\0` to insert a null
// character. This includes the final substring, so that the whole string ends
// with `\0\0`, which can be detected when counting through.
//
// XSTRING(PCRE2_CODE_UNIT_WIDTH) == "8"
// XSTRING(MAX_NAME_SIZE)         == "128"
// XSTRING(MAX_NAME_COUNT)        == "10000"
//
// EBCDIC is not defined, so error 68 uses the ASCII text and no on-the-fly
// EBCDIC translation is performed.
static compile_error_texts: &[u8] = b"\
no error\0\
\\ at end of pattern\0\
\\c at end of pattern\0\
unrecognized character follows \\\0\
numbers out of order in {} quantifier\0\
number too big in {} quantifier\0\
missing terminating ] for character class\0\
escape sequence is invalid in character class\0\
range out of order in character class\0\
quantifier does not follow a repeatable item\0\
internal error: unexpected repeat\0\
unrecognized character after (? or (?-\0\
POSIX named classes are supported only within a class\0\
POSIX collating elements are not supported\0\
missing closing parenthesis\0\
reference to non-existent subpattern\0\
pattern passed as NULL with non-zero length\0\
unrecognised compile-time option bit(s)\0\
missing ) after (?# comment\0\
parentheses are too deeply nested\0\
regular expression is too large\0\
failed to allocate heap memory\0\
unmatched closing parenthesis\0\
internal error: code overflow\0\
missing closing parenthesis for condition\0\
length of lookbehind assertion is not limited\0\
a relative value of zero is not allowed\0\
conditional subpattern contains more than two branches\0\
atomic assertion expected after (?( or (?(?C)\0\
digit expected after (?+\0\
unknown POSIX class name\0\
internal error in pcre2_study(): should not occur\0\
this version of PCRE2 does not have Unicode support\0\
parentheses are too deeply nested (stack check)\0\
character code point value in \\x{} or \\o{} is too large\0\
lookbehind is too complicated\0\
\\C is not allowed in a lookbehind assertion in UTF-8 mode\0\
PCRE2 does not support \\F, \\L, \\l, \\N{name}, \\U, or \\u\0\
number after (?C is greater than 255\0\
closing parenthesis for (?C expected\0\
invalid escape sequence in (*VERB) name\0\
unrecognized character after (?P\0\
syntax error in subpattern name (missing terminator?)\0\
two named subpatterns have the same name (PCRE2_DUPNAMES not set)\0\
subpattern name must start with a non-digit\0\
this version of PCRE2 does not have support for \\P, \\p, or \\X\0\
malformed \\P or \\p sequence\0\
unknown property after \\P or \\p\0\
subpattern name is too long (maximum 128 code units)\0\
too many named subpatterns (maximum 10000)\0\
invalid range in character class\0\
octal value is greater than \\377 in 8-bit non-UTF-8 mode\0\
internal error: overran compiling workspace\0\
internal error: previously-checked referenced subpattern not found\0\
DEFINE subpattern contains more than one branch\0\
missing opening brace after \\o\0\
internal error: unknown newline setting\0\
\\g is not followed by a braced, angle-bracketed, or quoted name/number or by a plain number\0\
(?R (recursive pattern call) must be followed by a closing parenthesis\0\
obsolete error (should not occur)\0\
(*VERB) not recognized or malformed\0\
subpattern number is too big\0\
subpattern name expected\0\
internal error: parsed pattern overflow\0\
non-octal character in \\o{} (closing brace missing?)\0\
different names for subpatterns of the same number are not allowed\0\
(*MARK) must have an argument\0\
non-hex character in \\x{} (closing brace missing?)\0\
\\c must be followed by a printable ASCII character\0\
\\k is not followed by a braced, angle-bracketed, or quoted name\0\
internal error: unknown meta code in check_lookbehinds()\0\
\\N is not supported in a class\0\
callout string is too long\0\
disallowed Unicode code point (>= 0xd800 && <= 0xdfff)\0\
using UTF is disabled by the application\0\
using UCP is disabled by the application\0\
name is too long in (*MARK), (*PRUNE), (*SKIP), or (*THEN)\0\
character code point value in \\u.... sequence is too large\0\
digits missing after \\x or in \\x{} or \\o{} or \\N{U+}\0\
syntax error or number too big in (?(VERSION condition\0\
internal error: unknown opcode in auto_possessify()\0\
missing terminating delimiter for callout with string argument\0\
unrecognized string delimiter follows (?C\0\
using \\C is disabled by the application\0\
(?| and/or (?J: or (?x: parentheses are too deeply nested\0\
using \\C is disabled in this PCRE2 library\0\
regular expression is too complicated\0\
lookbehind assertion is too long\0\
pattern string is longer than the limit set by the application\0\
internal error: unknown code in parsed pattern\0\
internal error: bad code value in parsed_skip()\0\
PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES is not allowed in UTF-16 mode\0\
invalid option bits with PCRE2_LITERAL\0\
\\N{U+dddd} is supported only in Unicode (UTF) mode\0\
invalid hyphen in option setting\0\
(*alpha_assertion) not recognized\0\
script runs require Unicode support, which this version of PCRE2 does not have\0\
too many capturing groups (maximum 65535)\0\
octal digit missing after \\0 (PCRE2_EXTRA_NO_BS0 is set)\0\
\\K is not allowed in lookarounds (but see PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK)\0\
branch too long in variable-length lookbehind assertion\0\
compiled pattern would be longer than the limit set by the application\0\
octal value given by \\ddd is greater than \\377 (forbidden by PCRE2_EXTRA_PYTHON_OCTAL)\0\
using callouts is disabled by the application\0\
PCRE2_EXTRA_TURKISH_CASING require Unicode (UTF or UCP) mode\0\
PCRE2_EXTRA_TURKISH_CASING requires UTF in 8-bit mode\0\
PCRE2_EXTRA_TURKISH_CASING and PCRE2_EXTRA_CASELESS_RESTRICT are not compatible\0\
extended character class nesting is too deep\0\
invalid operator in extended character class\0\
unexpected operator in extended character class (no preceding operand)\0\
expected operand after operator in extended character class\0\
square brackets needed to clarify operator precedence in extended character class\0\
missing terminating ] for extended character class (note '[' must be escaped under PCRE2_ALT_EXTENDED_CLASS)\0\
unexpected expression in extended character class (no preceding operator)\0\
empty expression in extended character class\0\
terminating ] with no following closing parenthesis in (?[...]\0\
unexpected character in (?[...]) extended character class\0\
expected capture group number or name\0\
missing opening parenthesis\0\
syntax error in subpattern number (missing terminator?)\0\
erroroffset passed as NULL\0\
\0";

// Match-time and UTF error texts are in the same format.
static match_error_texts: &[u8] = b"\
no error\0\
no match\0\
partial match\0\
UTF-8 error: 1 byte missing at end\0\
UTF-8 error: 2 bytes missing at end\0\
UTF-8 error: 3 bytes missing at end\0\
UTF-8 error: 4 bytes missing at end\0\
UTF-8 error: 5 bytes missing at end\0\
UTF-8 error: byte 2 top bits not 0x80\0\
UTF-8 error: byte 3 top bits not 0x80\0\
UTF-8 error: byte 4 top bits not 0x80\0\
UTF-8 error: byte 5 top bits not 0x80\0\
UTF-8 error: byte 6 top bits not 0x80\0\
UTF-8 error: 5-byte character is not allowed (RFC 3629)\0\
UTF-8 error: 6-byte character is not allowed (RFC 3629)\0\
UTF-8 error: code points greater than 0x10ffff are not defined\0\
UTF-8 error: code points 0xd800-0xdfff are not defined\0\
UTF-8 error: overlong 2-byte sequence\0\
UTF-8 error: overlong 3-byte sequence\0\
UTF-8 error: overlong 4-byte sequence\0\
UTF-8 error: overlong 5-byte sequence\0\
UTF-8 error: overlong 6-byte sequence\0\
UTF-8 error: isolated byte with 0x80 bit set\0\
UTF-8 error: illegal byte (0xfe or 0xff)\0\
UTF-16 error: missing low surrogate at end\0\
UTF-16 error: invalid low surrogate\0\
UTF-16 error: isolated low surrogate\0\
UTF-32 error: code points 0xd800-0xdfff are not defined\0\
UTF-32 error: code points greater than 0x10ffff are not defined\0\
bad data value\0\
patterns do not all use the same character tables\0\
magic number missing\0\
pattern compiled in wrong mode: 8/16/32-bit error\0\
bad offset value\0\
bad option value\0\
invalid replacement string\0\
bad offset into UTF string\0\
callout error code\0\
invalid data in workspace for DFA restart\0\
too much recursion for DFA matching\0\
backreference condition or recursion test is not supported for DFA matching\0\
function is not supported for DFA matching\0\
pattern contains an item that is not supported for DFA matching\0\
workspace size exceeded in DFA matching\0\
internal error - pattern overwritten?\0\
bad JIT option\0\
JIT stack limit reached\0\
match limit exceeded\0\
no more memory\0\
unknown substring\0\
non-unique substring name\0\
NULL argument passed with non-zero length\0\
nested recursion at the same subject position\0\
matching depth limit exceeded\0\
requested value is not available\0\
requested value is not set\0\
offset limit set without PCRE2_USE_OFFSET_LIMIT\0\
bad escape sequence in replacement string\0\
expected closing curly bracket in replacement string\0\
bad substitution in replacement string\0\
match with end before start or start moved backwards is not supported\0\
too many replacements (more than INT_MAX)\0\
bad serialized data\0\
heap limit exceeded\0\
invalid syntax\0\
internal error: duplicate substitution match\0\
PCRE2_MATCH_INVALID_UTF is not supported for DFA matching\0\
internal error: invalid substring offset\0\
feature is not supported by the JIT compiler\0\
error performing replacement case transformation\0\
replacement too large (longer than PCRE2_SIZE)\0\
substitute pattern differs from prior match call\0\
substitute subject differs from prior match call\0\
substitute start offset differs from prior match call\0\
substitute options differ from prior match call\0\
disallowed use of \\K in lookaround\0\
replacement $' or $_ not supported with partial match\0\
\0";

/// `pcre2_get_error_message()` — copy an error message into a buffer whose
/// units are of an appropriate width.
///
/// Arguments:
///   enumber       error number
///   buffer        where to put the message (zero terminated)
///   size          size of the buffer in code units
///
/// Returns:        length of message if all is well
///                 negative on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_error_message_8(
    enumber: c_int,
    buffer: *mut PCRE2_UCHAR,
    size: PCRE2_SIZE,
) -> c_int {
    unsafe {
        // Empty message list for invalid error numbers.
        // C: `(const unsigned char *)"\0"` — a 2-byte array (explicit NUL plus
        // the implicit string terminator), which the counting loop relies on.
        static EMPTY_MSG: &[u8] = b"\0\0";

        let mut i: PCRE2_SIZE;
        let mut n: c_int;
        let mut rc: c_int = 0;

        if size == 0 {
            return PCRE2_ERROR_NOMEMORY as c_int;
        }

        // `message` is an index into the chosen table.
        let table: &[u8];
        if enumber as i64 >= COMPILE_ERROR_BASE {
            // Compile error
            table = compile_error_texts;
            n = (enumber as i64 - COMPILE_ERROR_BASE) as c_int;
        } else if enumber < 0 {
            // Match or UTF error
            table = match_error_texts;
            n = -enumber;
        } else {
            // Invalid error number
            table = EMPTY_MSG;
            n = 1;
        }

        let mut message: usize = 0;

        // Count through to the wanted message. This mirrors:
        //   while (*message++ != CHAR_NUL) {}
        //   if (*message == CHAR_NUL) return PCRE2_ERROR_BADDATA;
        while n > 0 {
            while table[message] != 0 {
                message += 1;
            }
            message += 1; // step over the NUL (post-increment in C)
            if table[message] == 0 {
                return PCRE2_ERROR_BADDATA as c_int;
            }
            n -= 1;
        }

        i = 0;
        while table[message] != 0 {
            if i >= size - 1 {
                rc = PCRE2_ERROR_NOMEMORY as c_int;
                break;
            }
            *buffer.add(i) = table[message];
            message += 1;
            i += 1;
        }

        // EBCDIC is not defined, so no on-the-fly translation is performed.

        *buffer.add(i) = 0; // Terminate message, even if truncated.
        if rc != 0 { rc } else { i as c_int }
    }
}
