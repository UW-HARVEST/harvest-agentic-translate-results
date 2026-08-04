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

fn set_err(err: &mut Option<String>, fmt: &str, args: &[&str]) {
    let mut s = String::new();
    snprintf::rpl_vsnprintf(&mut s, 4096, fmt, args);
    *err = Some(s);
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

/// Returns the number of leading dashes (0, 1, or 2).
fn xopt_get_size(arg: &str) -> i32 {
    let mut size = 0;
    for ch in arg.chars().take(2) {
        if ch != '-' {
            break;
        }
        size += 1;
    }
    size
}

/// Looks up the option by either short character (size==1) or long name.
/// Returns (argRequirement, Option<index>).
///   * 0 -> flag (no value)
///   * 1 -> optional value
///   * 2 -> requires value
fn xopt_get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: i32,
) -> (i32, Option<usize>) {
    let mut found: Option<usize> = None;
    let arg_chars: Vec<char> = arg.chars().collect();

    for (i, opt) in options.iter().enumerate() {
        // Stop when we hit the null terminator.
        if opt.long_arg.is_none() && opt.short_arg == '\0' {
            break;
        }
        if size == 1 {
            if opt.short_arg != '\0' && !arg_chars.is_empty() && opt.short_arg == arg_chars[0] {
                found = Some(i);
                break;
            }
        } else {
            // size == 2; long arg
            if let Some(la) = &opt.long_arg {
                if la.chars().count() == len {
                    let prefix: String = arg.chars().take(len).collect();
                    if *la == prefix {
                        found = Some(i);
                        break;
                    }
                }
            }
        }
    }

    match found {
        None => (0, None),
        Some(idx) => {
            let opt = &options[idx];
            if (opt.options & XOPT_TYPE_BOOL) != 0 {
                (0, Some(idx))
            } else if (opt.options & XOPT_OPTIONAL) != 0 {
                (1, Some(idx))
            } else {
                (2, Some(idx))
            }
        }
    }
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
        default_callback(val, data, option, long_arg, err);
    }
}

fn default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    // If a value is required (non-bool) but missing, just return without writing.
    let value_str = value.unwrap_or("");
    let has_value = !value_str.is_empty();

    if !has_value && (option.options & XOPT_TYPE_BOOL) == 0 {
        return;
    }

    // Compute target pointer.
    let target = unsafe { data.add(option.offset) };

    let type_bits = option.options & 0x3F;
    let mut parse_failed = false;
    match type_bits {
        x if x == XOPT_TYPE_BOOL => {
            unsafe {
                *(target as *mut bool) = true;
            }
        }
        x if x == XOPT_TYPE_STRING => {
            // Strings are tricky in safe Rust because the lifetime of the source
            // string is tied to argv. We won't write a Rust pointer here; the
            // user-supplied data layout should be `*const u8` or similar. We
            // store the raw pointer to the bytes of `value_str` (assuming it
            // lives long enough). This mirrors the C behavior.
            unsafe {
                *(target as *mut *const u8) = value_str.as_ptr();
            }
        }
        x if x == XOPT_TYPE_INT => match parse_int(value_str) {
            Ok(v) => unsafe {
                *(target as *mut i32) = v as i32;
            },
            Err(_) => parse_failed = true,
        },
        x if x == XOPT_TYPE_LONG => match parse_int(value_str) {
            Ok(v) => unsafe {
                *(target as *mut i64) = v;
            },
            Err(_) => parse_failed = true,
        },
        x if x == XOPT_TYPE_FLOAT => match value_str.parse::<f64>() {
            Ok(v) => unsafe {
                *(target as *mut f32) = v as f32;
            },
            Err(_) => parse_failed = true,
        },
        x if x == XOPT_TYPE_DOUBLE => match value_str.parse::<f64>() {
            Ok(v) => unsafe {
                *(target as *mut f64) = v;
            },
            Err(_) => parse_failed = true,
        },
        _ => {
            eprintln!(
                "warning: XOpt argument type invalid: {}",
                option.options & 0x2F
            );
        }
    }

    if parse_failed {
        if long_arg {
            let la = option.long_arg.as_deref().unwrap_or("");
            set_err(
                err,
                "value isn't a valid number: --%s=%s",
                &[la, value_str],
            );
        } else {
            let ch_buf = option.short_arg.to_string();
            set_err(
                err,
                "value isn't a valid number: -%c %s",
                &[&ch_buf, value_str],
            );
        }
    }
}

/// Parse an integer from a C-style string supporting "0x" hex and "0" octal
/// prefixes, mirroring `strtol(..., 0)`.
fn parse_int(s: &str) -> Result<i64, ()> {
    let s = s.trim();
    if s.is_empty() {
        return Err(());
    }
    let (sign, rest) = if let Some(stripped) = s.strip_prefix('-') {
        (-1i64, stripped)
    } else if let Some(stripped) = s.strip_prefix('+') {
        (1i64, stripped)
    } else {
        (1i64, s)
    };

    let (radix, digits) = if let Some(d) = rest.strip_prefix("0x").or(rest.strip_prefix("0X")) {
        (16u32, d)
    } else if let Some(d) = rest.strip_prefix('0') {
        if d.is_empty() {
            return Ok(0);
        }
        (8u32, d)
    } else {
        (10u32, rest)
    };

    if digits.is_empty() {
        return Err(());
    }
    match i64::from_str_radix(digits, radix) {
        Ok(v) => Ok(sign * v),
        Err(_) => Err(()),
    }
}

/// Process a single argv entry. Returns `Ok(true)` if the entry was an
/// "extra" (non-option) argument, `Ok(false)` if it was an option, or `Err`
/// indicating a parse failure (the error is also stored in `err`).
fn parse_arg(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    argi: &mut i32,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    let mut is_extra = false;
    let arg_full = argv[*argi as usize];

    if ctx.doubledash {
        return true;
    }

    let size = xopt_get_size(arg_full);
    let arg: &str = &arg_full[size as usize..];
    let length = arg.chars().count();

    if size == 1 && length == 0 {
        return true;
    }

    if size == 2 && length == 0 {
        ctx.doubledash = true;
        return false;
    }

    match size {
        1 => {
            // short option
            let chars: Vec<char> = arg.chars().collect();
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS
            {
                set_err(err, "short options cannot be combined: %s", &[arg_full]);
            } else if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS {
                let single_char: String = chars[0..1].iter().collect();
                let (arg_req, opt_idx) = xopt_get_arg(&single_char, 1, &ctx.options, size);
                if opt_idx.is_none() {
                    if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                        let c_buf = chars[0].to_string();
                        set_err(err, "invalid option: -%c", &[&c_buf]);
                    }
                    return is_extra;
                }
                if arg_req == 0 {
                    let c_buf = chars[0].to_string();
                    set_err(err, "option doesn't take a value: -%c", &[&c_buf]);
                    return is_extra;
                }
                let value: String = chars[1..].iter().collect();
                let opt = ctx.options[opt_idx.unwrap()].clone();
                xopt_set(data, &opt, Some(&value), false, err);
            } else {
                // parse all condensed
                let mut idx = 0usize;
                let mut remaining = length;
                while remaining > 0 {
                    let cur_char: String = chars[idx..idx + 1].iter().collect();
                    idx += 1;
                    remaining -= 1;
                    let (arg_req, opt_idx) = xopt_get_arg(&cur_char, 1, &ctx.options, size);
                    if opt_idx.is_none() {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            set_err(err, "invalid option: -%c", &[&cur_char]);
                        }
                        break;
                    }
                    let opt = ctx.options[opt_idx.unwrap()].clone();
                    match arg_req {
                        0 => {
                            xopt_set(data, &opt, None, false, err);
                        }
                        1 => {
                            // optional value
                            if *argi + 1 < argc
                                && xopt_get_size(argv[(*argi + 1) as usize]) == 0
                            {
                                *argi += 1;
                                let v = argv[*argi as usize];
                                xopt_set(data, &opt, Some(v), false, err);
                            } else {
                                xopt_set(data, &opt, None, false, err);
                            }
                        }
                        2 => {
                            if remaining == 0 {
                                if *argi + 1 < argc {
                                    if xopt_get_size(argv[(*argi + 1) as usize]) != 0 {
                                        let c_buf = opt.short_arg.to_string();
                                        set_err(
                                            err,
                                            "missing option value: -%c",
                                            &[&c_buf],
                                        );
                                    } else {
                                        *argi += 1;
                                        let v = argv[*argi as usize];
                                        xopt_set(data, &opt, Some(v), false, err);
                                    }
                                } else {
                                    let c_buf = opt.short_arg.to_string();
                                    set_err(err, "missing option value: -%c", &[&c_buf]);
                                }
                            } else {
                                let c_buf = opt.short_arg.to_string();
                                set_err(
                                    err,
                                    "combined short option requiring value is not last: -%c",
                                    &[&c_buf],
                                );
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
            // long option
            let mut name = arg.to_string();
            let mut value: Option<String> = None;
            if let Some(eq) = arg.find('=') {
                name = arg[..eq].to_string();
                let val_part = &arg[eq + 1..];
                if val_part.is_empty() {
                    value = None;
                } else {
                    value = Some(val_part.to_string());
                }
            }
            let name_len = name.chars().count();
            let (arg_req, opt_idx) = xopt_get_arg(&name, name_len, &ctx.options, size);
            if opt_idx.is_none() {
                set_err(err, "invalid option: --%s", &[&name]);
            } else {
                let opt = ctx.options[opt_idx.unwrap()].clone();
                match arg_req {
                    0 => {
                        if value.is_some() {
                            set_err(
                                err,
                                "option doesn't take a value: --%s",
                                &[&name],
                            );
                        } else {
                            xopt_set(data, &opt, value.as_deref(), true, err);
                        }
                    }
                    2 => {
                        if value.is_none() {
                            set_err(err, "missing option value: --%s", &[&name]);
                        }
                    }
                    _ => {}
                }
                if err.is_none() {
                    xopt_set(data, &opt, value.as_deref(), true, err);
                }
            }
        }
        0 => {
            is_extra = true;
        }
        _ => {}
    }

    is_extra
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
    let mut argi: i32 = 0;
    let mut extras_vec: Vec<String> = Vec::new();

    if (ctx.flags & XOPT_CTX_KEEPFIRST) == 0 {
        argi += 1;
    }

    while argi < argc {
        let is_extra = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }
        if is_extra {
            extras_vec.push(argv[argi as usize].to_string());
        } else if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !extras_vec.is_empty() {
            set_err(
                err,
                "options cannot be specified after arguments: %s",
                &[argv[argi as usize]],
            );
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

    // Find max width.
    let mut width: usize = 0;
    for o in &ctx.options {
        if o.long_arg.is_none() && o.short_arg == '\0' {
            break;
        }
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
            twidth += 2;
        }
        if twidth > width {
            width = twidth;
        }
    }

    for o in &ctx.options {
        if o.long_arg.is_none() && o.short_arg == '\0' {
            break;
        }
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

    if let Some(opts) = options {
        if let Some(suffix) = &opts.suffix {
            let _ = write!(stream, "{}{}\n", nl, suffix);
        }
    }
    let _ = nl; // silence unused warnings if relevant.
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
        if let Some(mut _ctx) = $crate::xopt::xopt_context(
            $name,
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        ) {
            if $err_ptr.is_none() {
                let count = $crate::xopt::xopt_parse(
                    &mut _ctx,
                    $argc,
                    $argv,
                    $config_ptr as *mut u8,
                    $extrav_ptr,
                    $err_ptr,
                );
                *$extrac_ptr = count;
            }
        }
    }};
}
