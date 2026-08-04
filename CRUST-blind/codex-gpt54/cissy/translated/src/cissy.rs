use crate::{csvline::CsvLine};
use crate::range::{RangeElement, RangeType};
use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};

pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;

const RV_STATE_NORMAL: i32 = 0;
const RV_STATE_MULTILINE: i32 = 0x01;
const RV_STATE_EOL: i32 = 0x02;
const RV_DELIM: i32 = 0x04;

#[derive(Clone)]
struct Config {
    delim_in: char,
    delim_out: char,
    quote_in: char,
    quote_out: char,
    end_line: Option<String>,
    allow_binary: bool,
    verbose: i32,
    out_columns: Option<Box<RangeElement>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delim_in: ',',
            delim_out: ',',
            quote_in: '"',
            quote_out: '"',
            end_line: None,
            allow_binary: false,
            verbose: 0,
            out_columns: None,
        }
    }
}

thread_local! {
    static CONFIG: RefCell<Config> = RefCell::new(Config::default());
}

pub fn output_line(cline: &CsvLine) {
let rendered = CONFIG.with(|cfg| render_output_line(cline, &cfg.borrow()));
let _ = io::stdout().write_all(rendered.as_bytes());
}
pub fn usage(fp: &mut std::fs::File) {
let help =
    "cissy [options]\n\
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
pub fn debug(level: i32, fmt: &str) {
CONFIG.with(|cfg| {
    if level <= cfg.borrow().verbose {
        let _ = io::stderr().write_all(fmt.as_bytes());
    }
});
}
pub fn is_bin_char(c: char) -> bool {
matches!(c as u32, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}
pub fn format_output(str: &mut [&str]) {
if let Some(first) = str.first_mut() {
    let updated = CONFIG.with(|cfg| format_output_value(first, &cfg.borrow()));
    *first = Box::leak(updated.into_boxed_str());
}
}
pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
CONFIG.with(|cfg| {
    let cfg = cfg.borrow();
    let bytes = buf.as_bytes();
    let limit = buflen.min(bytes.len());
    for idx in 0..limit {
        *end = idx;
        let c = bytes[idx] as char;
        if !cfg.allow_binary && is_bin_char(c) {
            panic!("error: binary character ({}) found", c as u32);
        }
        if *in_quoted {
            if c == cfg.quote_in {
                *in_quoted = false;
            }
        } else {
            if c == cfg.quote_in {
                *in_quoted = true;
            }
            if c == cfg.delim_in {
                return RV_DELIM;
            }
            if c == '\r' || c == '\n' {
                return RV_STATE_EOL;
            }
            if c == '\0' && idx == limit.saturating_sub(1) {
                return RV_STATE_EOL;
            }
        }
    }
    *end = limit;
    RV_STATE_EOL
})
}
pub fn main(argc: i32, argv: &[&str]) -> i32 {
let arg_limit = argc.max(0) as usize;
let args = if arg_limit <= argv.len() { &argv[..arg_limit] } else { argv };

let mut config = Config::default();
let mut input_path: Option<String> = None;
let mut output_path: Option<String> = None;

let mut arginc = 1_usize;
while arginc < args.len() {
    match args[arginc] {
        "-i" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            input_path = Some(args[arginc].to_string());
        }
        "-o" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            output_path = Some(args[arginc].to_string());
        }
        "-d" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            let mut chars = args[arginc].chars();
            let delim = match (chars.next(), chars.next()) {
                (Some(ch), None) => ch,
                _ => {
                    eprintln!("error: only single character delimiters allowed: '{}'", args[arginc]);
                    return -1;
                }
            };
            config.delim_in = delim;
            config.delim_out = delim;
        }
        "-di" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            let mut chars = args[arginc].chars();
            config.delim_in = match (chars.next(), chars.next()) {
                (Some(ch), None) => ch,
                _ => {
                    eprintln!("error: only single character delimiters allowed: '{}'", args[arginc]);
                    return -1;
                }
            };
        }
        "-do" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            let mut chars = args[arginc].chars();
            config.delim_out = match (chars.next(), chars.next()) {
                (Some(ch), None) => ch,
                _ => {
                    eprintln!("error: only single character delimiters allowed: '{}'", args[arginc]);
                    return -1;
                }
            };
        }
        "-q" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            let mut chars = args[arginc].chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => {
                    config.quote_in = ch;
                    config.quote_out = ch;
                }
                _ => {
                    eprintln!("error: only single character delimiters allowed: '{}'", args[arginc]);
                    return -1;
                }
            }
        }
        "-qi" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            let mut chars = args[arginc].chars();
            config.quote_in = match (chars.next(), chars.next()) {
                (Some(ch), None) => ch,
                _ => {
                    eprintln!("error: only single character delimiters allowed: '{}'", args[arginc]);
                    return -1;
                }
            };
        }
        "-qo" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            let mut chars = args[arginc].chars();
            config.quote_out = match (chars.next(), chars.next()) {
                (Some(ch), None) => ch,
                _ => {
                    eprintln!("error: only single character delimiters allowed: '{}'", args[arginc]);
                    return -1;
                }
            };
        }
        "-eu" => config.end_line = Some("\n".to_string()),
        "-ed" => config.end_line = Some("\r\n".to_string()),
        "-em" => config.end_line = Some("\r".to_string()),
        "-c" => {
            if args.len() <= arginc + 1 {
                eprintln!("error: missing argument for '{}'", args[arginc]);
                return -1;
            }
            arginc += 1;
            config.out_columns = RangeElement::parse_int_ranges(args[arginc]);
        }
        "-h" => {
            let _ = io::stdout().write_all(
                b"cissy [options]\n\t-i <inputfile>\t\t (defaults to stdin)\n\t-o <outputfile>\t\t (defaults to stdout)\n\n\t-c <columns>\t\t specify columns to output eg. [2][5-8][12-]\n\t-d <delimiter>\t\t set the input and output delimiter\n\t\t\t\t defaults to ','\n\t-di <input delimiter>\t set the input delimiter\n\t-do <output delimiter>\t set the output delimiter\n\n\t-q <quote character>\t defaults to \"\n\t-qi <quote input character>\n\t-qo <quote output character>\n\n\t-ed \t\t\t dos end of line \\r\\n\n\t-eu \t\t\t unix end of line \\n\n\t-em \t\t\t mac end of line \\r\n\n\t-b \t\t\t allow binary data\n\t-v \t\t\t send processing info to stderr\n\t-h \t\t\t help\n",
            );
            return 0;
        }
        "-v" => config.verbose += 1,
        "-DDD" => config.verbose += 100,
        "-b" => config.allow_binary = true,
        other => {
            eprintln!("error: unknown switch ({})", other);
            return -1;
        }
    }
    arginc += 1;
}

CONFIG.with(|cfg| *cfg.borrow_mut() = config.clone());

let mut reader: Box<dyn BufRead> = match input_path {
    Some(path) => match File::open(&path) {
        Ok(file) => Box::new(BufReader::new(file)),
        Err(_) => {
            eprintln!("error: unable to open ({}) for input", path);
            return -1;
        }
    },
    None => Box::new(BufReader::new(io::stdin())),
};

let mut writer: Box<dyn Write> = match output_path {
    Some(path) => match File::create(&path) {
        Ok(file) => Box::new(file),
        Err(_) => {
            eprintln!("error: unable to open ({}) for output", path);
            return -1;
        }
    },
    None => Box::new(io::stdout()),
};

let mut raw_input = String::new();
loop {
    raw_input.clear();
    match reader.read_line(&mut raw_input) {
        Ok(0) => return 0,
        Ok(_) => {}
        Err(_) => return -1,
    }

    if raw_input.starts_with('#') {
        let _ = writer.write_all(raw_input.as_bytes());
        continue;
    }

    let mut cline = CsvLine::new();
    let mut current = raw_input.clone();
    let mut append_mode = false;
    let mut inside_quote = false;

    loop {
        let mut start_idx = 0_usize;
        let mut completed_line = false;

        while !completed_line {
            let mut field_len = 0_usize;
            let parse_state = get_field(
                &current[start_idx..],
                current.len().saturating_sub(start_idx),
                &mut field_len,
                &mut inside_quote,
            );

            if append_mode {
                cline.append_field(&current, start_idx, field_len);
            } else {
                cline.add_field(&current, start_idx, field_len);
            }

            start_idx += field_len;
            if parse_state == RV_DELIM && start_idx < current.len() {
                start_idx += config.delim_in.len_utf8();
            }
            append_mode = inside_quote;

            if parse_state == RV_STATE_EOL {
                if inside_quote {
                    current.clear();
                    match reader.read_line(&mut current) {
                        Ok(0) => {
                            eprintln!("error: unterminated quote");
                            return -1;
                        }
                        Ok(_) => {
                            append_mode = true;
                            break;
                        }
                        Err(_) => return -1,
                    }
                } else {
                    cline.eol_str = detect_eol(&current).to_string();
                    completed_line = true;
                }
            }
        }

        if !inside_quote && completed_line {
            break;
        }
    }

    let rendered = render_output_line(&cline, &config);
    let _ = writer.write_all(rendered.as_bytes());
}
}

fn detect_eol(line: &str) -> &str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else if line.ends_with('\r') {
        "\r"
    } else {
        ""
    }
}

fn format_output_value(input: &str, cfg: &Config) -> String {
    if cfg.quote_in == cfg.quote_out {
        return input.to_string();
    }
    let chars: Vec<char> = input.chars().collect();
    if chars.len() < 2 {
        return input.to_string();
    }
    if chars.first() == Some(&cfg.quote_in) && chars.last() == Some(&cfg.quote_in) {
        let mut updated = String::with_capacity(input.len());
        updated.push(cfg.quote_out);
        for ch in &chars[1..chars.len() - 1] {
            updated.push(*ch);
        }
        updated.push(cfg.quote_out);
        updated
    } else {
        input.to_string()
    }
}

fn render_output_line(cline: &CsvLine, cfg: &Config) -> String {
    let mut out = String::new();

    if cfg.out_columns.is_none() {
        let field_count = cline.get_field_count();
        for i in 0..field_count {
            let field = format_output_value(cline.get_field(i).unwrap_or(""), cfg);
            out.push_str(&field);
            if i + 1 != field_count {
                out.push(cfg.delim_out);
            }
        }
    } else {
        let mut list = cfg.out_columns.as_deref();
        let col_count = cline.get_field_count();
        while let Some(element) = list {
            match element.rangetype {
                RangeType::Empty => {}
                RangeType::Single => {
                    if element.start > 0 {
                        let field = format_output_value(
                            cline.get_field((element.start - 1) as usize).unwrap_or(""),
                            cfg,
                        );
                        if !field.is_empty() {
                            out.push_str(&field);
                        }
                    }
                    if element.next.is_some() {
                        out.push(cfg.delim_out);
                    }
                }
                RangeType::StartEnd => {
                    if element.start > 0 {
                        let start_idx = (element.start - 1) as usize;
                        let end_idx = element.end as usize;
                        for i in start_idx..end_idx.saturating_sub(1) {
                            let field = format_output_value(cline.get_field(i).unwrap_or(""), cfg);
                            if field.is_empty() {
                                out.push(cfg.delim_out);
                            } else {
                                out.push_str(&field);
                                out.push(cfg.delim_out);
                            }
                        }
                        let last = element.end.saturating_sub(1) as usize;
                        if last < col_count {
                            let field = format_output_value(cline.get_field(last).unwrap_or(""), cfg);
                            if !field.is_empty() {
                                out.push_str(&field);
                            }
                        }
                    }
                    if element.next.is_some() {
                        out.push(cfg.delim_out);
                    }
                }
                RangeType::GreaterEqual => {
                    if col_count > 0 && element.start > 0 {
                        let start_idx = (element.start - 1) as usize;
                        for i in start_idx..col_count.saturating_sub(1) {
                            let field = format_output_value(cline.get_field(i).unwrap_or(""), cfg);
                            if !field.is_empty() {
                                out.push_str(&field);
                                out.push(cfg.delim_out);
                            }
                        }
                        let field = format_output_value(
                            cline.get_field(col_count.saturating_sub(1)).unwrap_or(""),
                            cfg,
                        );
                        if !field.is_empty() {
                            out.push_str(&field);
                        }
                    }
                    if element.next.is_some() {
                        out.push(cfg.delim_out);
                    }
                }
            }
            list = element.next.as_deref();
        }
    }

    out.push_str(cfg.end_line.as_deref().unwrap_or(&cline.eol_str));
    out
}
