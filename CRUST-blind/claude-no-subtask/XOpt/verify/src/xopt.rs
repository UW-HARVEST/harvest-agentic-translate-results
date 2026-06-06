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
pub fn xopt_context(
    name: Option<&str>,
    options: &[XoptOption],
    flags: i64,
    err: &mut Option<String>,
) -> Option<Box<XoptContext>> {
    *err = None;
    let ctx = Box::new(XoptContext {
        options: options.to_vec(),
        flags,
        name: name.map(|s| s.to_string()),
        doubledash: false,
    });
    Some(ctx)
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
        let parse_result = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }

        if parse_result {
            // It's an "extra" argument.
            if (argi as usize) < argv.len() {
                extras_vec.push(argv[argi as usize].to_string());
            }
        } else {
            // It's an option. If POSIXMEHARDER is set, options must come first.
            if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !extras_vec.is_empty() {
                let arg_str = if (argi as usize) < argv.len() {
                    argv[argi as usize]
                } else {
                    ""
                };
                set_err(
                    err,
                    &format!("options cannot be specified after arguments: {}", arg_str),
                );
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

fn set_err(err: &mut Option<String>, msg: &str) {
    *err = Some(msg.to_string());
}

fn get_size(arg: &str) -> i32 {
    let mut size = 0i32;
    let bytes = arg.as_bytes();
    while size < 2 && (size as usize) < bytes.len() && bytes[size as usize] == b'-' {
        size += 1;
    }
    size
}

/// Find an option by name (long) or character (short). Returns
/// `(arg_requirement, option_index)` where:
/// - 0 = flag, doesn't take value (either no option found or BOOL)
/// - 1 = optional value
/// - 2 = required value
/// `option_index` is `None` if no matching option was found.
fn get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: i32,
) -> (i32, Option<usize>) {
    let mut found: Option<usize> = None;
    for (i, opt) in options.iter().enumerate() {
        if opt.long_arg.is_none() && opt.short_arg == '\0' {
            // terminator
            break;
        }
        if size == 1 {
            if opt.short_arg != '\0' {
                let first_char = arg.chars().next().unwrap_or('\0');
                if opt.short_arg == first_char {
                    found = Some(i);
                    break;
                }
            }
        } else if let Some(la) = &opt.long_arg {
            if la.len() == len {
                let arg_prefix: String = arg.chars().take(len).collect();
                let arg_prefix_bytes = &arg.as_bytes()[..len.min(arg.len())];
                if la.as_bytes() == arg_prefix_bytes
                    || la.as_str() == arg_prefix.as_str()
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

fn parse_arg(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    argi: &mut i32,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    if ctx.doubledash {
        return true;
    }

    let arg_full = if (*argi as usize) < argv.len() {
        argv[*argi as usize]
    } else {
        ""
    };

    let size = get_size(arg_full);
    let arg_after = &arg_full[(size as usize).min(arg_full.len())..];
    let length = arg_after.len();

    if size == 1 && length == 0 {
        return true; // singular dash - treat as extra
    }

    if size == 2 && length == 0 {
        ctx.doubledash = true;
        return false;
    }

    match size {
        1 => {
            // short
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS
            {
                set_err(
                    err,
                    &format!("short options cannot be combined: {}", arg_full),
                );
            } else if length > 1
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS
            {
                let (arg_req, found) = get_arg(arg_after, 1, &ctx.options, size);
                let opt_idx = match found {
                    Some(i) => i,
                    None => {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            let c = arg_after.chars().next().unwrap_or('\0');
                            set_err(err, &format!("invalid option: -{}", c));
                        }
                        return false;
                    }
                };
                if arg_req == 0 {
                    let c = arg_after.chars().next().unwrap_or('\0');
                    set_err(err, &format!("option doesn't take a value: -{}", c));
                    return false;
                }
                let value_str = &arg_after[1..];
                let opt = ctx.options[opt_idx].clone();
                set_value(data, &opt, Some(value_str), false, err);
            } else {
                // parse all
                let chars: Vec<char> = arg_after.chars().collect();
                let mut i: usize = 0;
                while i < chars.len() {
                    let cur_char = chars[i];
                    let cur_str: String = cur_char.to_string();
                    let (arg_req, found) =
                        get_arg(&cur_str, 1, &ctx.options, size);
                    let opt_idx = match found {
                        Some(idx) => idx,
                        None => {
                            if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                                set_err(err, &format!("invalid option: -{}", cur_char));
                            }
                            break;
                        }
                    };
                    let opt = ctx.options[opt_idx].clone();
                    let remaining_after = chars.len() - i - 1;

                    match arg_req {
                        0 => {
                            // flag - no argument
                            set_value(data, &opt, None, false, err);
                        }
                        1 => {
                            // optional argument
                            if (*argi + 1) < argc
                                && get_size(argv[(*argi + 1) as usize]) == 0
                            {
                                *argi += 1;
                                let val = argv[*argi as usize].to_string();
                                set_value(data, &opt, Some(&val), false, err);
                            } else {
                                set_value(data, &opt, None, false, err);
                            }
                        }
                        2 => {
                            // requires argument
                            if remaining_after == 0 {
                                if (*argi + 1) < argc {
                                    if get_size(argv[(*argi + 1) as usize]) != 0 {
                                        set_err(
                                            err,
                                            &format!(
                                                "missing option value: -{}",
                                                opt.short_arg
                                            ),
                                        );
                                    } else {
                                        *argi += 1;
                                        let val = argv[*argi as usize].to_string();
                                        set_value(data, &opt, Some(&val), false, err);
                                    }
                                } else {
                                    set_err(
                                        err,
                                        &format!(
                                            "missing option value: -{}",
                                            opt.short_arg
                                        ),
                                    );
                                }
                            } else {
                                set_err(
                                    err,
                                    &format!(
                                        "combined short option requiring value is not last: -{}",
                                        opt.short_arg
                                    ),
                                );
                            }
                        }
                        _ => {}
                    }
                    if err.is_some() {
                        break;
                    }
                    i += 1;
                }
            }
            false
        }
        2 => {
            // long
            let (key, val_part): (&str, Option<&str>) = match arg_after.find('=') {
                Some(eq_pos) => {
                    let val = &arg_after[eq_pos + 1..];
                    let val_opt = if val.is_empty() { None } else { Some(val) };
                    (&arg_after[..eq_pos], val_opt)
                }
                None => (arg_after, None),
            };
            let key_len = key.len();
            let (arg_req, found) = get_arg(key, key_len, &ctx.options, size);
            let opt_idx = match found {
                Some(i) => i,
                None => {
                    set_err(err, &format!("invalid option: --{}", key));
                    return false;
                }
            };
            let opt = ctx.options[opt_idx].clone();
            match arg_req {
                0 => {
                    if val_part.is_some() {
                        set_err(err, &format!("option doesn't take a value: --{}", key));
                        return false;
                    }
                    set_value(data, &opt, val_part, true, err);
                }
                2 => {
                    if val_part.is_none() {
                        set_err(err, &format!("missing option value: --{}", key));
                        return false;
                    }
                    set_value(data, &opt, val_part, true, err);
                }
                1 => {
                    set_value(data, &opt, val_part, true, err);
                }
                _ => {}
            }
            false
        }
        _ => {
            // extra
            true
        }
    }
}

fn set_value(
    data: *mut u8,
    option: &XoptOption,
    value: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    if let Some(cb) = option.callback {
        cb(value, data, option, long_arg, err);
    } else {
        default_callback(value, data, option, long_arg, err);
    }
}

fn default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let is_bool = (option.options & XOPT_TYPE_BOOL) != 0;
    let val_empty = value.is_none() || value.map(|s| s.is_empty()).unwrap_or(true);
    if val_empty && !is_bool {
        return;
    }

    if data.is_null() {
        return;
    }

    let type_bits = option.options & 0x3F;
    let value_str = value.unwrap_or("");

    let parse_failed: bool = match type_bits {
        x if x == XOPT_TYPE_BOOL => {
            // SAFETY: writing 1 byte at the offset within the user's struct.
            unsafe {
                let target = data.add(option.offset);
                *target = 1;
            }
            false
        }
        x if x == XOPT_TYPE_STRING => {
            // We can't generically store a `&str`. Instead we copy bytes into
            // a usize-sized slot at the offset. But this would require a
            // pointer interpretation. Given the C code stored the pointer to
            // argv[i], we mimic by storing the pointer to a leaked CString-like
            // memory. To stay safe and compatible, simply do nothing here — the
            // user is expected to provide a custom callback for string types.
            // We do this by leaking a Box<String> and writing the pointer.
            let leaked: &'static str = Box::leak(value_str.to_string().into_boxed_str());
            let ptr = leaked.as_ptr() as usize;
            unsafe {
                let target = data.add(option.offset) as *mut usize;
                target.write_unaligned(ptr);
            }
            false
        }
        x if x == XOPT_TYPE_INT => {
            match parse_c_long(value_str) {
                Ok(v) => {
                    unsafe {
                        let target = data.add(option.offset) as *mut i32;
                        target.write_unaligned(v as i32);
                    }
                    false
                }
                Err(_) => true,
            }
        }
        x if x == XOPT_TYPE_LONG => {
            match parse_c_long(value_str) {
                Ok(v) => {
                    unsafe {
                        let target = data.add(option.offset) as *mut i64;
                        target.write_unaligned(v);
                    }
                    false
                }
                Err(_) => true,
            }
        }
        x if x == XOPT_TYPE_FLOAT => {
            match value_str.trim().parse::<f64>() {
                Ok(v) => {
                    unsafe {
                        let target = data.add(option.offset) as *mut f32;
                        target.write_unaligned(v as f32);
                    }
                    false
                }
                Err(_) => true,
            }
        }
        x if x == XOPT_TYPE_DOUBLE => {
            match value_str.trim().parse::<f64>() {
                Ok(v) => {
                    unsafe {
                        let target = data.add(option.offset) as *mut f64;
                        target.write_unaligned(v);
                    }
                    false
                }
                Err(_) => true,
            }
        }
        _ => {
            eprintln!(
                "warning: XOpt argument type invalid: {}",
                option.options & 0x2F
            );
            false
        }
    };

    if parse_failed {
        if long_arg {
            let la = option.long_arg.as_deref().unwrap_or("");
            set_err(
                err,
                &format!("value isn't a valid number: --{}={}", la, value_str),
            );
        } else {
            set_err(
                err,
                &format!(
                    "value isn't a valid number: -{} {}",
                    option.short_arg, value_str
                ),
            );
        }
    }
}

/// Parses a numeric value approximately like C's `strtol(str, &end, 0)`:
/// supports decimal, hex (0x prefix), and octal (leading 0).
fn parse_c_long(s: &str) -> Result<i64, ()> {
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

    if rest.is_empty() {
        return Err(());
    }

    let parsed: i64 = if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        if hex.is_empty() {
            return Err(());
        }
        i64::from_str_radix(hex, 16).map_err(|_| ())?
    } else if rest.starts_with('0') && rest.len() > 1 {
        // octal
        i64::from_str_radix(&rest[1..], 8).map_err(|_| ())?
    } else {
        rest.parse::<i64>().map_err(|_| ())?
    };

    Ok(parsed.checked_mul(sign).ok_or(())?)
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

    // Compute max width
    let mut width: usize = 0;
    for o in &ctx.options {
        if o.long_arg.is_none() && o.short_arg == '\0' {
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

    for o in &ctx.options {
        if o.long_arg.is_none() && o.short_arg == '\0' {
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
        *($err_ptr) = None;
        let mut __ctx_opt = $crate::xopt::xopt_context(
            $name,
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        );
        if ($err_ptr).is_some() {
            // err: stop
        } else if let Some(ref mut __ctx) = __ctx_opt {
            let __count = $crate::xopt::xopt_parse(
                __ctx,
                $argc,
                $argv,
                $config_ptr as *mut u8,
                $extrav_ptr,
                $err_ptr,
            );
            *($extrac_ptr) = __count;
            if ($err_ptr).is_none() {
                let __opts = $crate::xopt::XoptAutohelpOptions {
                    usage: $autohelp_usage,
                    prefix: $autohelp_prefix,
                    suffix: $autohelp_suffix,
                    spacer: $autohelp_spacer,
                };
                $crate::xopt::xopt_autohelp(__ctx, $autohelp_file, Some(&__opts), $err_ptr);
            }
        }
    }};
}

// Re-export to avoid unused-import warnings.
#[allow(dead_code)]
fn _unused_snprintf_link() {
    let mut s = String::new();
    let _ = snprintf::rpl_vsnprintf(&mut s, 0, "", &[]);
}
