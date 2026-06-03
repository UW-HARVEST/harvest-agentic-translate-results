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
    Some(Box::new(XoptContext {
        options: options.to_vec(),
        flags,
        name: name.map(|s| s.to_string()),
        doubledash: false,
    }))
}

/// Returns the number of leading dashes (capped at 2).
fn xopt_get_size(arg: &str) -> i32 {
    let bytes = arg.as_bytes();
    let mut size = 0i32;
    while (size as usize) < 2 && (size as usize) < bytes.len() {
        if bytes[size as usize] != b'-' {
            break;
        }
        size += 1;
    }
    size
}

/// Find the option in the option list matching the given name (long if size==2,
/// or short character if size==1).  Returns the argument requirement:
///   0 = flag (bool, no value)
///   1 = optional value
///   2 = required value
/// Returns (requirement, Option<index>) so the caller can fetch the option.
fn xopt_get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: i32,
) -> (i32, Option<usize>) {
    let mut found: Option<usize> = None;
    for (i, opt) in options.iter().enumerate() {
        // Stop at terminator.
        if opt.long_arg.is_none() && opt.short_arg == '\0' {
            break;
        }
        if size == 1 {
            if opt.short_arg != '\0' {
                let arg_chars: Vec<char> = arg.chars().collect();
                if !arg_chars.is_empty() && opt.short_arg == arg_chars[0] {
                    found = Some(i);
                    break;
                }
            }
        } else {
            if let Some(la) = &opt.long_arg {
                let la_bytes = la.as_bytes();
                let arg_bytes = arg.as_bytes();
                if la_bytes.len() == len && len <= arg_bytes.len()
                    && &arg_bytes[..len] == la_bytes
                {
                    found = Some(i);
                    break;
                }
            }
        }
    }
    let requirement = match found {
        None => 0,
        Some(idx) => {
            let opt = &options[idx];
            if (opt.options & XOPT_TYPE_BOOL) != 0 {
                0
            } else if (opt.options & XOPT_OPTIONAL) != 0 {
                1
            } else {
                2
            }
        }
    };
    (requirement, found)
}

/// Default callback - writes the parsed value into the data buffer at the
/// option's offset.  Uses unsafe pointer writes because the C API exposes
/// the data buffer as a raw `*mut u8` and the caller wants direct field
/// population (mirroring `offsetof(struct, field)` semantics).
fn xopt_default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let is_bool = (option.options & XOPT_TYPE_BOOL) != 0;

    // If no value supplied and not a boolean, do nothing (matches C behavior
    // where a missing optional value is silently ignored).
    let value_is_empty = match value {
        None => true,
        Some(v) => v.is_empty(),
    };
    if value_is_empty && !is_bool {
        return;
    }

    let target = unsafe { data.add(option.offset) };
    let type_bits = option.options & 0x3F;

    match type_bits {
        x if x == XOPT_TYPE_BOOL => {
            unsafe { *(target as *mut bool) = true; }
        }
        x if x == XOPT_TYPE_STRING => {
            // The original code stored a pointer; we cannot meaningfully store
            // a `&str` through a raw byte pointer in safe Rust.  Skip — the
            // caller would need a custom callback for string options.
        }
        x if x == XOPT_TYPE_INT => {
            let v = value.unwrap_or("");
            match parse_int_c_style(v) {
                Some(n) => unsafe { *(target as *mut i32) = n as i32; },
                None => set_parse_err(err, option, v, long_arg),
            }
        }
        x if x == XOPT_TYPE_LONG => {
            let v = value.unwrap_or("");
            match parse_int_c_style(v) {
                Some(n) => unsafe { *(target as *mut i64) = n; },
                None => set_parse_err(err, option, v, long_arg),
            }
        }
        x if x == XOPT_TYPE_FLOAT => {
            let v = value.unwrap_or("");
            match v.parse::<f64>() {
                Ok(n) => unsafe { *(target as *mut f32) = n as f32; },
                Err(_) => set_parse_err(err, option, v, long_arg),
            }
        }
        x if x == XOPT_TYPE_DOUBLE => {
            let v = value.unwrap_or("");
            match v.parse::<f64>() {
                Ok(n) => unsafe { *(target as *mut f64) = n; },
                Err(_) => set_parse_err(err, option, v, long_arg),
            }
        }
        _ => {
            eprintln!(
                "warning: XOpt argument type invalid: {}",
                option.options & 0x2F
            );
        }
    }
}

/// Parse an integer the way `strtol(value, &end, 0)` does: detect a leading
/// `0x`/`0X` for hex, leading `0` for octal, otherwise decimal.  Allows an
/// optional leading sign.
fn parse_int_c_style(s: &str) -> Option<i64> {
    let s = s.trim_start();
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
    let (base, digits): (u32, &str) = if let Some(stripped) = rest.strip_prefix("0x") {
        (16, stripped)
    } else if let Some(stripped) = rest.strip_prefix("0X") {
        (16, stripped)
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8, &rest[1..])
    } else {
        (10, rest)
    };
    if digits.is_empty() {
        // bare "0" handled here: original `rest` was "0", base 10
        return if rest == "0" { Some(0) } else { None };
    }
    let n = i64::from_str_radix(digits, base).ok()?;
    Some(sign * n)
}

fn set_parse_err(err: &mut Option<String>, option: &XoptOption, value: &str, long_arg: bool) {
    if long_arg {
        *err = Some(format!(
            "value isn't a valid number: --{}={}",
            option.long_arg.as_deref().unwrap_or(""),
            value
        ));
    } else {
        *err = Some(format!(
            "value isn't a valid number: -{} {}",
            option.short_arg, value
        ));
    }
}

fn xopt_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let cb = option.callback.unwrap_or(xopt_default_callback as XoptCallback);
    cb(val, data, option, long_arg, err);
}

/// Returns true if the argument is an "extra" (non-option), false if it's an
/// option that has been consumed.
fn xopt_parse_arg(
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

    let raw = argv[*argi as usize];
    let size = xopt_get_size(raw);
    let arg = &raw[size as usize..];
    let length = arg.len();

    if size == 1 && length == 0 {
        // Lone dash → extra
        return true;
    }
    if size == 2 && length == 0 {
        // `--` → enable doubledash mode
        ctx.doubledash = true;
        return false;
    }

    match size {
        0 => {
            // Extra positional argument
            return true;
        }
        1 => {
            // Short option(s)
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0 {
                *err = Some(format!("short options cannot be combined: {}", raw));
            } else if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != 0 {
                let (req, opt_idx) = xopt_get_arg(arg, 1, &ctx.options, size);
                match opt_idx {
                    None => {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            let c = arg.chars().next().unwrap_or('?');
                            *err = Some(format!("invalid option: -{}", c));
                        }
                    }
                    Some(idx) => {
                        if req == 0 {
                            let c = arg.chars().next().unwrap_or('?');
                            *err = Some(format!("option doesn't take a value: -{}", c));
                        } else {
                            let opt = ctx.options[idx].clone();
                            // value is everything after the first char
                            let val = &arg[1..];
                            xopt_set(data, &opt, Some(val), false, err);
                        }
                    }
                }
            } else {
                // Parse each character as a separate short option
                let arg_chars: Vec<char> = arg.chars().collect();
                let total = arg_chars.len();
                for i in 0..total {
                    let remaining = total - i - 1;
                    let ch = arg_chars[i];
                    let cur = &arg[i..];
                    let (req, opt_idx) = xopt_get_arg(cur, 1, &ctx.options, size);
                    match opt_idx {
                        None => {
                            if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                                *err = Some(format!("invalid option: -{}", ch));
                            }
                            break;
                        }
                        Some(idx) => {
                            let opt = ctx.options[idx].clone();
                            match req {
                                0 => {
                                    xopt_set(data, &opt, None, false, err);
                                }
                                1 => {
                                    if (*argi + 1) < argc
                                        && xopt_get_size(argv[(*argi + 1) as usize]) == 0
                                    {
                                        *argi += 1;
                                        let nv = argv[*argi as usize];
                                        xopt_set(data, &opt, Some(nv), false, err);
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
                                                let nv = argv[*argi as usize];
                                                xopt_set(data, &opt, Some(nv), false, err);
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
            // Long option
            let (name_part, val_part): (&str, Option<&str>) = match arg.find('=') {
                Some(eq_pos) => {
                    let name = &arg[..eq_pos];
                    let val = &arg[eq_pos + 1..];
                    let val_opt = if val.is_empty() { None } else { Some(val) };
                    (name, val_opt)
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
                    let mut local_err: Option<String> = None;
                    match req {
                        0 => {
                            if val_part.is_some() {
                                local_err =
                                    Some(format!("option doesn't take a value: --{}", arg));
                            }
                        }
                        2 => {
                            if val_part.is_none() {
                                local_err =
                                    Some(format!("missing option value: --{}", arg));
                            }
                        }
                        _ => {}
                    }

                    if local_err.is_some() {
                        *err = local_err;
                    } else {
                        xopt_set(data, &opt, val_part, true, err);
                    }
                }
            }
        }
        _ => {}
    }

    false
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
        let parse_result = xopt_parse_arg(ctx, argc, argv, &mut argi, data, err);
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

    // Compute the maximum width of all option labels.
    let mut width: usize = 0;
    for o in ctx.options.iter() {
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

    // Print each option line.
    for o in ctx.options.iter() {
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
            while twidth < (width + spacer) {
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
        // Minimal Rust translation of the C XOPT_SIMPLE_PARSE macro.
        // It builds a context, parses the args, and runs autohelp on `--help`.
        let mut __xopt_err: Option<String> = None;
        let mut __xopt_ctx_opt = $crate::xopt::xopt_context(
            $name,
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            &mut __xopt_err,
        );
        if __xopt_err.is_none() {
            if let Some(ref mut __xopt_ctx) = __xopt_ctx_opt {
                let mut __xopt_extras: Option<Vec<String>> = None;
                let __xopt_count = $crate::xopt::xopt_parse(
                    __xopt_ctx,
                    $argc,
                    $argv,
                    $config_ptr as *mut u8,
                    &mut __xopt_extras,
                    &mut __xopt_err,
                );
                *$extrac_ptr = __xopt_count;
                *$extrav_ptr = __xopt_extras;
            }
        }
        *$err_ptr = __xopt_err;
        let _ = $autohelp_file;
        let _ = $autohelp_usage;
        let _ = $autohelp_prefix;
        let _ = $autohelp_suffix;
        let _ = $autohelp_spacer;
    }};
}
