use crate::{csvline::CsvLine};
use std::io::Write;
pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

pub fn output_line(cline: &CsvLine) {
    let mut out = std::io::stdout();
    let fieldcnt = cline.get_field_count();
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
    if level <= 0 {
        eprintln!("{}", fmt);
    }
}

pub fn is_bin_char(c: char) -> bool {
    let v = c as u32;
    matches!(v,
        1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127
    )
}

pub fn format_output(_str: &mut [&str]) {
    // Quote translation; in C this rewrites in place. Here we're a placeholder.
}

const RV_STATE_EOL: i32 = 0x02;
const RV_DELIM: i32 = 0x04;

pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let bytes = buf.as_bytes();
    let quote = b'"';
    let delim = b',';
    *end = 0;
    while *end < buflen {
        let c = bytes[*end];
        if *in_quoted {
            if c == quote {
                *in_quoted = false;
            }
        } else {
            if c == quote {
                *in_quoted = true;
            }
            if c == delim {
                return RV_DELIM;
            }
            if c == b'\r' {
                return RV_STATE_EOL;
            }
            if c == b'\n' {
                return RV_STATE_EOL;
            }
            if c == 0 && *end == buflen - 1 {
                return RV_STATE_EOL;
            }
        }
        *end += 1;
    }
    RV_STATE_EOL
}

pub fn main(_argc: i32, _argv: &[&str]) -> i32 {
    0
}
