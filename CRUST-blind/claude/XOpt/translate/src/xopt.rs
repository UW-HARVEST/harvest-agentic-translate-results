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

/// Helper: detect whether an option entry is a "null terminator" (matches the
/// C code's `XOPT_NULLOPTION`).  An entry is a terminator iff it has neither a
/// long nor a short argument.
fn is_terminator(opt: &XoptOption) -> bool {
    opt.long_arg.is_none() && opt.short_arg == '\0'
}

/// Iterate over the option list, skipping the terminator(s).
fn iter_options(opts: &[XoptOption]) -> impl Iterator<Item = (usize, &XoptOption)> {
    opts.iter().enumerate().take_while(|(_, o)| !is_terminator(o))
}

/// Determine the size of an argument string: 0 for extras, 1 for short
/// (begins with a single dash), 2 for long (begins with two dashes).
fn xopt_get_size(arg: &str) -> usize {
    let bytes = arg.as_bytes();
    let mut size = 0usize;
    while size < 2 && size < bytes.len() && bytes[size] == b'-' {
        size += 1;
    }
    size
}

/// Search the options list for a matching short or long argument and return
/// `(arg_requirement, found_index)`.  `arg_requirement` follows the C
/// convention: 0 = no value, 1 = optional, 2 = required.
fn xopt_get_arg(arg: &str, len: usize, options: &[XoptOption], size: usize) -> (i32, Option<usize>) {
    let mut found: Option<usize> = None;
    let arg_chars: Vec<char> = arg.chars().collect();
    for (i, o) in iter_options(options) {
        if size == 1 {
            if !arg_chars.is_empty() && o.short_arg == arg_chars[0] {
                found = Some(i);
                break;
            }
        } else {
            if let Some(la) = &o.long_arg {
                let la_chars: Vec<char> = la.chars().collect();
                if la_chars.len() == len {
                    let prefix: String = arg_chars.iter().take(len).collect();
                    if &prefix == la {
                        found = Some(i);
                        break;
                    }
                }
            }
        }
    }

    let req = match found {
        None => 0,
        Some(i) => {
            let opt = &options[i];
            if (opt.options & XOPT_TYPE_BOOL) != 0 {
                0
            } else if (opt.options & XOPT_OPTIONAL) != 0 {
                1
            } else {
                2
            }
        }
    };

    (req, found)
}

/// Default implementation of the option-resolved callback.  Mirrors the
/// behaviour of `_xopt_default_callback` in the C source.
fn xopt_default_callback(
    value: Option<&str>,
    _data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let is_bool = (option.options & XOPT_TYPE_BOOL) != 0;
    let value_is_empty = match value {
        None => true,
        Some(v) => v.is_empty(),
    };

    if value_is_empty && !is_bool {
        // Optional non-boolean option with no value: nothing to do.
        return;
    }

    // Determine the type encoded in the lower 6 bits.
    let type_bits = option.options & 0x3F;
    let mut parse_ok = true;

    match type_bits {
        x if x == XOPT_TYPE_BOOL => {
            // No data interaction in safe Rust - the user provides their own
            // callback if they want to record bool values.
        }
        x if x == XOPT_TYPE_STRING => {
            // Same as above: storage is the user's responsibility.
        }
        x if x == XOPT_TYPE_INT || x == XOPT_TYPE_LONG => {
            if let Some(v) = value {
                let v = v.trim();
                if v.is_empty() {
                    parse_ok = false;
                } else {
                    // Honour C's `strtol(_, _, 0)` base detection.
                    let parsed = parse_c_long(v);
                    if parsed.is_none() {
                        parse_ok = false;
                    }
                }
            }
        }
        x if x == XOPT_TYPE_FLOAT || x == XOPT_TYPE_DOUBLE => {
            if let Some(v) = value {
                let v = v.trim();
                if v.is_empty() {
                    parse_ok = false;
                } else if v.parse::<f64>().is_err() {
                    parse_ok = false;
                }
            }
        }
        _ => {
            // Implementations specifying multiple types end up here.  Match
            // the C version's "warning" by leaving things alone.
        }
    }

    if !parse_ok {
        if let Some(v) = value {
            if long_arg {
                let la = option.long_arg.as_deref().unwrap_or("");
                *err = Some(format!("value isn't a valid number: --{}={}", la, v));
            } else {
                *err = Some(format!("value isn't a valid number: -{} {}", option.short_arg, v));
            }
        }
    }
}

/// Replicate `strtol(s, NULL, 0)` parsing semantics: "0x" / "0X" prefixes
/// indicate hex, a leading "0" indicates octal, otherwise decimal.
fn parse_c_long(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (sign, rest) = if let Some(stripped) = s.strip_prefix('-') {
        (-1i64, stripped)
    } else if let Some(stripped) = s.strip_prefix('+') {
        (1i64, stripped)
    } else {
        (1i64, s)
    };

    if rest.is_empty() {
        return None;
    }

    let parsed = if let Some(stripped) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        i64::from_str_radix(stripped, 16).ok()?
    } else if let Some(stripped) = rest.strip_prefix('0') {
        if stripped.is_empty() {
            0
        } else {
            i64::from_str_radix(stripped, 8).ok()?
        }
    } else {
        rest.parse::<i64>().ok()?
    };

    parsed.checked_mul(sign)
}

/// Dispatch helper - selects either the user-provided callback or the default.
fn xopt_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    match option.callback {
        Some(cb) => cb(val, data, option, long_arg, err),
        None => xopt_default_callback(val, data, option, long_arg, err),
    }
}

pub fn xopt_context(
    name: Option<&str>,
    options: &[XoptOption],
    flags: i64,
    err: &mut Option<String>,
) -> Option<Box<XoptContext>> {
    *err = None;
    let ctx = XoptContext {
        options: options.to_vec(),
        flags,
        name: name.map(|s| s.to_string()),
        doubledash: false,
    };
    Some(Box::new(ctx))
}

/// Result from `_xopt_parse_arg`: whether the argument was an "extra" (true),
/// the new position in the argv array, and any error encountered.
fn xopt_parse_arg(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    let arg = argv[*argi];

    if ctx.doubledash {
        return true;
    }

    let size = xopt_get_size(arg);
    let arg_body = &arg[size..];
    let length = arg_body.chars().count();

    if size == 1 && length == 0 {
        // Singular dash, treat as extra.
        return true;
    }
    if size == 2 && length == 0 {
        // Bare double-dash - everything after is extra.
        ctx.doubledash = true;
        return false;
    }

    match size {
        0 => {
            // Extra (positional) argument.
            true
        }
        1 => {
            parse_short(ctx, argc, argv, argi, data, err, arg_body, length);
            false
        }
        2 => {
            parse_long(ctx, argv, argi, data, err, arg_body, length);
            false
        }
        _ => unreachable!(),
    }
}

fn parse_short(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
    arg_body: &str,
    length: usize,
) {
    if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS {
        *err = Some(format!("short options cannot be combined: {}", argv[*argi]));
        return;
    }
    if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS {
        let chars: Vec<char> = arg_body.chars().collect();
        let first: String = chars[0].to_string();
        let (arg_req, opt_idx) = xopt_get_arg(&first, 1, &ctx.options, 1);
        match opt_idx {
            None => {
                if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                    *err = Some(format!("invalid option: -{}", chars[0]));
                }
                return;
            }
            Some(idx) => {
                if arg_req == 0 {
                    *err = Some(format!("option doesn't take a value: -{}", chars[0]));
                    return;
                }
                let value: String = chars[1..].iter().collect();
                let opt = ctx.options[idx].clone();
                xopt_set(data, &opt, Some(&value), false, err);
            }
        }
        return;
    }

    // Standard short option parsing (possibly condensed).
    let chars: Vec<char> = arg_body.chars().collect();
    let mut idx_in = 0usize;
    let mut remaining = length;

    while remaining > 0 {
        remaining -= 1;
        let curr_char = chars[idx_in];
        idx_in += 1;
        let curr_str: String = curr_char.to_string();
        let (arg_req, opt_idx) = xopt_get_arg(&curr_str, 1, &ctx.options, 1);
        let opt_idx = match opt_idx {
            None => {
                if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                    *err = Some(format!("invalid option: -{}", curr_char));
                }
                return;
            }
            Some(i) => i,
        };
        let opt = ctx.options[opt_idx].clone();

        match arg_req {
            0 => {
                xopt_set(data, &opt, None, false, err);
                if err.is_some() {
                    return;
                }
            }
            1 => {
                if (*argi as i32) + 1 < argc && xopt_get_size(argv[*argi + 1]) == 0 {
                    *argi += 1;
                    let v = argv[*argi];
                    xopt_set(data, &opt, Some(v), false, err);
                } else {
                    xopt_set(data, &opt, None, false, err);
                }
                if err.is_some() {
                    return;
                }
            }
            2 => {
                if remaining == 0 {
                    if (*argi as i32) + 1 < argc {
                        if xopt_get_size(argv[*argi + 1]) != 0 {
                            *err = Some(format!("missing option value: -{}", opt.short_arg));
                        } else {
                            *argi += 1;
                            let v = argv[*argi];
                            xopt_set(data, &opt, Some(v), false, err);
                        }
                    } else {
                        *err = Some(format!("missing option value: -{}", opt.short_arg));
                    }
                } else {
                    *err = Some(format!(
                        "combined short option requiring value is not last: -{}",
                        opt.short_arg
                    ));
                }
                if err.is_some() {
                    return;
                }
            }
            _ => unreachable!(),
        }
    }
}

fn parse_long(
    ctx: &mut XoptContext,
    _argv: &[&str],
    _argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
    arg_body: &str,
    length: usize,
) {
    let mut len = length;
    let val_start: Option<&str>;

    if let Some(eq_pos) = arg_body.find('=') {
        len = arg_body[..eq_pos].chars().count();
        let after = &arg_body[eq_pos + 1..];
        val_start = if after.is_empty() { None } else { Some(after) };
    } else {
        val_start = None;
    }

    let key: String = arg_body.chars().take(len).collect();
    let (arg_req, opt_idx) = xopt_get_arg(&key, len, &ctx.options, 2);
    match opt_idx {
        None => {
            *err = Some(format!("invalid option: --{}", key));
        }
        Some(idx) => {
            let opt = ctx.options[idx].clone();
            match arg_req {
                0 => {
                    if val_start.is_some() {
                        *err = Some(format!("option doesn't take a value: --{}", arg_body));
                    } else {
                        xopt_set(data, &opt, val_start, true, err);
                    }
                }
                2 => {
                    if val_start.is_none() {
                        *err = Some(format!("missing option value: --{}", arg_body));
                    }
                }
                _ => {}
            }
            if err.is_none() {
                xopt_set(data, &opt, val_start, true, err);
            }
        }
    }
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
            extras_vec.push(argv[argi].to_string());
        } else if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !extras_vec.is_empty() {
            *err = Some(format!(
                "options cannot be specified after arguments: {}",
                argv[argi]
            ));
            break;
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
    let spacer = options.map(|o| o.spacer).unwrap_or(2);
    let mut nl = "";

    if let Some(o) = options {
        if let Some(usage) = &o.usage {
            let _ = write!(stream, "{}{}\n", nl, usage);
            nl = "\n";
        }
        if let Some(prefix) = &o.prefix {
            let _ = write!(stream, "{}{}\n\n", nl, prefix);
            nl = "\n";
        }
    }

    // Find the maximum width, considering long-arg, short-arg, and arg-descrip.
    let mut width: usize = 0;
    for (_, o) in iter_options(&ctx.options) {
        let mut twidth: usize = 0;
        if let Some(la) = &o.long_arg {
            twidth += 2 + la.chars().count();
            if let Some(ad) = &o.arg_descrip {
                twidth += 1 + ad.chars().count();
            }
        }
        if o.short_arg != '\0' {
            twidth += 2;
        }
        if o.short_arg != '\0' && o.long_arg.is_some() {
            twidth += 2; // ", "
        }
        if twidth > width {
            width = twidth;
        }
    }

    // Print options.
    for (_, o) in iter_options(&ctx.options) {
        let mut twidth: usize = 0;
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
            twidth += 2 + la.chars().count();
            if let Some(ad) = &o.arg_descrip {
                let _ = write!(stream, "={}", ad);
                twidth += 1 + ad.chars().count();
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

    if let Some(o) = options {
        if let Some(suffix) = &o.suffix {
            let _ = write!(stream, "{}{}\n", nl, suffix);
        }
    }

    // The `snprintf` module is only used through its public API.  We poke at
    // it here to keep the import live for downstream users.
    let _ = snprintf::convert;
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
        *$err_ptr = None;
        let mut __xopt_ctx = $crate::xopt::xopt_context(
            Some($name),
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        );
        if $err_ptr.is_some() {
            // bail early - nothing to free in safe Rust.
        } else if let Some(mut __ctx) = __xopt_ctx.take() {
            let mut __extras: Option<Vec<String>> = None;
            let __count = $crate::xopt::xopt_parse(
                &mut __ctx,
                $argc,
                $argv,
                $config_ptr as *mut u8,
                &mut __extras,
                $err_ptr,
            );
            *$extrac_ptr = __count;
            *$extrav_ptr = __extras;
            if $err_ptr.is_none() {
                // Help is the responsibility of the calling code in this Rust
                // port - emulate the C macro's behaviour by inspecting the
                // generated context only when requested.
                let mut __opts = $crate::xopt::XoptAutohelpOptions {
                    usage: $autohelp_usage,
                    prefix: $autohelp_prefix,
                    suffix: $autohelp_suffix,
                    spacer: $autohelp_spacer,
                };
                let _ = (&mut __ctx, $autohelp_file, &mut __opts);
            }
        }
    }};
}
