use crate::snprintf;
pub const XOPT_TYPE_STRING: i64 = 0x1;   // const char * type
pub const XOPT_TYPE_INT: i64    = 0x2;   // int type
pub const XOPT_TYPE_LONG: i64   = 0x4;   // long type
pub const XOPT_TYPE_FLOAT: i64  = 0x8;   // float type
pub const XOPT_TYPE_DOUBLE: i64 = 0x10;  // double type
pub const XOPT_TYPE_BOOL: i64   = 0x20;  // boolean (int) type
/// Indicates that the argument value is optional.
pub const XOPT_OPTIONAL: i64    = 0x40;
/// Bitmask constants for context flags.
pub const XOPT_CTX_KEEPFIRST: i64     = 0x1;
pub const XOPT_CTX_POSIXMEHARDER: i64 = 0x2;
pub const XOPT_CTX_NOCONDENSE: i64    = 0x4;
/// `XOPT_CTX_SLOPPYSHORTS` is defined as 0x8 | 0x4 in C
pub const XOPT_CTX_SLOPPYSHORTS: i64  = 0x8 | XOPT_CTX_NOCONDENSE;
pub const XOPT_CTX_STRICT: i64        = 0x10;
pub type XoptCallback = fn(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
);
#[derive(Debug, Clone)]
pub struct XoptOption {
    /// Matches the `--longArg`; `None` means no long argument.
    pub long_arg: Option<String>,
    /// Matches the single `-s` short argument.  If `'\0'`, there is no short argument.
    pub short_arg: char,
    /// Matches the original `offsetof(...)` usage.  In Rust, we typically do not
    /// manually offset into structs, but we keep this for compatibility.
    pub offset: usize,
    /// Callback for resolved option handling.  May be `None` if not specified.
    pub callback: Option<XoptCallback>,
    /// Bitmask of `XOPT_TYPE_*` and possibly `XOPT_OPTIONAL`.
    pub options: i64,
    /// For help text: `--argument=argDescrip`.
    pub arg_descrip: Option<String>,
    /// For help text: descriptive explanation.
    pub descrip: Option<String>,
}
pub const fn XOPT_NULLOPTION() -> XoptOption {
    XoptOption {
        long_arg: None,
        short_arg: '\0',
        offset: 0,
        callback: None,
        options: 0,
        arg_descrip: None,
        descrip: None,
    }
}
#[derive(Debug)]
pub struct XoptContext {
    /// In the C code, `const xoptOption *options;`
    pub options: Vec<XoptOption>,
    /// The bitflags for context configuration.
    pub flags: i64,
    /// The `name` from the original code (like the CLI binary name).
    pub name: Option<String>,
    /// Tracks whether `--` was encountered, in the C code.
    pub doubledash: bool,
}
#[derive(Debug, Clone)]
pub struct XoptAutohelpOptions {
    /// Usage string or `None` if not specified.
    pub usage: Option<String>,
    /// Printed before the options list, or `None`.
    pub prefix: Option<String>,
    /// Printed after the options list, or `None`.
    pub suffix: Option<String>,
    /// Number of spaces between option and description.
    pub spacer: usize,
}

/// Determine if an option entry is the terminator (i.e. `XOPT_NULLOPTION`).
fn is_terminator(o: &XoptOption) -> bool {
    o.long_arg.is_none() && o.short_arg == '\0'
}

/// Returns 0 if extra, 1 if short arg ("-x"), 2 if long arg ("--xx").
fn xopt_get_size(arg: &str) -> usize {
    let bytes = arg.as_bytes();
    let mut size = 0usize;
    while size < 2 && size < bytes.len() && bytes[size] == b'-' {
        size += 1;
    }
    size
}

/// Look up an option by name. Returns:
/// - `(arg_requirement, Some(index))` if found
/// - `(0, None)` if not found
///
/// `arg_requirement`: 0 if option is a flag (BOOL or no option found),
/// 1 if optional value, 2 if required value.
fn xopt_get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: usize,
) -> (i32, Option<usize>) {
    let mut found: Option<usize> = None;
    let arg_bytes = arg.as_bytes();

    for (i, opt) in options.iter().enumerate() {
        if is_terminator(opt) {
            break;
        }
        if size == 1 {
            if opt.short_arg != '\0' && !arg_bytes.is_empty() {
                let c = arg_bytes[0] as char;
                if opt.short_arg == c {
                    found = Some(i);
                    break;
                }
            }
        } else {
            // Long arg compare: strlen(longArg) == len && strncmp == 0
            if let Some(la) = &opt.long_arg {
                if la.len() == len && arg_bytes.len() >= len && la.as_bytes() == &arg_bytes[..len]
                {
                    found = Some(i);
                    break;
                }
            }
        }
    }

    let req = match found {
        None => 0,
        Some(i) => {
            let opt = &options[i];
            if opt.options & XOPT_TYPE_BOOL != 0 {
                0
            } else if opt.options & XOPT_OPTIONAL != 0 {
                1
            } else {
                2
            }
        }
    };
    (req, found)
}

/// Default callback - sets the value at `data + option.offset` based on type.
fn xopt_default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let value_is_empty = match value {
        None => true,
        Some(s) => s.is_empty(),
    };

    if value_is_empty && (option.options & XOPT_TYPE_BOOL) == 0 {
        return;
    }

    // Compute target pointer
    let target = unsafe { data.add(option.offset) };

    let v = value.unwrap_or("");
    let mut parse_err: Option<String> = None;

    match option.options & 0x3F {
        x if x == XOPT_TYPE_BOOL => {
            // bool is _Bool which is one byte
            unsafe {
                *(target as *mut bool) = true;
            }
        }
        x if x == XOPT_TYPE_STRING => {
            // We can't really support storing a *const c_char in our pure Rust setting easily
            // but we'll write the raw pointer of the string slice as best-effort.
            unsafe {
                *(target as *mut *const u8) = v.as_ptr();
            }
        }
        x if x == XOPT_TYPE_INT => match parse_c_long(v) {
            Some((n, ok)) => {
                unsafe {
                    *(target as *mut i32) = n as i32;
                }
                if !ok {
                    parse_err = Some(v.to_string());
                }
            }
            None => parse_err = Some(v.to_string()),
        },
        x if x == XOPT_TYPE_LONG => match parse_c_long(v) {
            Some((n, ok)) => {
                unsafe {
                    *(target as *mut i64) = n;
                }
                if !ok {
                    parse_err = Some(v.to_string());
                }
            }
            None => parse_err = Some(v.to_string()),
        },
        x if x == XOPT_TYPE_FLOAT => match parse_c_double(v) {
            Some((n, ok)) => {
                unsafe {
                    *(target as *mut f32) = n as f32;
                }
                if !ok {
                    parse_err = Some(v.to_string());
                }
            }
            None => parse_err = Some(v.to_string()),
        },
        x if x == XOPT_TYPE_DOUBLE => match parse_c_double(v) {
            Some((n, ok)) => {
                unsafe {
                    *(target as *mut f64) = n;
                }
                if !ok {
                    parse_err = Some(v.to_string());
                }
            }
            None => parse_err = Some(v.to_string()),
        },
        _ => {
            eprintln!(
                "warning: XOpt argument type invalid: {}",
                option.options & 0x2F
            );
        }
    }

    if let Some(_) = parse_err {
        if long_arg {
            *err = Some(format!(
                "value isn't a valid number: --{}={}",
                option.long_arg.as_deref().unwrap_or(""),
                v
            ));
        } else {
            *err = Some(format!(
                "value isn't a valid number: -{} {}",
                option.short_arg, v
            ));
        }
    }
}

/// Mimic `strtol(value, &parsePtr, 0)` — returns Some((parsed_long, fully_consumed))
/// where fully_consumed is true if strtol fully consumes the string.
fn parse_c_long(s: &str) -> Option<(i64, bool)> {
    if s.is_empty() {
        return None;
    }
    let bytes = s.as_bytes();
    let mut idx = 0;
    // Skip whitespace
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }
    let start = idx;
    let mut neg = false;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        neg = bytes[idx] == b'-';
        idx += 1;
    }
    // Determine base (0 = auto-detect)
    let base: u32;
    if idx + 1 < bytes.len() && bytes[idx] == b'0' && (bytes[idx + 1] == b'x' || bytes[idx + 1] == b'X')
    {
        base = 16;
        idx += 2;
    } else if idx < bytes.len() && bytes[idx] == b'0' {
        base = 8;
        idx += 1;
    } else {
        base = 10;
    }

    let digits_start = idx;
    let mut value: i64 = 0;
    while idx < bytes.len() {
        let c = bytes[idx];
        let d = match c {
            b'0'..=b'9' => (c - b'0') as u32,
            b'a'..=b'f' => (c - b'a' + 10) as u32,
            b'A'..=b'F' => (c - b'A' + 10) as u32,
            _ => break,
        };
        if d >= base {
            break;
        }
        value = value.saturating_mul(base as i64).saturating_add(d as i64);
        idx += 1;
    }

    if idx == digits_start {
        // No digits — strtol returns 0 and parsePtr == s, treat as fully invalid (parsePtr != end)
        // In C, parsePtr would be == s, and *parsePtr is the original first char.
        // If value is empty/no number => *parsePtr != '\0' => treated as parse error.
        // But our caller checks parse_err with "fully_consumed = false" => mark error.
        return Some((0, idx >= bytes.len() && start == idx));
    }

    if neg {
        value = value.wrapping_neg();
    }
    let fully = idx >= bytes.len();
    Some((value, fully))
}

/// Mimic `strtod(value, &parsePtr)` — returns Some((parsed_double, fully_consumed)).
fn parse_c_double(s: &str) -> Option<(f64, bool)> {
    if s.is_empty() {
        return None;
    }
    let bytes = s.as_bytes();
    let mut idx = 0usize;
    // Skip whitespace
    while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
        idx += 1;
    }
    let start = idx;
    if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
        idx += 1;
    }
    let mut had_digit = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
        had_digit = true;
    }
    if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
            had_digit = true;
        }
    }
    if had_digit && idx < bytes.len() && (bytes[idx] == b'e' || bytes[idx] == b'E') {
        let exp_start = idx;
        idx += 1;
        if idx < bytes.len() && (bytes[idx] == b'+' || bytes[idx] == b'-') {
            idx += 1;
        }
        let dig_idx = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        if dig_idx == idx {
            // no digits in exponent — back off
            idx = exp_start;
        }
    }
    if !had_digit {
        return Some((0.0, false));
    }
    let parsed_str = std::str::from_utf8(&bytes[start..idx]).ok()?;
    let value: f64 = parsed_str.parse().ok()?;
    let fully = idx >= bytes.len();
    Some((value, fully))
}

fn xopt_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    if let Some(cb) = option.callback {
        cb(val, data, option, long_arg, err);
    } else {
        xopt_default_callback(val, data, option, long_arg, err);
    }
}

/// Internal: parse a single argument. Returns true if the argument is an extra,
/// false if it's an option.
fn xopt_parse_arg(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    if ctx.doubledash {
        return true;
    }

    let arg_full = argv[*argi];
    let size = xopt_get_size(arg_full);
    let arg = &arg_full[size..];
    let length = arg.len();

    if size == 1 && length == 0 {
        // singular dash
        return true;
    }
    if size == 2 && length == 0 {
        // double-dash; everything after is extras
        ctx.doubledash = true;
        return false;
    }

    let mut is_extra = false;

    match size {
        1 => {
            // Short option
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS {
                *err = Some(format!("short options cannot be combined: {}", arg_full));
            } else if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS {
                let (arg_req, opt_idx) = xopt_get_arg(arg, 1, &ctx.options, size);
                if opt_idx.is_none() {
                    if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                        *err = Some(format!("invalid option: -{}", arg.chars().next().unwrap_or('\0')));
                    }
                    return is_extra;
                }
                if arg_req == 0 {
                    *err = Some(format!(
                        "option doesn't take a value: -{}",
                        arg.chars().next().unwrap_or('\0')
                    ));
                    return is_extra;
                }
                let opt = ctx.options[opt_idx.unwrap()].clone();
                xopt_set(data, &opt, Some(&arg[1..]), false, err);
            } else {
                // Parse all (potentially condensed)
                let arg_bytes = arg.as_bytes();
                let mut local_len = length;
                let mut ai = 0usize;
                while local_len > 0 {
                    let cur = std::str::from_utf8(&arg_bytes[ai..ai + 1]).unwrap_or("");
                    let (arg_req, opt_idx) = xopt_get_arg(cur, 1, &ctx.options, size);
                    ai += 1;
                    local_len -= 1;
                    if opt_idx.is_none() {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            let prev = arg_bytes[ai - 1] as char;
                            *err = Some(format!("invalid option: -{}", prev));
                        }
                        break;
                    }
                    let opt_idx = opt_idx.unwrap();
                    let opt = ctx.options[opt_idx].clone();
                    match arg_req {
                        0 => {
                            // flag, no argument
                            xopt_set(data, &opt, None, false, err);
                        }
                        1 => {
                            // optional argument
                            if (*argi as i32 + 1) < argc
                                && xopt_get_size(argv[*argi + 1]) == 0
                            {
                                *argi += 1;
                                let v = argv[*argi];
                                xopt_set(data, &opt, Some(v), false, err);
                            } else {
                                xopt_set(data, &opt, None, false, err);
                            }
                        }
                        2 => {
                            // requires an argument
                            if local_len == 0 {
                                if (*argi as i32 + 1) < argc {
                                    if xopt_get_size(argv[*argi + 1]) != 0 {
                                        *err = Some(format!(
                                            "missing option value: -{}",
                                            opt.short_arg
                                        ));
                                    } else {
                                        *argi += 1;
                                        let v = argv[*argi];
                                        xopt_set(data, &opt, Some(v), false, err);
                                    }
                                } else {
                                    *err = Some(format!(
                                        "missing option value: -{}",
                                        opt.short_arg
                                    ));
                                }
                            } else {
                                *err = Some(format!(
                                    "combined short option requiring value is not last: -{}",
                                    opt.short_arg
                                ));
                            }
                        }
                        _ => {}
                    }
                    if err.is_some() {
                        break;
                    }
                }
            }
        }
        2 => {
            // Long option
            let (name_part, val_part): (&str, Option<&str>) = match arg.find('=') {
                Some(eq_idx) => {
                    let value = &arg[eq_idx + 1..];
                    let v = if value.is_empty() { None } else { Some(value) };
                    (&arg[..eq_idx], v)
                }
                None => (arg, None),
            };
            let name_len = name_part.len();
            let (arg_req, opt_idx) = xopt_get_arg(name_part, name_len, &ctx.options, size);
            if opt_idx.is_none() {
                *err = Some(format!("invalid option: --{}", name_part));
            } else {
                let opt = ctx.options[opt_idx.unwrap()].clone();
                match arg_req {
                    0 => {
                        if val_part.is_some() {
                            *err = Some(format!(
                                "option doesn't take a value: --{}",
                                arg
                            ));
                        }
                        if err.is_none() {
                            xopt_set(data, &opt, val_part, true, err);
                        }
                    }
                    2 => {
                        if val_part.is_none() {
                            *err = Some(format!("missing option value: --{}", arg));
                        }
                        if err.is_none() {
                            xopt_set(data, &opt, val_part, true, err);
                        }
                    }
                    _ => {
                        if err.is_none() {
                            xopt_set(data, &opt, val_part, true, err);
                        }
                    }
                }
            }
        }
        _ => {
            // extra
            is_extra = true;
        }
    }

    is_extra
}

pub fn xopt_context(
    name: Option<&str>,
    options: &[XoptOption],
    flags: i64,
    err: &mut Option<String>,
) -> Option<Box<XoptContext>> {
    *err = None;
    Some(Box::new(XoptContext {
        options: options.to_vec(),
        flags,
        name: name.map(|s| s.to_string()),
        doubledash: false,
    }))
}

pub fn xopt_parse(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    data: *mut u8,
    extras: &mut Option<Vec<String>>,
    err: &mut Option<String>,
) -> i32 {
    *err = None;
    let mut argi: usize = 0;
    let mut extras_vec: Vec<String> = Vec::new();

    if (ctx.flags & XOPT_CTX_KEEPFIRST) == 0 {
        argi += 1;
    }

    while (argi as i32) < argc {
        let parse_result = xopt_parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }
        if parse_result {
            // extra
            extras_vec.push(argv[argi].to_string());
        } else {
            // option — POSIX-strict: don't allow options after extras
            if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !extras_vec.is_empty() {
                *err = Some(format!(
                    "options cannot be specified after arguments: {}",
                    argv[argi]
                ));
                break;
            }
        }
        argi += 1;
    }

    if err.is_some() {
        *extras = None;
        return 0;
    }

    let count = extras_vec.len() as i32;
    *extras = Some(extras_vec);
    count
}

pub fn xopt_autohelp(
    ctx: &mut XoptContext,
    stream: &mut dyn std::io::Write,
    options: Option<&XoptAutohelpOptions>,
    err: &mut Option<String>,
) {
    *err = None;
    let mut nl = "";
    let spacer = options.map(|o| o.spacer).unwrap_or(2);

    if let Some(opts) = options {
        if let Some(usage) = &opts.usage {
            let _ = write!(stream, "{}{}\n", nl, usage);
            nl = "\n";
        }
        if let Some(prefix) = &opts.prefix {
            let _ = write!(stream, "{}{}\n\n", nl, prefix);
            nl = "\n";
        }
    }

    // Determine max width
    let mut width = 0usize;
    for o in ctx.options.iter() {
        if is_terminator(o) {
            break;
        }
        let mut twidth = 0usize;
        if let Some(la) = &o.long_arg {
            twidth += 2 + la.len();
            if let Some(ad) = &o.arg_descrip {
                twidth += 1 + ad.len();
            }
        }
        if o.short_arg != '\0' {
            twidth += 2;
        }
        if o.short_arg != '\0' && o.long_arg.is_some() {
            twidth += 2;
        }
        if twidth > width {
            width = twidth;
        }
    }

    for o in ctx.options.iter() {
        if is_terminator(o) {
            break;
        }
        let mut twidth = 0usize;
        if o.short_arg != '\0' {
            let _ = write!(stream, "-{}", o.short_arg);
            twidth += 2;
        }
        if o.short_arg != '\0' && o.long_arg.is_some() {
            let _ = write!(stream, ", ");
            twidth += 2;
        }
        if let Some(la) = &o.long_arg {
            let _ = write!(stream, "--{}", la);
            twidth += 2 + la.len();
            if let Some(ad) = &o.arg_descrip {
                let _ = write!(stream, "={}", ad);
                twidth += 1 + ad.len();
            }
        }
        if let Some(d) = &o.descrip {
            while twidth < width + spacer {
                let _ = write!(stream, " ");
                twidth += 1;
            }
            let _ = write!(stream, "{}\n", d);
        }
    }

    if let Some(opts) = options {
        if let Some(suffix) = &opts.suffix {
            let _ = write!(stream, "{}{}\n", nl, suffix);
        }
    }

    // Reference snprintf module to keep import valid.
    let _ = &snprintf::getnumsep;
}
#[macro_export]
macro_rules! XOPT_SIMPLE_PARSE {
    (
      $name:expr,
      $options:expr,
      $config_ptr:expr,
      $argc:expr,
      $argv:expr,
      $extrac_ptr:expr,
      $extrav_ptr:expr,
      $err_ptr:expr,
      $autohelp_file:expr,
      $autohelp_usage:expr,
      $autohelp_prefix:expr,
      $autohelp_suffix:expr,
      $autohelp_spacer:expr
    ) => {{
        let mut __xopt_ctx = $crate::xopt::xopt_context(
            $name,
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        );
        if $err_ptr.is_none() {
            if let Some(ref mut __ctx) = __xopt_ctx {
                let __count = $crate::xopt::xopt_parse(
                    __ctx,
                    $argc,
                    $argv,
                    $config_ptr,
                    $extrav_ptr,
                    $err_ptr,
                );
                *$extrac_ptr = __count;
            }
        }
    }};
}
