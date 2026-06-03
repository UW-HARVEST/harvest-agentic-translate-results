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

/// Find a terminator option (one whose `long_arg` is `None` and `short_arg` is
/// '\0') and slice off everything from that point on.  This mirrors the C
/// API's expectation that the caller appends `XOPT_NULLOPTION` to terminate
/// the array.
fn truncate_at_null(options: &[XoptOption]) -> Vec<XoptOption> {
    let mut result = Vec::new();
    for o in options {
        if o.long_arg.is_none() && o.short_arg == '\0' {
            break;
        }
        result.push(o.clone());
    }
    result
}

pub fn xopt_context(
    name: Option<&str>,
    options: &[XoptOption],
    flags: i64,
    err: &mut Option<String>,
) -> Option<Box<XoptContext>> {
    *err = None;
    let ctx = XoptContext {
        options: truncate_at_null(options),
        flags,
        name: name.map(|s| s.to_string()),
        doubledash: false,
    };
    Some(Box::new(ctx))
}

/// Compute the dash "size" of an argument: 0 for non-options, 1 for short
/// (`-x`), 2 for long (`--xyz` or `--`).
fn xopt_get_size(arg: &str) -> i32 {
    let mut size = 0i32;
    for (i, c) in arg.chars().enumerate() {
        if i >= 2 {
            break;
        }
        if c != '-' {
            break;
        }
        size += 1;
    }
    size
}

/// Look up an option by name.  Returns the matching option's index plus a
/// "value requirement" code: 0 for flags, 1 for optional values, 2 for
/// required values.
fn xopt_get_arg<'a>(
    arg: &str,
    len: usize,
    options: &'a [XoptOption],
    size: i32,
) -> (Option<&'a XoptOption>, i32) {
    let mut found: Option<&XoptOption> = None;
    let arg_chars: Vec<char> = arg.chars().collect();
    let prefix: String = arg_chars.iter().take(len).collect();
    for o in options {
        if size == 1 && o.short_arg != '\0' {
            if let Some(first) = arg_chars.first() {
                if *first == o.short_arg {
                    found = Some(o);
                    break;
                }
            }
        } else if size == 2 {
            if let Some(la) = &o.long_arg {
                if la.chars().count() == len && la == &prefix {
                    found = Some(o);
                    break;
                }
            }
        }
    }
    let req = match found {
        None => 0,
        Some(o) if (o.options & XOPT_TYPE_BOOL) != 0 => 0,
        Some(o) if (o.options & XOPT_OPTIONAL) != 0 => 1,
        Some(_) => 2,
    };
    (found, req)
}

fn default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    // If no value was specified and this isn't a boolean option, bail.
    let has_value = value.map(|v| !v.is_empty()).unwrap_or(false);
    if !has_value && (option.options & XOPT_TYPE_BOOL) == 0 {
        return;
    }

    // SAFETY: callers of `xopt_parse` are responsible for passing a `data`
    // pointer that points to a struct large enough that `offset` is in-bounds
    // and the value being written has the correct type.  This mirrors the C
    // API contract exactly.
    unsafe {
        let target = data.add(option.offset);
        match option.options & 0x3F {
            x if x == XOPT_TYPE_BOOL => {
                *(target as *mut bool) = true;
            }
            x if x == XOPT_TYPE_STRING => {
                // Store a pointer to the string contents.  We can't store the
                // `&str` directly because we don't know the receiving type, but
                // we can store a pointer to the byte data and length the same
                // way the C code stored a `const char *`.  In our simplified
                // Rust model we store the raw `*const u8` pointer; consumers
                // may interpret it however suits their config struct.
                let v = value.unwrap_or("");
                *(target as *mut *const u8) = v.as_ptr();
            }
            x if x == XOPT_TYPE_INT => {
                let v = value.unwrap_or("0");
                match parse_c_int(v) {
                    Ok(parsed) => *(target as *mut i32) = parsed as i32,
                    Err(_) => set_parse_err(err, option, v, long_arg),
                }
            }
            x if x == XOPT_TYPE_LONG => {
                let v = value.unwrap_or("0");
                match parse_c_int(v) {
                    Ok(parsed) => *(target as *mut i64) = parsed,
                    Err(_) => set_parse_err(err, option, v, long_arg),
                }
            }
            x if x == XOPT_TYPE_FLOAT => {
                let v = value.unwrap_or("0");
                match v.parse::<f64>() {
                    Ok(parsed) => *(target as *mut f32) = parsed as f32,
                    Err(_) => set_parse_err(err, option, v, long_arg),
                }
            }
            x if x == XOPT_TYPE_DOUBLE => {
                let v = value.unwrap_or("0");
                match v.parse::<f64>() {
                    Ok(parsed) => *(target as *mut f64) = parsed,
                    Err(_) => set_parse_err(err, option, v, long_arg),
                }
            }
            _ => {
                // Unknown / mixed type: silently ignore (the C code prints a
                // warning to stderr — we omit that to keep `xopt_parse`
                // side-effect free in the success path).
            }
        }
    }
}

fn parse_c_int(s: &str) -> Result<i64, ()> {
    // Mirrors `strtol(s, ..., 0)`: 0x/0X means hex, leading 0 means octal,
    // otherwise base-10.
    let trimmed = s.trim_start();
    let (negative, rest) = if let Some(stripped) = trimmed.strip_prefix('-') {
        (true, stripped)
    } else if let Some(stripped) = trimmed.strip_prefix('+') {
        (false, stripped)
    } else {
        (false, trimmed)
    };
    let parsed = if let Some(hex) = rest
        .strip_prefix("0x")
        .or_else(|| rest.strip_prefix("0X"))
    {
        i64::from_str_radix(hex, 16).map_err(|_| ())?
    } else if rest.starts_with('0') && rest.len() > 1 {
        i64::from_str_radix(&rest[1..], 8).map_err(|_| ())?
    } else if rest.is_empty() {
        return Err(());
    } else {
        rest.parse::<i64>().map_err(|_| ())?
    };
    Ok(if negative { -parsed } else { parsed })
}

fn set_parse_err(err: &mut Option<String>, option: &XoptOption, value: &str, long_arg: bool) {
    let mut buf = String::new();
    if long_arg {
        let la = option.long_arg.as_deref().unwrap_or("");
        snprintf::rpl_vsnprintf(
            &mut buf,
            usize::MAX,
            "value isn't a valid number: --%s=%s",
            &[la, value],
        );
    } else {
        let mut sa = String::new();
        sa.push(option.short_arg);
        snprintf::rpl_vsnprintf(
            &mut buf,
            usize::MAX,
            "value isn't a valid number: -%c %s",
            &[&sa, value],
        );
    }
    *err = Some(buf);
}

fn dispatch_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let cb = option.callback.unwrap_or(default_callback);
    cb(val, data, option, long_arg, err);
}

/// Outcome from parsing one argument: did it produce an "extra" non-option
/// value, or was it consumed as an option?
fn parse_arg(
    ctx: &mut XoptContext,
    argc: i32,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    let mut is_extra = false;
    let arg_full = argv[*argi];

    if ctx.doubledash {
        return true;
    }

    let size = xopt_get_size(arg_full);
    let arg = &arg_full[size as usize..];
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
            // Short option(s).
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS
            {
                set_err(
                    err,
                    "short options cannot be combined: %s",
                    &[arg_full],
                );
            } else if length > 1
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS
            {
                // Sloppy short: -xVALUE where x is the option and VALUE is
                // appended directly.
                let arg_chars: Vec<char> = arg.chars().collect();
                let single: String = arg_chars.iter().take(1).collect();
                let (option, req) = xopt_get_arg(&single, 1, &ctx.options, size);
                if option.is_none() {
                    if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                        let c0: String = single.clone();
                        set_err(err, "invalid option: -%c", &[&c0]);
                    }
                    return is_extra;
                }
                let option = option.unwrap().clone();
                if req == 0 {
                    let c0: String = single.clone();
                    set_err(err, "option doesn't take a value: -%c", &[&c0]);
                    return is_extra;
                }
                let rest: String = arg_chars.iter().skip(1).collect();
                dispatch_set(data, &option, Some(&rest), false, err);
            } else {
                // Iterate through condensed shorts.
                let arg_chars: Vec<char> = arg.chars().collect();
                let mut idx = 0usize;
                let mut remaining = length;
                while remaining > 0 {
                    let cur: String = arg_chars[idx].to_string();
                    let (option, req) = xopt_get_arg(&cur, 1, &ctx.options, size);
                    idx += 1;
                    remaining -= 1;
                    if option.is_none() {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            set_err(err, "invalid option: -%c", &[&cur]);
                        }
                        break;
                    }
                    let option = option.unwrap().clone();
                    match req {
                        0 => {
                            // Flag.
                            dispatch_set(data, &option, None, false, err);
                        }
                        1 => {
                            // Optional value.
                            if (*argi as i32) + 1 < argc
                                && xopt_get_size(argv[*argi + 1]) == 0
                            {
                                *argi += 1;
                                dispatch_set(data, &option, Some(argv[*argi]), false, err);
                            } else {
                                dispatch_set(data, &option, None, false, err);
                            }
                        }
                        2 => {
                            // Required value.
                            if remaining == 0 {
                                if (*argi as i32) + 1 < argc {
                                    if xopt_get_size(argv[*argi + 1]) != 0 {
                                        let sa = option.short_arg.to_string();
                                        set_err(err, "missing option value: -%c", &[&sa]);
                                    } else {
                                        *argi += 1;
                                        dispatch_set(
                                            data,
                                            &option,
                                            Some(argv[*argi]),
                                            false,
                                            err,
                                        );
                                    }
                                } else {
                                    let sa = option.short_arg.to_string();
                                    set_err(err, "missing option value: -%c", &[&sa]);
                                }
                            } else {
                                let sa = option.short_arg.to_string();
                                set_err(
                                    err,
                                    "combined short option requiring value is not last: -%c",
                                    &[&sa],
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
            // Long option.
            let mut val_start: Option<String> = None;
            let mut name_part: String = arg.to_string();
            if let Some(eq_idx) = arg.find('=') {
                name_part = arg[..eq_idx].to_string();
                let after = &arg[eq_idx + 1..];
                if !after.is_empty() {
                    val_start = Some(after.to_string());
                } else {
                    val_start = None;
                }
            }
            let name_len = name_part.chars().count();
            let (option, req) = xopt_get_arg(&name_part, name_len, &ctx.options, size);
            if option.is_none() {
                set_err(err, "invalid option: --%.*s", &[&name_part]);
            } else {
                let option = option.unwrap().clone();
                match req {
                    0 => {
                        if val_start.is_some() {
                            set_err(err, "option doesn't take a value: --%s", &[&name_part]);
                        } else {
                            dispatch_set(data, &option, val_start.as_deref(), true, err);
                        }
                    }
                    2 => {
                        if val_start.is_none() {
                            set_err(err, "missing option value: --%s", &[&name_part]);
                        }
                    }
                    _ => {}
                }
                if err.is_none() {
                    dispatch_set(data, &option, val_start.as_deref(), true, err);
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

fn set_err(err: &mut Option<String>, fmt: &str, args: &[&str]) {
    let mut buf = String::new();
    snprintf::rpl_vsnprintf(&mut buf, usize::MAX, fmt, args);
    *err = Some(buf);
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
    let mut extras_vec: Vec<String> = Vec::new();

    let mut argi: usize = if (ctx.flags & XOPT_CTX_KEEPFIRST) == 0 {
        1
    } else {
        0
    };

    while (argi as i32) < argc {
        let parse_result = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }

        if parse_result {
            extras_vec.push(argv[argi].to_string());
        } else if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !extras_vec.is_empty() {
            set_err(
                err,
                "options cannot be specified after arguments: %s",
                &[argv[argi]],
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

    // Compute max width.
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

    // Print each option.
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
            while twidth < (width + spacer) {
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

    let _ = stream.flush();
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
        // The C macro performs:
        //   1. Build a context with POSIXMEHARDER | STRICT flags.
        //   2. Run xopt_parse, populating extrav and extrac.
        //   3. If config_ptr->help is set, print autohelp and goto xopt_help.
        //
        // In Rust we mimic the behaviour without `goto` — instead we evaluate
        // to a `bool` indicating whether the caller should jump to its help
        // path (`true` = help requested, `false` = continue normally).  The
        // caller is expected to inspect `*err_ptr` afterwards.
        let mut __xopt_ctx_opt = $crate::xopt::xopt_context(
            Some($name),
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        );
        let __xopt_help_requested: bool;
        if $err_ptr.is_some() {
            __xopt_help_requested = false;
        } else {
            let mut __xopt_extras: Option<Vec<String>> = None;
            let __xopt_count = $crate::xopt::xopt_parse(
                __xopt_ctx_opt.as_mut().unwrap(),
                $argc,
                $argv,
                $config_ptr as *mut u8,
                &mut __xopt_extras,
                $err_ptr,
            );
            *$extrac_ptr = __xopt_count;
            *$extrav_ptr = __xopt_extras;
            if $err_ptr.is_some() {
                __xopt_help_requested = false;
            } else if $config_ptr.help {
                let __xopt_autohelp_opts = $crate::xopt::XoptAutohelpOptions {
                    usage: Some($autohelp_usage.to_string()),
                    prefix: Some($autohelp_prefix.to_string()),
                    suffix: Some($autohelp_suffix.to_string()),
                    spacer: $autohelp_spacer,
                };
                $crate::xopt::xopt_autohelp(
                    __xopt_ctx_opt.as_mut().unwrap(),
                    $autohelp_file,
                    Some(&__xopt_autohelp_opts),
                    $err_ptr,
                );
                __xopt_help_requested = $err_ptr.is_none();
            } else {
                __xopt_help_requested = false;
            }
        }
        __xopt_help_requested
    }};
}
