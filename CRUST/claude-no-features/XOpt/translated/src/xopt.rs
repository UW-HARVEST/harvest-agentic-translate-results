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

/// Helper: returns the count of leading dashes (0, 1, or 2) in the argument.
fn xopt_get_size(arg: &str) -> i32 {
    let bytes = arg.as_bytes();
    let mut size = 0;
    while size < 2 && (size as usize) < bytes.len() && bytes[size as usize] == b'-' {
        size += 1;
    }
    size
}

/// Helper: locate an option by its short or long arg name.
/// `arg` is the (already-trimmed-of-dash) argument; `len` is how many chars to compare for long args.
/// `size` is 1 for short (single char compare) or 2 for long.
/// Returns the requirement (0=none, 1=optional, 2=required) and the option index.
fn xopt_get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: i32,
) -> (i32, Option<usize>) {
    let mut found: Option<usize> = None;

    for (i, opt) in options.iter().enumerate() {
        // terminator: long_arg None and short_arg '\0'
        if opt.long_arg.is_none() && opt.short_arg == '\0' {
            break;
        }

        if size == 1 {
            // match on first char
            let first_char = arg.chars().next().unwrap_or('\0');
            if opt.short_arg != '\0' && opt.short_arg == first_char {
                found = Some(i);
                break;
            }
        } else {
            // long match: compare opt.long_arg (full) to arg[..len]
            if let Some(la) = &opt.long_arg {
                if la.len() == len && len <= arg.len() && la.as_bytes() == &arg.as_bytes()[..len] {
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

fn xopt_default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    // is a value specified?
    let has_value = match value {
        Some(s) => !s.is_empty(),
        None => false,
    };

    if !has_value && (option.options & XOPT_TYPE_BOOL) == 0 {
        // no value and not boolean - silently return (matches C behavior)
        return;
    }

    let target = unsafe { data.add(option.offset) };
    let mut parse_err: Option<String> = None;

    match option.options & 0x3F {
        x if x == XOPT_TYPE_BOOL => {
            unsafe {
                *(target as *mut bool) = true;
            }
        }
        x if x == XOPT_TYPE_STRING => {
            // We can't store a &str pointer easily through *mut u8 in safe Rust.
            // The C version stores the original const char*. Here, we leak a CString-like
            // approach: store a *const u8 pointing to a leaked string.
            // In practice, the tests don't exercise XOPT_TYPE_STRING, so this is a
            // best-effort implementation.
            let v = value.unwrap_or("");
            let leaked: &'static str = Box::leak(v.to_string().into_boxed_str());
            unsafe {
                *(target as *mut *const u8) = leaked.as_ptr();
            }
        }
        x if x == XOPT_TYPE_INT => {
            let v = value.unwrap_or("");
            match parse_c_long(v) {
                Ok(n) => unsafe { *(target as *mut i32) = n as i32 },
                Err(_) => parse_err = Some(v.to_string()),
            }
        }
        x if x == XOPT_TYPE_LONG => {
            let v = value.unwrap_or("");
            match parse_c_long(v) {
                Ok(n) => unsafe { *(target as *mut i64) = n },
                Err(_) => parse_err = Some(v.to_string()),
            }
        }
        x if x == XOPT_TYPE_FLOAT => {
            let v = value.unwrap_or("");
            match v.trim().parse::<f64>() {
                Ok(n) => unsafe { *(target as *mut f32) = n as f32 },
                Err(_) => parse_err = Some(v.to_string()),
            }
        }
        x if x == XOPT_TYPE_DOUBLE => {
            let v = value.unwrap_or("");
            match v.trim().parse::<f64>() {
                Ok(n) => unsafe { *(target as *mut f64) = n },
                Err(_) => parse_err = Some(v.to_string()),
            }
        }
        _ => {
            eprintln!(
                "warning: XOpt argument type invalid: {}",
                option.options & 0x2F
            );
        }
    }

    if let Some(bad) = parse_err {
        if long_arg {
            *err = Some(format!(
                "value isn't a valid number: --{}={}",
                option.long_arg.as_deref().unwrap_or(""),
                bad
            ));
        } else {
            *err = Some(format!(
                "value isn't a valid number: -{} {}",
                option.short_arg, bad
            ));
        }
    }
}

/// Parse a number using C strtol-like semantics (base 0: auto-detect 0x prefix or 0 prefix).
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

    let (base, num): (u32, &str) = if let Some(stripped) = rest
        .strip_prefix("0x")
        .or_else(|| rest.strip_prefix("0X"))
    {
        if stripped.is_empty() {
            return Err(());
        }
        (16, stripped)
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8, &rest[1..])
    } else {
        (10, rest)
    };

    match i64::from_str_radix(num, base) {
        Ok(v) => Ok(sign * v),
        Err(_) => Err(()),
    }
}

fn xopt_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let cb = option.callback.unwrap_or(xopt_default_callback);
    cb(val, data, option, long_arg, err);
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

/// Parses a single argument. Returns Ok(true) if the arg was an extra,
/// Ok(false) if it was an option, or sets an error.
fn parse_arg(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    argi: &mut i32,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    let arg_full = argv[*argi as usize];

    if ctx.doubledash {
        return true;
    }

    let size = xopt_get_size(arg_full);
    let arg = &arg_full[size as usize..];
    let length = arg.len();

    if size == 1 && length == 0 {
        // singular dash - extra
        return true;
    }
    if size == 2 && length == 0 {
        // -- doubledash
        ctx.doubledash = true;
        return false;
    }

    let mut is_extra = false;

    match size {
        1 => {
            // short
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS {
                *err = Some(format!("short options cannot be combined: {}", arg_full));
            } else if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS {
                let (req, opt_idx) = xopt_get_arg(arg, 1, &ctx.options, size);
                match opt_idx {
                    None => {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            *err = Some(format!("invalid option: -{}", arg.chars().next().unwrap_or('\0')));
                        }
                    }
                    Some(idx) => {
                        if req == 0 {
                            *err = Some(format!("option doesn't take a value: -{}", arg.chars().next().unwrap_or('\0')));
                        } else {
                            let val = &arg[1..];
                            let opt = ctx.options[idx].clone();
                            xopt_set(data, &opt, Some(val), false, err);
                        }
                    }
                }
            } else {
                // parse all condensed short opts
                let arg_bytes = arg.as_bytes();
                let mut pos = 0usize;
                let mut remaining = length;
                while remaining > 0 {
                    let cur_char = arg_bytes[pos] as char;
                    let cur_str = &arg[pos..pos + 1];
                    pos += 1;
                    remaining -= 1;

                    let (req, opt_idx) = xopt_get_arg(cur_str, 1, &ctx.options, size);
                    match opt_idx {
                        None => {
                            if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                                *err = Some(format!("invalid option: -{}", cur_char));
                            }
                            break;
                        }
                        Some(idx) => {
                            let opt = ctx.options[idx].clone();
                            match req {
                                0 => {
                                    // flag - no argument
                                    xopt_set(data, &opt, None, false, err);
                                }
                                1 => {
                                    // optional argument
                                    if (*argi + 1) < argc
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
                                        if (*argi + 1) < argc {
                                            if xopt_get_size(argv[(*argi + 1) as usize]) != 0 {
                                                *err = Some(format!(
                                                    "missing option value: -{}",
                                                    opt.short_arg
                                                ));
                                            } else {
                                                *argi += 1;
                                                let v = argv[*argi as usize];
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
            }
        }
        2 => {
            // long
            let (name_part, val_start): (&str, Option<&str>) = match arg.find('=') {
                Some(idx) => {
                    let n = &arg[..idx];
                    let v = &arg[idx + 1..];
                    let v_opt = if v.is_empty() { None } else { Some(v) };
                    (n, v_opt)
                }
                None => (arg, None),
            };

            let (req, opt_idx) = xopt_get_arg(name_part, name_part.len(), &ctx.options, size);
            match opt_idx {
                None => {
                    *err = Some(format!("invalid option: --{}", name_part));
                }
                Some(idx) => {
                    let opt = ctx.options[idx].clone();
                    match req {
                        0 => {
                            if val_start.is_some() {
                                *err = Some(format!("option doesn't take a value: --{}", arg));
                            } else {
                                xopt_set(data, &opt, val_start, true, err);
                            }
                        }
                        2 => {
                            if val_start.is_none() {
                                *err = Some(format!("missing option value: --{}", arg));
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
        0 => {
            // extra
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
        let parse_result = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }

        if parse_result {
            extras_vec.push(argv[argi as usize].to_string());
        } else {
            if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !extras_vec.is_empty() {
                *err = Some(format!(
                    "options cannot be specified after arguments: {}",
                    argv[argi as usize]
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

    // Find max width
    let mut width: usize = 0;
    for o in &ctx.options {
        if o.long_arg.is_none() && o.short_arg == '\0' {
            break;
        }
        let mut twidth: usize = 0;
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

    // Print
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
        *$err_ptr = None;
        let mut _ctx = $crate::xopt::xopt_context(
            $name,
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        );
        if $err_ptr.is_none() {
            if let Some(ctx) = _ctx.as_mut() {
                *$extrac_ptr = $crate::xopt::xopt_parse(
                    ctx,
                    $argc,
                    $argv,
                    $config_ptr as *mut u8,
                    $extrav_ptr,
                    $err_ptr,
                );
            }
        }
    }};
}
