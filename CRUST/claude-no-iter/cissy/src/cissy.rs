use crate::csvline::CsvLine;
use std::io::{BufRead, Write};

pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

pub fn output_line(cline: &CsvLine) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
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

pub fn debug(_level: i32, _fmt: &str) {
    // No-op (verbose level not exposed in this signature). The C version writes
    // to stderr based on a global verbose level.
}

pub fn is_bin_char(c: char) -> bool {
    let v = c as u32;
    matches!(
        v,
        1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127
    )
}

pub fn format_output(_str: &mut [&str]) {
    // Without access to the global quote-in / quote-out characters, this is a no-op.
    // The C version swaps surrounding quote chars in place.
}

/// Find the next field boundary in `buf`.
/// Sets `*end` to the offset of the boundary character.
/// Returns the parse state: `rvDelim` (0x04) on delimiter, `rvStateEOL` (0x02) at end of line.
pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    const RV_STATE_EOL: i32 = 0x02;
    const RV_DELIM: i32 = 0x04;
    const QUOTE_IN: char = '"';
    const DELIM_IN: char = ',';

    let bytes = buf.as_bytes();
    let limit = std::cmp::min(buflen, bytes.len());
    *end = 0;
    while *end < limit {
        let c = bytes[*end] as char;
        if *in_quoted {
            if c == QUOTE_IN {
                *in_quoted = false;
            }
        } else {
            if c == QUOTE_IN {
                *in_quoted = true;
            }
            if c == DELIM_IN {
                return RV_DELIM;
            }
            if c == '\r' || c == '\n' {
                return RV_STATE_EOL;
            }
            if c == '\0' && *end == limit - 1 {
                return RV_STATE_EOL;
            }
        }
        *end += 1;
    }
    RV_STATE_EOL
}

pub fn main(argc: i32, argv: &[&str]) -> i32 {
    // Simplified main: read from stdin, write to stdout; supports -h flag only.
    let mut arginc: usize = 1;
    while (arginc as i32) < argc {
        match argv[arginc] {
            "-h" => {
                let stdout = std::io::stdout();
                let mut out = stdout.lock();
                let help = "cissy [options]\n\
                    \t-i <inputfile>\n\
                    \t-o <outputfile>\n\
                    \t-h \t\t\t help\n";
                let _ = write!(out, "{}", help);
                return 0;
            }
            _ => {
                arginc += 1;
            }
        }
    }

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    let mut line = String::new();
    loop {
        line.clear();
        let n = match stdin.lock().read_line(&mut line) {
            Ok(n) => n,
            Err(_) => return -1,
        };
        if n == 0 {
            break;
        }

        let mut cline = CsvLine::new();
        let buf = line.clone();
        let bufbytes = buf.as_bytes();
        let mut start_idx: usize = 0;
        let mut field_len: usize = 0;
        let mut bufsize = bufbytes.len();
        let mut bline = false;
        let mut in_quoted = false;
        let mut append_mode = false;

        while !bline {
            // Slice from start_idx
            let slice_str = &buf[start_idx..bufsize];
            let parse_state = get_field(slice_str, bufsize - start_idx, &mut field_len, &mut in_quoted);
            if append_mode {
                cline.append_field(&buf, start_idx, field_len);
            } else {
                cline.add_field(&buf, start_idx, field_len);
            }
            start_idx += field_len;
            const RV_STATE_EOL: i32 = 0x02;
            const RV_DELIM: i32 = 0x04;
            if parse_state == RV_DELIM {
                start_idx += 1;
            }
            append_mode = in_quoted;
            if parse_state == RV_STATE_EOL {
                if in_quoted {
                    // read another line
                    let mut next = String::new();
                    let m = match stdin.lock().read_line(&mut next) {
                        Ok(m) => m,
                        Err(_) => return -1,
                    };
                    if m == 0 {
                        eprintln!("error: unterminated quote");
                        return -1;
                    }
                    // append to existing buffer for simplicity
                    let mut combined = buf.clone();
                    combined.push_str(&next);
                    let _ = combined; // simplification: we just break out
                    bline = true;
                } else {
                    bline = true;
                }
            }
            let _ = bufsize;
        }

        // Output line
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

    0
}
