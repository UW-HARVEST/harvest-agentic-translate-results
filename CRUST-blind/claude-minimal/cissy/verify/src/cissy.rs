use crate::csvline::CsvLine;
use std::io::Write;

pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

pub fn output_line(cline: &CsvLine) {
    // Print all fields with default ',' delimiter, since globals are not configurable here.
    let fieldcnt = cline.get_field_count();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for i in 0..fieldcnt {
        if let Some(s) = cline.get_field(i) {
            let _ = write!(out, "{}", s);
        }
        if i + 1 != fieldcnt {
            let _ = write!(out, ",");
        }
    }
    let _ = write!(out, "{}", cline.eol_str);
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
    let _ = write!(fp, "{}", help);
}

pub fn debug(level: i32, fmt: &str) {
    // Verbose level not tracked here since this is a function-level helper.
    // Mirror C's behavior of printing to stderr if level <= verbose.
    // We only print if level <= 0 to be conservative without globals.
    if level <= 0 {
        eprint!("{}", fmt);
    }
}

pub fn is_bin_char(c: char) -> bool {
    let v = c as u32;
    matches!(
        v,
        1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127
    )
}

pub fn format_output(_str: &mut [&str]) {
    // Without globals for input/output quote characters this function is a stub
    // that mirrors the C behavior when input quote == output quote (no change).
}

// rvStateNormal = 0, rvStateMultiline = 0x01, rvStateEOL = 0x02, rvDelim = 0x04
pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let rv_state_eol: i32 = 0x02;
    let rv_delim: i32 = 0x04;
    let bytes = buf.as_bytes();
    let delim_in: u8 = b',';
    let quote_in: u8 = b'"';

    *end = 0;
    while *end < buflen {
        let c = bytes[*end];
        if *in_quoted {
            if c == quote_in {
                *in_quoted = false;
            }
        } else {
            if c == quote_in {
                *in_quoted = true;
            }
            if c == delim_in {
                return rv_delim;
            }
            if c == b'\r' {
                return rv_state_eol;
            }
            if c == b'\n' {
                return rv_state_eol;
            }
            if c == 0 && *end == buflen - 1 {
                return rv_state_eol;
            }
        }
        *end += 1;
    }
    rv_state_eol
}

pub fn main(_argc: i32, _argv: &[&str]) -> i32 {
    // Stubbed entry-point matching the C signature. The full CLI would require
    // global configuration state which is intentionally not modeled here.
    0
}
