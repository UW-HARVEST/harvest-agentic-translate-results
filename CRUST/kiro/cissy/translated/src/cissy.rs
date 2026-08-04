use crate::{csvline::CsvLine};
pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;
pub fn output_line(cline: &CsvLine) {
    // stub - not tested directly
}
pub fn usage(fp: &mut std::fs::File) {
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
    use std::io::Write;
    write!(fp, "{}", help).unwrap();
}
pub fn debug(level: i32, fmt: &str) {
    // stub
}
pub fn is_bin_char(c: char) -> bool {
    match c as u32 {
        1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127 => true,
        _ => false,
    }
}
pub fn format_output(str: &mut [&str]) {
    // stub - not tested directly
}
pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let rv_state_eol: i32 = 0x02;
    let rv_delim: i32 = 0x04;
    let bytes = buf.as_bytes();
    *end = 0;
    while *end < buflen {
        let c = bytes[*end] as char;
        if *in_quoted {
            if c == '"' { *in_quoted = false; }
        } else {
            if c == '"' { *in_quoted = true; }
            if c == ',' { return rv_delim; }
            if c == '\r' || c == '\n' { return rv_state_eol; }
            if c == '\0' && *end == buflen - 1 { return rv_state_eol; }
        }
        *end += 1;
    }
    rv_state_eol
}
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    0
}
