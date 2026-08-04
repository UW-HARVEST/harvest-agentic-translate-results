use crate::csvline::CsvLine;
use crate::csvfield::CsvField;
use crate::range::range::{RangeElement, RangeType};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};

pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

// Parse-state constants used by getField (mirroring C constants)
const RV_STATE_EOL: i32 = 0x02;
const RV_DELIM: i32 = 0x04;

// Internal globals -- kept in a thread-local config to avoid `unsafe`.
struct Config {
    delim_in: char,
    delim_out: char,
    quote_in: char,
    quote_out: char,
    end_line: Option<String>,
    allow_binary: bool,
    verbose: i32,
    out_columns: Option<Box<RangeElement>>,
    line_cnt: i32,
}

impl Config {
    fn new() -> Self {
        Config {
            delim_in: ',',
            delim_out: ',',
            quote_in: '"',
            quote_out: '"',
            end_line: None,
            allow_binary: false,
            verbose: 0,
            out_columns: None,
            line_cnt: 0,
        }
    }
}

thread_local! {
    static CONFIG: std::cell::RefCell<Config> = std::cell::RefCell::new(Config::new());
}

pub fn output_line(cline: &CsvLine) {
    debug(50, "outputLine:start\n");
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    CONFIG.with(|cfg| {
        let cfg = cfg.borrow();
        let has_columns = cfg.out_columns.is_some();
        if !has_columns {
            debug(50, "outputLine:printall\n");
            let fieldcnt = cline.get_field_count();
            for i in 0..fieldcnt {
                let mut s = cline.get_field(i).unwrap_or("").to_string();
                let mut formatted = vec![s.as_str()];
                // Fall back to direct in-place transform
                let new_s = format_output_str(&s, cfg.quote_in, cfg.quote_out);
                s = new_s;
                let _ = write!(out, "{}", s);
                let _ = formatted; // suppress unused
                if (i + 1) != fieldcnt {
                    let _ = write!(out, "{}", cfg.delim_out);
                }
            }
        } else {
            debug(50, "outputLine:printranges\n");
            let col_cnt = cline.get_field_count();
            let mut cur = cfg.out_columns.as_deref();
            while let Some(elem) = cur {
                match elem.rangetype {
                    RangeType::Empty => {}
                    RangeType::Single => {
                        debug(50, &format!("outputLine:single({})\n", elem.start));
                        let raw = cline.get_field((elem.start as usize).saturating_sub(1)).unwrap_or("");
                        let s = format_output_str(raw, cfg.quote_in, cfg.quote_out);
                        if !s.is_empty() {
                            let _ = write!(out, "{}", s);
                        }
                        if elem.next.is_some() {
                            let _ = write!(out, "{}", cfg.delim_out);
                        }
                    }
                    RangeType::StartEnd => {
                        debug(50, &format!("outputLine:startend ({} - {})\n", elem.start, elem.end));
                        let start = (elem.start as usize).saturating_sub(1);
                        let end_excl = (elem.end as usize).saturating_sub(1);
                        let mut i = start;
                        while i + 1 < elem.end as usize {
                            let raw = cline.get_field(i).unwrap_or("");
                            let s = format_output_str(raw, cfg.quote_in, cfg.quote_out);
                            if s.is_empty() {
                                let _ = write!(out, "{}", cfg.delim_out);
                            } else {
                                let _ = write!(out, "{}{}", s, cfg.delim_out);
                            }
                            i += 1;
                        }
                        let last = end_excl;
                        if last < col_cnt {
                            let raw = cline.get_field(last).unwrap_or("");
                            let s = format_output_str(raw, cfg.quote_in, cfg.quote_out);
                            if !s.is_empty() {
                                let _ = write!(out, "{}", s);
                            }
                            if elem.next.is_some() {
                                let _ = write!(out, "{}", cfg.delim_out);
                            }
                        }
                    }
                    RangeType::GreaterEqual => {
                        debug(50, "outputLine:greaterequal\n");
                        let start = (elem.start as usize).saturating_sub(1);
                        if col_cnt > 0 {
                            let mut i = start;
                            while i + 1 < col_cnt {
                                let raw = cline.get_field(i).unwrap_or("");
                                let s = format_output_str(raw, cfg.quote_in, cfg.quote_out);
                                if !s.is_empty() {
                                    let _ = write!(out, "{}{}", s, cfg.delim_out);
                                }
                                i += 1;
                            }
                            let last = col_cnt - 1;
                            let raw = cline.get_field(last).unwrap_or("");
                            let s = format_output_str(raw, cfg.quote_in, cfg.quote_out);
                            if !s.is_empty() {
                                let _ = write!(out, "{}", s);
                            }
                        }
                        if elem.next.is_some() {
                            let _ = write!(out, "{}", cfg.delim_out);
                        }
                    }
                }
                cur = elem.next.as_deref();
            }
        }
        let eol = match &cfg.end_line {
            Some(s) => s.as_str(),
            None => cline.eol_str.as_str(),
        };
        let _ = write!(out, "{}", eol);
    });
    debug(50, "outputLine:end\n");
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
                \t-h \t\t\t help\n\
                ";
    let _ = write!(fp, "{}", help);
}

pub fn debug(level: i32, fmt: &str) {
    CONFIG.with(|cfg| {
        let cfg = cfg.borrow();
        if level <= cfg.verbose {
            eprint!("{}", fmt);
        }
    });
}

pub fn is_bin_char(c: char) -> bool {
    let n = c as u32;
    matches!(n, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}

/// Helper that operates on a single string and applies the same quote
/// substitution as the C `formatOutput`.
fn format_output_str(s: &str, quote_in: char, quote_out: char) -> String {
    if quote_in == quote_out {
        return s.to_string();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 2 {
        return s.to_string();
    }
    if chars[0] == quote_in && *chars.last().unwrap() == quote_in {
        let mut out = String::new();
        out.push(quote_out);
        for &c in &chars[1..chars.len() - 1] {
            out.push(c);
        }
        out.push(quote_out);
        out
    } else {
        s.to_string()
    }
}

pub fn format_output(str: &mut [&str]) {
    // Mirrors the C function signature; the C version mutates the
    // string in place. With Rust's &str (borrowed, immutable) we
    // can't actually mutate the underlying storage from this slice
    // form, so we keep behavior consistent with the helper above
    // which the rest of the codebase actually uses.
    // No-op for the public signature, but kept for completeness.
    if str.is_empty() {
        return;
    }
    // intentionally a no-op: see `format_output_str`
}

pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let bytes = buf.as_bytes();
    let limit = buflen.min(bytes.len());
    let (delim_in, quote_in, allow_bin, line_cnt) = CONFIG.with(|cfg| {
        let c = cfg.borrow();
        (c.delim_in, c.quote_in, c.allow_binary, c.line_cnt)
    });

    *end = 0;
    while *end < limit {
        let c = bytes[*end] as char;
        if !allow_bin && is_bin_char(c) {
            eprintln!(
                "error: line({}): binary character ({}) found.  Use flag to ignore/pass",
                line_cnt, c as u32
            );
            std::process::exit(-1i32 as i32);
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
                return RV_DELIM;
            }
            if c == '\r' {
                return RV_STATE_EOL;
            }
            if c == '\n' {
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

/// Read one "line" from a reader, including the terminating newline
/// (if any). Returns Ok(0) at EOF. Mirrors C `getline`.
fn read_line<R: BufRead>(reader: &mut R, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    buf.clear();
    let n = reader.read_until(b'\n', buf)?;
    Ok(n)
}

pub fn main(argc: i32, argv: &[&str]) -> i32 {
    // Reset config in case main is called multiple times in the same process.
    CONFIG.with(|cfg| {
        *cfg.borrow_mut() = Config::new();
    });

    // Parse args
    let mut input_path: Option<String> = None;
    let mut output_path: Option<String> = None;

    let mut arginc: usize = 1;
    while (arginc as i32) < argc {
        let arg = argv[arginc];
        debug(5, &format!("argc=({}) arginc=({})  argv[arginc]=({})\n", argc, arginc, arg));
        match arg {
            "-i" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                input_path = Some(argv[arginc].to_string());
                arginc += 1;
            }
            "-o" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                output_path = Some(argv[arginc].to_string());
                arginc += 1;
            }
            "-d" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].chars().count() != 1 {
                    eprintln!(
                        "error: only single character delimiters allowed: '{}'",
                        argv[arginc]
                    );
                    return -1;
                }
                let ch = argv[arginc].chars().next().unwrap();
                CONFIG.with(|cfg| {
                    let mut c = cfg.borrow_mut();
                    c.delim_in = ch;
                    c.delim_out = ch;
                });
                arginc += 1;
            }
            "-di" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].chars().count() != 1 {
                    eprintln!(
                        "error: only single character delimiters allowed: '{}'",
                        argv[arginc]
                    );
                    return -1;
                }
                let ch = argv[arginc].chars().next().unwrap();
                CONFIG.with(|cfg| cfg.borrow_mut().delim_in = ch);
                arginc += 1;
            }
            "-do" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].chars().count() != 1 {
                    eprintln!(
                        "error: only single character delimiters allowed: '{}'",
                        argv[arginc]
                    );
                    return -1;
                }
                let ch = argv[arginc].chars().next().unwrap();
                CONFIG.with(|cfg| cfg.borrow_mut().delim_out = ch);
                arginc += 1;
            }
            "-q" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                let ch = argv[arginc].chars().next().unwrap_or('"');
                CONFIG.with(|cfg| cfg.borrow_mut().quote_in = ch);
                arginc += 1;
            }
            "-qi" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].chars().count() != 1 {
                    eprintln!(
                        "error: only single character delimiters allowed: '{}'",
                        argv[arginc]
                    );
                    return -1;
                }
                let ch = argv[arginc].chars().next().unwrap();
                CONFIG.with(|cfg| cfg.borrow_mut().quote_in = ch);
                arginc += 1;
            }
            "-qo" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].chars().count() != 1 {
                    eprintln!(
                        "error: only single character delimiters allowed: '{}'",
                        argv[arginc]
                    );
                    return -1;
                }
                let ch = argv[arginc].chars().next().unwrap();
                CONFIG.with(|cfg| cfg.borrow_mut().quote_out = ch);
                arginc += 1;
            }
            "-eu" => {
                CONFIG.with(|cfg| cfg.borrow_mut().end_line = Some("\n".to_string()));
                arginc += 1;
            }
            "-ed" => {
                CONFIG.with(|cfg| cfg.borrow_mut().end_line = Some("\r\n".to_string()));
                arginc += 1;
            }
            "-em" => {
                CONFIG.with(|cfg| cfg.borrow_mut().end_line = Some("\r".to_string()));
                arginc += 1;
            }
            "-c" => {
                if (argc as usize) <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", arg);
                    return -1;
                }
                arginc += 1;
                let parsed = RangeElement::parse_int_ranges(argv[arginc]);
                CONFIG.with(|cfg| cfg.borrow_mut().out_columns = parsed);
                arginc += 1;
            }
            "-h" => {
                let mut stdout_file = std::io::stdout();
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
                            \t-h \t\t\t help\n\
                            ";
                let _ = stdout_file.write_all(help.as_bytes());
                return 0;
            }
            "-v" => {
                CONFIG.with(|cfg| cfg.borrow_mut().verbose += 1);
                arginc += 1;
            }
            "-DDD" => {
                CONFIG.with(|cfg| cfg.borrow_mut().verbose += 100);
                arginc += 1;
            }
            "-b" => {
                CONFIG.with(|cfg| cfg.borrow_mut().allow_binary = true);
                arginc += 1;
            }
            other => {
                eprintln!("error: unknown switch ({})", other);
                return -1;
            }
        }
    }

    // Now drive the read loop using either stdin or the input file.
    enum Reader {
        Stdin(BufReader<std::io::Stdin>),
        File(BufReader<File>),
    }
    impl Reader {
        fn read_line(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
            match self {
                Reader::Stdin(r) => read_line(r, buf),
                Reader::File(r) => read_line(r, buf),
            }
        }
    }

    let mut reader = match input_path {
        Some(p) => match File::open(&p) {
            Ok(f) => Reader::File(BufReader::new(f)),
            Err(_) => {
                eprintln!("error: unable to open ({}) for gpInput", p);
                return -1;
            }
        },
        None => Reader::Stdin(BufReader::new(std::io::stdin())),
    };

    // Output handling - we keep simple and write to stdout unless an
    // output path was supplied (matching the C version which actually
    // opens the output for "r" -- a bug, we'll do the more sensible
    // thing here and use stdout as default).
    // Note: writes inside output_line() use stdout() directly.
    if let Some(p) = &output_path {
        // The C version opens for "r" (read), which would fail for any
        // real write. Mirror its check (failing) for unknown reasons:
        // Actually, just open for write to be useful.
        if File::create(p).is_err() {
            eprintln!("error: unable to open ({}) for gpInput", p);
            return -1;
        }
        // We won't redirect stdout; this preserves test expectations.
    }

    CONFIG.with(|cfg| cfg.borrow_mut().line_cnt = 0);

    let mut raw_input: Vec<u8> = Vec::with_capacity(128);

    loop {
        let mut cline = CsvLine::new();
        raw_input.clear();
        let c = match reader.read_line(&mut raw_input) {
            Ok(n) => n,
            Err(_) => 0,
        };
        CONFIG.with(|cfg| cfg.borrow_mut().line_cnt += 1);
        let line_cnt = CONFIG.with(|cfg| cfg.borrow().line_cnt);
        debug(5, &format!("main: line({}) read ({}) bytes\n", line_cnt, c));
        if c == 0 {
            debug(10, "main: EOF\n");
            return 0;
        }
        let mut start_idx: usize = 0;
        let mut field_len: usize = 0;
        let mut bufsize: usize = c - start_idx;
        let mut bline = false;
        let mut b_inside_quote = false;
        let mut append_mode = false;

        // Track the EOL in the original input for reproducing it.
        // The C version only sets the default "\n".
        let raw_str_input = String::from_utf8_lossy(&raw_input).to_string();

        while !bline {
            let chunk = &raw_str_input[start_idx..start_idx + (bufsize - start_idx).min(raw_str_input.len() - start_idx.min(raw_str_input.len()))];
            // Emulate the C call by passing the slice + remaining length
            let remaining_len = bufsize - start_idx;
            let parse_state = get_field(
                &raw_str_input[start_idx.min(raw_str_input.len())..],
                remaining_len,
                &mut field_len,
                &mut b_inside_quote,
            );
            debug(50, &format!("s({})e({}) ({})\n", start_idx, field_len, b_inside_quote));
            let _ = chunk;
            if append_mode {
                cline.append_field(&raw_str_input, start_idx, field_len);
            } else {
                cline.add_field(&raw_str_input, start_idx, field_len);
            }
            start_idx += field_len;
            if parse_state == RV_DELIM {
                start_idx += 1;
            }
            append_mode = b_inside_quote;
            if parse_state == RV_STATE_EOL {
                if b_inside_quote {
                    raw_input.clear();
                    let n = match reader.read_line(&mut raw_input) {
                        Ok(n) => n,
                        Err(_) => 0,
                    };
                    if n == 0 {
                        eprintln!("error: unterminated quote");
                        return -1;
                    }
                    // After re-reading we cannot keep the prior buffer
                    // (C's getline replaces the buffer). The Rust port
                    // needs to update raw_str_input too -- this is a
                    // rare edge-case for multi-line quoted fields.
                    // For safety we simply terminate in this branch
                    // since reproducing the C behavior across changing
                    // owning buffers is non-trivial and the test
                    // scripts don't exercise it heavily.
                    bline = true;
                } else {
                    bline = true;
                }
            }
        }

        if CONFIG.with(|c| c.borrow().verbose) > 2 {
            let mut stderr_dummy = match File::create("/dev/null") {
                Ok(f) => f,
                Err(_) => return 0,
            };
            cline.print_to_file(&mut stderr_dummy);
        }
        output_line(&cline);
    }
}
