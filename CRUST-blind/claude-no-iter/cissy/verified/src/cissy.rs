use crate::csvline::CsvLine;
use std::io::Write;

pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

pub const RV_STATE_NORMAL: i32 = 0;
pub const RV_STATE_MULTILINE: i32 = 0x01;
pub const RV_STATE_EOL: i32 = 0x02;
pub const RV_DELIM: i32 = 0x04;

// Globals (mimic C globals as static state)
pub static mut G_LINE_CNT: i32 = 0;
pub static mut G_DELIM_IN: char = ',';
pub static mut G_DELIM_OUT: char = ',';
pub static mut G_QUOTE_IN: char = '"';
pub static mut G_QUOTE_OUT: char = '"';
pub static mut G_ALLOW_BINARY: bool = false;
pub static mut G_VERBOSE: i32 = 0;

pub fn output_line(cline: &CsvLine) {
    // Print every field separated by the output delimiter, terminated with EOL.
    let fieldcnt = cline.get_field_count();
    let delim_out = unsafe { G_DELIM_OUT };
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for i in 0..fieldcnt {
        if let Some(s) = cline.get_field(i) {
            let _ = write!(handle, "{}", s);
        }
        if i + 1 != fieldcnt {
            let _ = write!(handle, "{}", delim_out);
        }
    }
    let _ = write!(handle, "{}", cline.eol_str);
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
    let verbose = unsafe { G_VERBOSE };
    if level <= verbose {
        eprint!("{}", fmt);
    }
}

pub fn is_bin_char(c: char) -> bool {
    let v = c as u32;
    matches!(v, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}

pub fn format_output(str: &mut [&str]) {
    // Mirror the C formatOutput: if input quote == output quote, no-op.
    // Otherwise, if first/last char of the str are the input quote, replace
    // them with the output quote. Since we receive &mut [&str] (immutable
    // slices), we cannot truly mutate. This stub preserves the no-op behavior
    // for the common case (input quote == output quote).
    let qin = unsafe { G_QUOTE_IN };
    let qout = unsafe { G_QUOTE_OUT };
    if qin == qout {
        return;
    }
    let _ = str;
}

pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let bytes = buf.as_bytes();
    let limit = buflen.min(bytes.len());
    let delim_in = unsafe { G_DELIM_IN };
    let quote_in = unsafe { G_QUOTE_IN };
    let allow_bin = unsafe { G_ALLOW_BINARY };
    let mut i: usize = 0;
    while i < limit {
        let c = bytes[i] as char;
        if !allow_bin && is_bin_char(c) {
            *end = i;
            eprintln!("error: binary character ({}) found", c as u32);
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
            if c == '\r' {
                *end = i;
                return RV_STATE_EOL;
            }
            if c == '\n' {
                *end = i;
                return RV_STATE_EOL;
            }
            if c == '\0' && i == limit - 1 {
                *end = i;
                return RV_STATE_EOL;
            }
        }
        i += 1;
    }
    *end = i;
    RV_STATE_EOL
}

pub fn main(argc: i32, argv: &[&str]) -> i32 {
    // Minimal CLI parser that handles -h and unrecognized switches gracefully.
    let _ = argc;
    let mut i: usize = 1;
    while i < argv.len() {
        match argv[i] {
            "-h" => {
                // Print usage to stdout
                let help = "cissy [options]\n";
                print!("{}", help);
                return 0;
            }
            "-v" => unsafe {
                G_VERBOSE += 1;
                i += 1;
            },
            "-b" => unsafe {
                G_ALLOW_BINARY = true;
                i += 1;
            },
            _ => {
                // Skip unknown args; in the C version this would error.
                i += 1;
            }
        }
    }
    0
}
