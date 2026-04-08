use crate::csvline::CsvLine;
use crate::range::range::{RangeElement, RangeType};
pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;
pub fn output_line(cline: &CsvLine) {
    // This is a simplified version - the full logic is in main()
    // Print all fields separated by commas with the line's eol_str
    let fieldcnt = cline.get_field_count();
    for i in 0..fieldcnt {
        if let Some(s) = cline.get_field(i) {
            print!("{}", s);
        }
        if i + 1 != fieldcnt {
            print!(",");
        }
    }
    print!("{}", cline.eol_str);
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
    let _ = write!(fp, "{}", help);
}
pub fn debug(level: i32, fmt: &str) {
    // In the C code, debug prints to stderr if level <= gpVerbose.
    // Since we don't have global state here, this is a no-op stub.
    // The main function handles debug output inline.
    let _ = (level, fmt);
}
pub fn is_bin_char(c: char) -> bool {
    let b = c as u32;
    matches!(b, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}
pub fn format_output(str: &mut [&str]) {
    // This modifies quote chars in-place in C. In Rust with &str slices,
    // we can't mutate. The actual formatting is handled in main's output logic.
    let _ = str;
}
pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let rv_state_eol: i32 = 0x02;
    let rv_delim: i32 = 0x04;

    // We need the global state for delim_in, quote_in, allow_binary, line_cnt
    // but the signature doesn't pass them. We'll use defaults: delim=',', quote='"'
    // The real processing happens in main() which has access to all state.
    let bytes = buf.as_bytes();
    *end = 0;
    while *end < buflen {
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
            if c == '\0' && *end == buflen - 1 {
                return rv_state_eol;
            }
        }
        *end += 1;
    }
    rv_state_eol
}
pub fn main(argc: i32, argv: &[&str]) -> i32 {
    use std::io::{self, BufRead, Write, BufWriter};
    use std::fs::File;

    let rv_state_eol: i32 = 0x02;
    let rv_delim: i32 = 0x04;

    // Global parameters
    let mut input: Option<File> = None;
    let mut output: Option<File> = None;
    let mut delim_in: char = ',';
    let mut delim_out: char = ',';
    let mut quote_in: char = '"';
    let mut quote_out: char = '"';
    let mut end_line: Option<String> = None;
    let mut verbose: i32 = 0;
    let mut allow_binary_flag = false;
    let mut out_columns: Option<Box<RangeElement>> = None;
    let mut line_cnt: i32 = 0;

    // Parse arguments
    let mut arginc: usize = 1;
    while arginc < argc as usize {
        let arg = argv[arginc];
        match arg {
            "-i" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                match File::open(argv[arginc]) {
                    Ok(f) => input = Some(f),
                    Err(_) => {
                        eprintln!("error: unable to open ({}) for gpInput", argv[arginc]);
                        return -1;
                    }
                }
                arginc += 1;
            }
            "-o" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                // Note: C code opens with "r" which is a bug, but we replicate behavior
                match File::open(argv[arginc]) {
                    Ok(f) => output = Some(f),
                    Err(_) => {
                        eprintln!("error: unable to open ({}) for gpInput", argv[arginc]);
                        return -1;
                    }
                }
                arginc += 1;
            }
            "-d" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                delim_in = argv[arginc].chars().next().unwrap();
                delim_out = delim_in;
                arginc += 1;
            }
            "-di" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                delim_in = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-do" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                delim_out = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-q" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                quote_in = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-qi" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                quote_in = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-qo" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                quote_out = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-eu" => { end_line = Some("\n".to_string()); arginc += 1; }
            "-ed" => { end_line = Some("\r\n".to_string()); arginc += 1; }
            "-em" => { end_line = Some("\r".to_string()); arginc += 1; }
            "-c" => {
                if argc as usize <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                out_columns = RangeElement::parse_int_ranges(argv[arginc]);
                arginc += 1;
            }
            "-h" => {
                // Print to stdout
                print!("cissy [options]\n\
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
                    \t-h \t\t\t help\n");
                std::process::exit(0);
            }
            "-v" => { verbose += 1; arginc += 1; }
            "-DDD" => { verbose += 100; arginc += 1; }
            "-b" => { allow_binary_flag = true; arginc += 1; }
            _ => {
                eprintln!("error: unknown switch ({})", arg);
                return -1;
            }
        }
    }

    // Helper closures / local functions
    let format_output_str = |s: &str, qi: char, qo: char| -> String {
        if qi == qo { return s.to_string(); }
        let bytes = s.as_bytes();
        if bytes.len() < 2 { return s.to_string(); }
        if bytes[0] == qi as u8 && bytes[bytes.len() - 1] == qi as u8 {
            let mut result = s.to_string();
            // SAFETY: we're replacing single-byte ASCII chars
            let b = result.as_bytes_mut_workaround();
            // Can't use as_bytes_mut without unsafe, so rebuild
            let mut chars: Vec<char> = s.chars().collect();
            chars[0] = qo;
            let last = chars.len() - 1;
            chars[last] = qo;
            return chars.into_iter().collect();
        }
        s.to_string()
    };

    // Local get_field that uses our parameters
    let local_get_field = |buf: &[u8], start: usize, buflen: usize, in_quoted: &mut bool, line_cnt: i32| -> (i32, usize) {
        let mut end: usize = 0;
        while end < buflen {
            let c = buf[start + end];
            if !allow_binary_flag && is_bin_char(c as char) {
                eprintln!("error: line({}): binary character ({}) found.  Use flag to ignore/pass", line_cnt, c);
                std::process::exit(-1);
            }
            if *in_quoted {
                if c == quote_in as u8 {
                    *in_quoted = false;
                }
            } else {
                if c == quote_in as u8 {
                    *in_quoted = true;
                }
                if c == delim_in as u8 {
                    return (rv_delim, end);
                }
                if c == b'\r' || c == b'\n' {
                    return (rv_state_eol, end);
                }
                if c == 0 && end == buflen - 1 {
                    return (rv_state_eol, end);
                }
            }
            end += 1;
        }
        (rv_state_eol, end)
    };

    // Output a line with all the context
    let output_line_ctx = |cline: &CsvLine, out_columns: &Option<Box<RangeElement>>, delim_out: char, quote_in: char, quote_out: char, end_line: &Option<String>, stdout: &mut dyn Write| {
        if out_columns.is_none() {
            // print all
            let fieldcnt = cline.get_field_count();
            for i in 0..fieldcnt {
                let s = cline.get_field(i).unwrap_or("");
                let s = format_output_str(s, quote_in, quote_out);
                let _ = write!(stdout, "{}", s);
                if i + 1 != fieldcnt {
                    let _ = write!(stdout, "{}", delim_out);
                }
            }
        } else {
            let mut list = out_columns.as_ref();
            let col_cnt = cline.get_field_count();
            while let Some(elem) = list {
                match elem.rangetype {
                    RangeType::Empty => {}
                    RangeType::Single => {
                        let s = cline.get_field(elem.start as usize - 1).unwrap_or("");
                        let s = format_output_str(s, quote_in, quote_out);
                        if !s.is_empty() {
                            let _ = write!(stdout, "{}", s);
                        }
                        if elem.next.is_some() {
                            let _ = write!(stdout, "{}", delim_out);
                        }
                    }
                    RangeType::StartEnd => {
                        let start_idx = elem.start as usize - 1;
                        let end_idx = elem.end as usize;
                        for i in start_idx..end_idx.saturating_sub(1) {
                            let s = cline.get_field(i).unwrap_or("");
                            let s = format_output_str(&s, quote_in, quote_out);
                            if s.is_empty() {
                                let _ = write!(stdout, "{}", delim_out);
                            } else {
                                let _ = write!(stdout, "{}{}", s, delim_out);
                            }
                        }
                        let last = elem.end as usize - 1;
                        if last < col_cnt {
                            let s = cline.get_field(last).unwrap_or("");
                            let s = format_output_str(&s, quote_in, quote_out);
                            if !s.is_empty() {
                                let _ = write!(stdout, "{}", s);
                            }
                        }
                        if elem.next.is_some() {
                            let _ = write!(stdout, "{}", delim_out);
                        }
                    }
                    RangeType::GreaterEqual => {
                        let start_idx = elem.start as usize - 1;
                        for i in start_idx..col_cnt.saturating_sub(1) {
                            let s = cline.get_field(i).unwrap_or("");
                            let s = format_output_str(&s, quote_in, quote_out);
                            if !s.is_empty() {
                                let _ = write!(stdout, "{}{}", s, delim_out);
                            }
                        }
                        let last = col_cnt.saturating_sub(1);
                        let s = cline.get_field(last).unwrap_or("");
                        let s = format_output_str(&s, quote_in, quote_out);
                        if !s.is_empty() {
                            let _ = write!(stdout, "{}", s);
                        }
                        if elem.next.is_some() {
                            let _ = write!(stdout, "{}", delim_out);
                        }
                    }
                }
                list = elem.next.as_ref();
            }
        }
        let eol = match end_line {
            Some(e) => e.as_str(),
            None => &cline.eol_str,
        };
        let _ = write!(stdout, "{}", eol);
    };

    // Main processing loop
    let stdin_handle = io::stdin();
    let reader: Box<dyn BufRead> = if let Some(f) = input {
        Box::new(io::BufReader::new(f))
    } else {
        Box::new(stdin_handle.lock())
    };

    let stdout_handle = io::stdout();
    let mut writer: Box<dyn Write> = Box::new(BufWriter::new(stdout_handle.lock()));

    let mut lines_iter = reader;
    let mut raw_input = String::new();

    loop {
        raw_input.clear();
        let bytes_read = match lines_iter.read_line(&mut raw_input) {
            Ok(n) => n,
            Err(_) => 0,
        };
        line_cnt += 1;

        if bytes_read == 0 {
            return 0;
        }

        let mut cline = CsvLine::new();
        let buf_bytes = raw_input.as_bytes();
        let bufsize = bytes_read;
        let mut start_idx: usize = 0;
        let mut b_inside_quote = false;
        let mut append_mode = false;

        loop {
            let remaining = bufsize - start_idx;
            let (parse_state, field_len) = local_get_field(buf_bytes, start_idx, remaining, &mut b_inside_quote, line_cnt);

            if append_mode {
                cline.append_field(&raw_input, start_idx, field_len);
            } else {
                cline.add_field(&raw_input, start_idx, field_len);
            }
            start_idx += field_len;
            if parse_state == rv_delim {
                start_idx += 1;
            }
            append_mode = b_inside_quote;

            if parse_state == rv_state_eol {
                if b_inside_quote {
                    // Need to read another line
                    raw_input.clear();
                    let c = match lines_iter.read_line(&mut raw_input) {
                        Ok(n) => n,
                        Err(_) => 0,
                    };
                    if c == 0 {
                        eprintln!("error: unterminated quote");
                        std::process::exit(-1);
                    }
                    start_idx = 0;
                    // bufsize needs to be updated but we re-read raw_input
                    // We need to continue the loop with the new buffer
                    // The C code resets and continues the while loop
                    let new_bufsize = c;
                    // We need to inline the continuation here
                    // Actually, let's restructure to handle multiline properly
                    loop {
                        let remaining = new_bufsize - start_idx;
                        let buf_bytes_inner = raw_input.as_bytes();
                        let (ps, fl) = local_get_field(buf_bytes_inner, start_idx, remaining, &mut b_inside_quote, line_cnt);
                        if append_mode {
                            cline.append_field(&raw_input, start_idx, fl);
                        } else {
                            cline.add_field(&raw_input, start_idx, fl);
                        }
                        start_idx += fl;
                        if ps == rv_delim {
                            start_idx += 1;
                        }
                        append_mode = b_inside_quote;
                        if ps == rv_state_eol {
                            if b_inside_quote {
                                raw_input.clear();
                                let c2 = match lines_iter.read_line(&mut raw_input) {
                                    Ok(n) => n,
                                    Err(_) => 0,
                                };
                                if c2 == 0 {
                                    eprintln!("error: unterminated quote");
                                    std::process::exit(-1);
                                }
                                start_idx = 0;
                                continue;
                            } else {
                                break;
                            }
                        }
                    }
                    break;
                } else {
                    break;
                }
            }
        }

        output_line_ctx(&cline, &out_columns, delim_out, quote_in, quote_out, &end_line, &mut *writer);
    }
}
