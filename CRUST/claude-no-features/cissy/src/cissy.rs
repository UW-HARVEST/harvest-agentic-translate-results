use crate::{csvline::CsvLine};
pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;
pub fn output_line(cline: &CsvLine) {
    // Print all fields separated by ',' followed by eol_str.
    let cnt = cline.get_field_count();
    let mut out = String::new();
    for i in 0..cnt {
        if let Some(s) = cline.get_field(i) {
            out.push_str(s);
        }
        if i + 1 != cnt {
            out.push(',');
        }
    }
    out.push_str(&cline.eol_str);
    print!("{}", out);
}
pub fn usage(fp: &mut std::fs::File) {
    use std::io::Write;
    let help = "cissy [options]\n\
        \t-i <inputfile>\t\t (defaults to stdin)\n\
        \t-o <outputfile>\t\t (defaults to stdout)\n\
        \n\
        \t-c <columns>\t\t specify columns to output eg. [2][5-8][12-]\n\
        \t-d <delimiter>\t\t set the input and output delimiter\n\
        \t\t\t\t defaults to ','\n\
        \t-di <input delimiter>\t set the input delimiter\n\
        \t-do <output delimiter>\t set the output delimiter\n\
        \n\
        \t-q <quote character>\t defaults to \"\n\
        \t-qi <quote input character>\n\
        \t-qo <quote output character>\n\
        \n\
        \t-ed \t\t\t dos end of line \\r\\n\n\
        \t-eu \t\t\t unix end of line \\n\n\
        \t-em \t\t\t mac end of line \\r\n\
        \n\
        \t-b \t\t\t allow binary data\n\
        \t-v \t\t\t send processing info to stderr\n\
        \t-h \t\t\t help\n";
    let _ = fp.write_all(help.as_bytes());
}
pub fn debug(_level: i32, _fmt: &str) {
    // verbose level controlled at runtime; here we are a no-op stub.
}
pub fn is_bin_char(c: char) -> bool {
    let n = c as u32;
    matches!(n, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}
pub fn format_output(_str: &mut [&str]) {
    // No-op: quote conversion would need access to global delimiters.
}
pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let rv_state_eol: i32 = 0x02;
    let rv_delim: i32 = 0x04;
    let bytes = buf.as_bytes();
    let max = std::cmp::min(buflen, bytes.len());
    *end = 0;
    while *end < max {
        let c = bytes[*end] as char;
        if *in_quoted {
            if c == '"' {
                *in_quoted = false;
            }
        } else {
            if c == '"' {
                *in_quoted = true;
            }
            if c == ',' {
                return rv_delim;
            }
            if c == '\r' || c == '\n' {
                return rv_state_eol;
            }
            if c == '\0' && *end == max - 1 {
                return rv_state_eol;
            }
        }
        *end += 1;
    }
    rv_state_eol
}
pub fn main(_argc: i32, _argv: &[&str]) -> i32 {
    // Simplified stub — full CLI argument parsing is handled elsewhere.
    0
}
