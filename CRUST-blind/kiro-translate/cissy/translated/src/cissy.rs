use crate::csvline::CsvLine;
use crate::range::range::{RangeElement, RangeType};
use std::io::{self, BufRead, Write, BufWriter};
use std::sync::Mutex;

pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

static GP_DELIM_IN: Mutex<char> = Mutex::new(',');
static GP_DELIM_OUT: Mutex<char> = Mutex::new(',');
static GP_QUOTE_IN: Mutex<char> = Mutex::new('"');
static GP_QUOTE_OUT: Mutex<char> = Mutex::new('"');
static GP_END_LINE: Mutex<Option<String>> = Mutex::new(None);
static GP_ALLOW_BINARY: Mutex<bool> = Mutex::new(false);
static GP_VERBOSE: Mutex<i32> = Mutex::new(0);
static GP_OUT_COLUMNS: Mutex<Option<Box<RangeElement>>> = Mutex::new(None);
static GP_LINE_CNT: Mutex<i32> = Mutex::new(0);
static GP_OUTPUT: Mutex<Option<BufWriter<Box<dyn Write + Send>>>> = Mutex::new(None);

pub fn output_line(cline: &CsvLine) {
    let delim_out = *GP_DELIM_OUT.lock().unwrap();
    let quote_in = *GP_QUOTE_IN.lock().unwrap();
    let quote_out = *GP_QUOTE_OUT.lock().unwrap();
    let end_line_opt = GP_END_LINE.lock().unwrap().clone();
    let out_columns = GP_OUT_COLUMNS.lock().unwrap();
    let mut output = GP_OUTPUT.lock().unwrap();
    let fp = output.as_mut().unwrap();

    let do_format = |s: &str, fp: &mut BufWriter<Box<dyn Write + Send>>| {
        let mut owned;
        let result = if quote_in != quote_out {
            let bytes = s.as_bytes();
            if bytes.len() >= 2 && bytes[0] == quote_in as u8 && bytes[bytes.len()-1] == quote_in as u8 {
                owned = String::with_capacity(s.len());
                owned.push(quote_out);
                owned.push_str(&s[1..s.len()-1]);
                owned.push(quote_out);
                owned.as_str()
            } else {
                s
            }
        } else {
            s
        };
        // result is what we print - but we need to handle lifetime
        // just write it directly
        let _ = write!(fp, "{}", result);
    };

    match &*out_columns {
        None => {
            let fieldcnt = cline.get_field_count();
            for i in 0..fieldcnt {
                let s = cline.get_field(i).unwrap_or("");
                do_format(s, fp);
                if i + 1 != fieldcnt {
                    let _ = write!(fp, "{}", delim_out);
                }
            }
        }
        Some(head) => {
            let mut list: &RangeElement = head;
            loop {
                match list.rangetype {
                    RangeType::Empty => {}
                    RangeType::Single => {
                        let s = cline.get_field((list.start - 1) as usize).unwrap_or("");
                        if !s.is_empty() {
                            do_format(s, fp);
                        }
                        if list.next.is_some() {
                            let _ = write!(fp, "{}", delim_out);
                        }
                    }
                    RangeType::StartEnd => {
                        let col_cnt = cline.get_field_count();
                        let start = (list.start - 1) as usize;
                        let end = list.end as usize;
                        for i in start..end.saturating_sub(1) {
                            let s = cline.get_field(i).unwrap_or("");
                            if s.is_empty() {
                                let _ = write!(fp, "{}", delim_out);
                            } else {
                                do_format(s, fp);
                                let _ = write!(fp, "{}", delim_out);
                            }
                        }
                        let last = (list.end - 1) as usize;
                        if last < col_cnt {
                            let s = cline.get_field(last).unwrap_or("");
                            if !s.is_empty() {
                                do_format(s, fp);
                            }
                        }
                        if list.next.is_some() {
                            let _ = write!(fp, "{}", delim_out);
                        }
                    }
                    RangeType::GreaterEqual => {
                        let col_cnt = cline.get_field_count();
                        let start = (list.start - 1) as usize;
                        if col_cnt > 0 {
                            for i in start..col_cnt - 1 {
                                let s = cline.get_field(i).unwrap_or("");
                                if !s.is_empty() {
                                    do_format(s, fp);
                                    let _ = write!(fp, "{}", delim_out);
                                }
                            }
                            let last = col_cnt - 1;
                            let s = cline.get_field(last).unwrap_or("");
                            if !s.is_empty() {
                                do_format(s, fp);
                            }
                        }
                        if list.next.is_some() {
                            let _ = write!(fp, "{}", delim_out);
                        }
                    }
                }
                match &list.next {
                    Some(next) => list = next,
                    None => break,
                }
            }
        }
    }

    let eol = match &end_line_opt {
        Some(e) => e.clone(),
        None => cline.eol_str.clone(),
    };
    let _ = write!(fp, "{}", eol);
}

pub fn usage(fp: &mut std::fs::File) {
    let help = "\
cissy [options]\n\
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
    let verbose = *GP_VERBOSE.lock().unwrap();
    if level <= verbose {
        eprint!("{}", fmt);
    }
}

pub fn is_bin_char(c: char) -> bool {
    let n = c as u32;
    matches!(n, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}

pub fn format_output(str: &mut [&str]) {
    let quote_in = *GP_QUOTE_IN.lock().unwrap();
    let quote_out = *GP_QUOTE_OUT.lock().unwrap();
    if quote_in == quote_out {
        return;
    }
    let _ = str;
}

pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
    let rv_state_eol: i32 = 0x02;
    let rv_delim: i32 = 0x04;
    let delim_in = *GP_DELIM_IN.lock().unwrap();
    let quote_in = *GP_QUOTE_IN.lock().unwrap();
    let allow_binary = *GP_ALLOW_BINARY.lock().unwrap();
    let line_cnt = *GP_LINE_CNT.lock().unwrap();

    let bytes = buf.as_bytes();
    *end = 0;
    while *end < buflen {
        let c = bytes[*end] as char;
        if !allow_binary && is_bin_char(c) {
            eprintln!("error: line({}): binary character ({}) found.  Use flag to ignore/pass", line_cnt, bytes[*end] as i32);
            std::process::exit(-1);
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
    // init globals
    *GP_DELIM_IN.lock().unwrap() = ',';
    *GP_DELIM_OUT.lock().unwrap() = ',';
    *GP_QUOTE_IN.lock().unwrap() = '"';
    *GP_QUOTE_OUT.lock().unwrap() = '"';
    *GP_END_LINE.lock().unwrap() = None;
    *GP_VERBOSE.lock().unwrap() = 0;
    *GP_ALLOW_BINARY.lock().unwrap() = false;
    *GP_OUT_COLUMNS.lock().unwrap() = None;

    let mut input_file: Option<String> = None;
    let mut output_file: Option<String> = None;

    let mut arginc: usize = 1;
    let argc = argc as usize;
    while arginc < argc {
        match argv[arginc] {
            "-i" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                input_file = Some(argv[arginc].to_string());
                arginc += 1;
            }
            "-o" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                output_file = Some(argv[arginc].to_string());
                arginc += 1;
            }
            "-d" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                let ch = argv[arginc].chars().next().unwrap();
                *GP_DELIM_IN.lock().unwrap() = ch;
                *GP_DELIM_OUT.lock().unwrap() = ch;
                arginc += 1;
            }
            "-di" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                *GP_DELIM_IN.lock().unwrap() = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-do" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                *GP_DELIM_OUT.lock().unwrap() = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-q" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                *GP_QUOTE_IN.lock().unwrap() = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-qi" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                *GP_QUOTE_IN.lock().unwrap() = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-qo" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                if argv[arginc].len() != 1 {
                    eprintln!("error: only single character delimiters allowed: '{}'", argv[arginc]);
                    return -1;
                }
                *GP_QUOTE_OUT.lock().unwrap() = argv[arginc].chars().next().unwrap();
                arginc += 1;
            }
            "-eu" => {
                *GP_END_LINE.lock().unwrap() = Some("\n".to_string());
                arginc += 1;
            }
            "-ed" => {
                *GP_END_LINE.lock().unwrap() = Some("\r\n".to_string());
                arginc += 1;
            }
            "-em" => {
                *GP_END_LINE.lock().unwrap() = Some("\r".to_string());
                arginc += 1;
            }
            "-c" => {
                if argc <= arginc + 1 {
                    eprintln!("error: missing argument for '{}'", argv[arginc]);
                    return -1;
                }
                arginc += 1;
                *GP_OUT_COLUMNS.lock().unwrap() = RangeElement::parse_int_ranges(argv[arginc]);
                arginc += 1;
            }
            "-h" => {
                // usage to stdout - but signature requires &mut File
                // In the C code it prints to stdout; we'll just print directly
                print!("\
cissy [options]\n\
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
            "-v" => {
                *GP_VERBOSE.lock().unwrap() += 1;
                arginc += 1;
            }
            "-DDD" => {
                *GP_VERBOSE.lock().unwrap() += 100;
                arginc += 1;
            }
            "-b" => {
                *GP_ALLOW_BINARY.lock().unwrap() = true;
                arginc += 1;
            }
            other => {
                eprintln!("error: unknown switch ({})", other);
                return -1;
            }
        }
    }

    // Set up output
    let output_writer: BufWriter<Box<dyn Write + Send>> = match &output_file {
        Some(path) => {
            match std::fs::File::open(path) {
                Ok(f) => BufWriter::new(Box::new(f)),
                Err(_) => {
                    eprintln!("error: unable to open ({}) for gpInput", path);
                    return -1;
                }
            }
        }
        None => BufWriter::new(Box::new(io::stdout())),
    };
    *GP_OUTPUT.lock().unwrap() = Some(output_writer);

    let rv_state_eol: i32 = 0x02;
    let rv_delim: i32 = 0x04;

    let mut line_cnt: i32 = 0;

    let mut input_reader: Box<dyn BufRead> = match &input_file {
        Some(path) => {
            match std::fs::File::open(path) {
                Ok(f) => Box::new(io::BufReader::new(f)),
                Err(_) => {
                    eprintln!("error: unable to open ({}) for gpInput", path);
                    return -1;
                }
            }
        }
        None => Box::new(io::BufReader::new(io::stdin())),
    };

    loop {
        let mut raw_input = String::new();
        let c = input_reader.read_line(&mut raw_input).unwrap_or(0);
        line_cnt += 1;
        *GP_LINE_CNT.lock().unwrap() = line_cnt;

        if c == 0 {
            // EOF
            // flush output
            if let Some(ref mut w) = *GP_OUTPUT.lock().unwrap() {
                let _ = w.flush();
            }
            return 0;
        }

        let mut cline = CsvLine::new();
        let mut start_idx: usize = 0;
        let mut bufsize = raw_input.len();
        let mut b_inside_quote = false;
        let mut append_mode = false;

        loop {
            let mut field_len: usize = 0;
            let parse_state = get_field(&raw_input[start_idx..], bufsize - start_idx, &mut field_len, &mut b_inside_quote);

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
                    raw_input.clear();
                    let c2 = input_reader.read_line(&mut raw_input).unwrap_or(0);
                    if c2 == 0 {
                        eprintln!("error: unterminated quote");
                        std::process::exit(-1);
                    }
                    start_idx = 0;
                    bufsize = raw_input.len();
                } else {
                    break;
                }
            }
        }

        output_line(&cline);
    }
}
