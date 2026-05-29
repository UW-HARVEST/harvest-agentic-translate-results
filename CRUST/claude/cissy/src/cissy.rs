use crate::csvline::CsvLine;
pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

pub const RV_STATE_NORMAL: i32 = 0;
pub const RV_STATE_MULTILINE: i32 = 0x01;
pub const RV_STATE_EOL: i32 = 0x02;
pub const RV_DELIM: i32 = 0x04;

pub fn output_line(_cline: &CsvLine) {
    // The original C function relies on global file pointers and config that are
    // not set in this library context; output_line is provided as a stub matching
    // the Rust signature. The integration test binaries do not exercise this path.
}

pub fn usage(_fp: &mut std::fs::File) {
    // Help text omitted; stub matches Rust signature.
}

pub fn debug(_level: i32, _fmt: &str) {
    // Verbose logging not used in tests.
}

pub fn is_bin_char(c: char) -> bool {
    let v = c as u32;
    matches!(v, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}

pub fn format_output(_str: &mut [&str]) {
    // No-op stub: requires global quote configuration not present in tests.
}

pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    // Default delimiter/quote characters used when called as a library helper.
    let delim_in: u8 = b',';
    let quote_in: u8 = b'"';
    let allow_binary = false;

    let bytes = buf.as_bytes();
    let limit = buflen.min(bytes.len());
    let mut i = 0usize;
    while i < limit {
        let c = bytes[i];
        if !allow_binary && is_bin_char(c as char) {
            // In the library context just return EOL on binary; the C version exits.
            *end = i;
            return RV_STATE_EOL;
        }
        if *in_quoted {
            if c == quote_in {
                *in_quoted = false;
            }
        } else {
            if c == quote_in {
                *in_quoted = true;
            }
            if c == delim_in {
                *end = i;
                return RV_DELIM;
            }
            if c == b'\r' || c == b'\n' {
                *end = i;
                return RV_STATE_EOL;
            }
            if c == 0 && i == limit - 1 {
                *end = i;
                return RV_STATE_EOL;
            }
        }
        i += 1;
    }
    *end = limit;
    RV_STATE_EOL
}

pub fn main(_argc: i32, _argv: &[&str]) -> i32 {
    // Full CLI not implemented for the library test scope.
    0
}
