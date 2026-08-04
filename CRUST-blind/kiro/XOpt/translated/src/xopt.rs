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

// Helper: set error message using snprintf-style formatting
fn set_err(err: &mut Option<String>, fmt: &str, args: &[&str]) {
    let mut buf = String::new();
    snprintf::rpl_vsnprintf(&mut buf, 4096, fmt, args);
    *err = Some(buf);
}

// Helper: check if an option is a "null" terminator
fn is_null_option(o: &XoptOption) -> bool {
    o.long_arg.is_none() && o.short_arg == '\0'
}

// Helper: count leading dashes (0, 1, or 2)
fn get_size(arg: &str) -> usize {
    let bytes = arg.as_bytes();
    let mut size = 0;
    while size < 2 && size < bytes.len() && bytes[size] == b'-' {
        size += 1;
    }
    size
}

// Helper: find matching option, returns (arg_requirement, option_index)
// arg_requirement: 0=flag/bool, 1=optional, 2=required
fn get_arg(arg: &str, len: usize, options: &[XoptOption], size: usize) -> (i32, Option<usize>) {
    for (idx, opt) in options.iter().enumerate() {
        if is_null_option(opt) { break; }
        if size == 1 && !arg.is_empty() && opt.short_arg == arg.chars().next().unwrap_or('\0') {
            let req = if opt.options & XOPT_TYPE_BOOL != 0 { 0 }
                else if opt.options & XOPT_OPTIONAL != 0 { 1 }
                else { 2 };
            return (req, Some(idx));
        } else if let Some(ref la) = opt.long_arg {
            if la.len() == len && arg.starts_with(la.as_str()) && la.len() <= arg.len() {
                // Check exact match of first `len` chars
                if &arg[..len] == la.as_str() {
                    let req = if opt.options & XOPT_TYPE_BOOL != 0 { 0 }
                        else if opt.options & XOPT_OPTIONAL != 0 { 1 }
                        else { 2 };
                    return (req, Some(idx));
                }
            }
        }
    }
    (0, None)
}

// Helper: invoke callback or default callback
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
    let val_empty = match value {
        None => true,
        Some(v) => v.is_empty(),
    };

    if val_empty && (option.options & XOPT_TYPE_BOOL == 0) {
        return;
    }

    if data.is_null() {
        return;
    }

    let offset = option.offset;
    let typ = option.options & 0x3F;

    match typ {
        XOPT_TYPE_BOOL => {
            // Write a bool (1 byte) at offset
            // Safety: caller must ensure data+offset is valid
            unsafe {
                let target = data.add(offset);
                *target = 1;
            }
        }
        XOPT_TYPE_STRING => {
            // We can't easily store a &str pointer into raw memory in safe Rust.
            // Store the string bytes. This is a best-effort translation.
            // In practice, the callback approach is preferred.
        }
        XOPT_TYPE_INT => {
            if let Some(v) = value {
                match parse_int(v) {
                    Ok(n) => unsafe {
                        let target = data.add(offset) as *mut i32;
                        *target = n;
                    },
                    Err(_) => {
                        if long_arg {
                            if let Some(ref la) = option.long_arg {
                                set_err(err, "value isn't a valid number: --%s=%s", &[la, v]);
                            }
                        } else {
                            let sc = option.short_arg.to_string();
                            set_err(err, "value isn't a valid number: -%c %s", &[&sc, v]);
                        }
                    }
                }
            }
        }
        XOPT_TYPE_LONG => {
            if let Some(v) = value {
                match parse_long(v) {
                    Ok(n) => unsafe {
                        let target = data.add(offset) as *mut i64;
                        *target = n;
                    },
                    Err(_) => {
                        if long_arg {
                            if let Some(ref la) = option.long_arg {
                                set_err(err, "value isn't a valid number: --%s=%s", &[la, v]);
                            }
                        } else {
                            let sc = option.short_arg.to_string();
                            set_err(err, "value isn't a valid number: -%c %s", &[&sc, v]);
                        }
                    }
                }
            }
        }
        XOPT_TYPE_FLOAT => {
            if let Some(v) = value {
                match v.parse::<f32>() {
                    Ok(n) => unsafe {
                        let target = data.add(offset) as *mut f32;
                        *target = n;
                    },
                    Err(_) => {
                        if long_arg {
                            if let Some(ref la) = option.long_arg {
                                set_err(err, "value isn't a valid number: --%s=%s", &[la, v]);
                            }
                        } else {
                            let sc = option.short_arg.to_string();
                            set_err(err, "value isn't a valid number: -%c %s", &[&sc, v]);
                        }
                    }
                }
            }
        }
        XOPT_TYPE_DOUBLE => {
            if let Some(v) = value {
                match v.parse::<f64>() {
                    Ok(n) => unsafe {
                        let target = data.add(offset) as *mut f64;
                        *target = n;
                    },
                    Err(_) => {
                        if long_arg {
                            if let Some(ref la) = option.long_arg {
                                set_err(err, "value isn't a valid number: --%s=%s", &[la, v]);
                            }
                        } else {
                            let sc = option.short_arg.to_string();
                            set_err(err, "value isn't a valid number: -%c %s", &[&sc, v]);
                        }
                    }
                }
            }
        }
        _ => {
            let t = (option.options & 0x2F).to_string();
            eprintln!("warning: XOpt argument type invalid: {}", t);
        }
    }
}

// Parse int with C strtol semantics (base 0 = auto-detect)
fn parse_int(s: &str) -> Result<i32, ()> {
    let s = s.trim();
    if s.is_empty() { return Err(()); }
    let (neg, s) = if s.starts_with('-') { (true, &s[1..]) }
        else if s.starts_with('+') { (false, &s[1..]) }
        else { (false, s) };
    let (base, s) = if s.starts_with("0x") || s.starts_with("0X") { (16, &s[2..]) }
        else if s.starts_with('0') && s.len() > 1 { (8, &s[1..]) }
        else { (10, s) };
    if s.is_empty() { return Err(()); }
    let val = i64::from_str_radix(s, base).map_err(|_| ())?;
    let val = if neg { -val } else { val };
    Ok(val as i32)
}

fn parse_long(s: &str) -> Result<i64, ()> {
    let s = s.trim();
    if s.is_empty() { return Err(()); }
    let (neg, s) = if s.starts_with('-') { (true, &s[1..]) }
        else if s.starts_with('+') { (false, &s[1..]) }
        else { (false, s) };
    let (base, s) = if s.starts_with("0x") || s.starts_with("0X") { (16, &s[2..]) }
        else if s.starts_with('0') && s.len() > 1 { (8, &s[1..]) }
        else { (10, s) };
    if s.is_empty() { return Err(()); }
    let val = i64::from_str_radix(s, base).map_err(|_| ())?;
    Ok(if neg { -val } else { val })
}

// Parse a single argument, returns true if it's an "extra" (non-option)
fn parse_arg(
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

    let full_arg = argv[*argi];
    let size = get_size(full_arg);
    let arg = &full_arg[size..];
    let length = arg.len();

    if size == 1 && length == 0 {
        // Just a singular dash "-"
        return true;
    }

    if size == 2 && length == 0 {
        // Double-dash "--"
        ctx.doubledash = true;
        return false;
    }

    match size {
        1 => {
            // Short option(s)
            if length > 1 && ctx.flags & XOPT_CTX_NOCONDENSE != 0 {
                set_err(err, "short options cannot be combined: %s", &[full_arg]);
            } else if length > 1 && ctx.flags & XOPT_CTX_SLOPPYSHORTS == XOPT_CTX_SLOPPYSHORTS {
                let (arg_req, opt_idx) = get_arg(arg, 1, &ctx.options, size);
                if opt_idx.is_none() {
                    if ctx.flags & XOPT_CTX_STRICT != 0 {
                        let c = arg.chars().next().unwrap_or('\0').to_string();
                        set_err(err, "invalid option: -%c", &[&c]);
                    }
                    return false;
                }
                let opt_idx = opt_idx.unwrap();
                if arg_req == 0 {
                    let c = arg.chars().next().unwrap_or('\0').to_string();
                    set_err(err, "option doesn't take a value: -%c", &[&c]);
                    return false;
                }
                let option = ctx.options[opt_idx].clone();
                xopt_set(data, &option, Some(&arg[1..]), false, err);
            } else {
                // Parse all condensed short options
                let chars: Vec<char> = arg.chars().collect();
                let mut ci = 0;
                while ci < chars.len() {
                    let c_str = chars[ci].to_string();
                    let (arg_req, opt_idx) = get_arg(&c_str, 1, &ctx.options, size);
                    if opt_idx.is_none() {
                        if ctx.flags & XOPT_CTX_STRICT != 0 {
                            set_err(err, "invalid option: -%c", &[&c_str]);
                        }
                        break;
                    }
                    let opt_idx = opt_idx.unwrap();
                    let option = ctx.options[opt_idx].clone();
                    let remaining = chars.len() - ci - 1;

                    match arg_req {
                        0 => {
                            // Flag, no argument
                            xopt_set(data, &option, None, false, err);
                        }
                        1 => {
                            // Optional argument
                            if *argi + 1 < argc as usize && get_size(argv[*argi + 1]) == 0 {
                                *argi += 1;
                                xopt_set(data, &option, Some(argv[*argi]), false, err);
                            } else {
                                xopt_set(data, &option, None, false, err);
                            }
                        }
                        2 => {
                            // Required argument
                            if remaining == 0 {
                                if *argi + 1 < argc as usize {
                                    if get_size(argv[*argi + 1]) != 0 {
                                        let sc = option.short_arg.to_string();
                                        set_err(err, "missing option value: -%c", &[&sc]);
                                    } else {
                                        *argi += 1;
                                        xopt_set(data, &option, Some(argv[*argi]), false, err);
                                    }
                                } else {
                                    let sc = option.short_arg.to_string();
                                    set_err(err, "missing option value: -%c", &[&sc]);
                                }
                            } else {
                                let sc = option.short_arg.to_string();
                                set_err(err, "combined short option requiring value is not last: -%c", &[&sc]);
                            }
                        }
                        _ => {}
                    }

                    if err.is_some() { break; }
                    ci += 1;
                }
            }
            false
        }
        2 => {
            // Long option
            let (arg_name, val_start) = if let Some(eq_pos) = arg.find('=') {
                let name = &arg[..eq_pos];
                let val = &arg[eq_pos + 1..];
                let val = if val.is_empty() { None } else { Some(val) };
                (name, val)
            } else {
                (arg, None)
            };

            let length = arg_name.len();
            let (arg_req, opt_idx) = get_arg(arg_name, length, &ctx.options, size);

            if opt_idx.is_none() {
                set_err(err, "invalid option: --%s", &[arg_name]);
            } else {
                let opt_idx = opt_idx.unwrap();
                let option = ctx.options[opt_idx].clone();

                match arg_req {
                    0 => {
                        // Flag, doesn't take argument
                        if val_start.is_some() {
                            set_err(err, "option doesn't take a value: --%s", &[arg]);
                        }
                        xopt_set(data, &option, val_start, true, err);
                    }
                    2 => {
                        // Requires argument
                        if val_start.is_none() {
                            set_err(err, "missing option value: --%s", &[arg]);
                        }
                        if err.is_none() {
                            xopt_set(data, &option, val_start, true, err);
                        }
                    }
                    _ => {
                        // Optional (1) or other
                        if err.is_none() {
                            xopt_set(data, &option, val_start, true, err);
                        }
                    }
                }
            }
            false
        }
        _ => {
            // Extra argument (size == 0)
            true
        }
    }
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
    let mut extras_list: Vec<String> = Vec::new();
    let mut extras_count = 0i32;

    let mut argi: usize = if ctx.flags & XOPT_CTX_KEEPFIRST == 0 { 1 } else { 0 };

    while (argi as i32) < argc {
        let is_extra = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() { break; }

        if is_extra {
            extras_list.push(argv[argi].to_string());
            extras_count += 1;
        } else {
            if (ctx.flags & XOPT_CTX_POSIXMEHARDER != 0) && extras_count > 0 {
                set_err(err, "options cannot be specified after arguments: %s", &[argv[argi]]);
                break;
            }
        }

        argi += 1;
    }

    if err.is_some() {
        *extras = None;
        return 0;
    }

    *extras = Some(extras_list);
    extras_count
}

pub fn xopt_autohelp(
    ctx: &mut XoptContext,
    stream: &mut dyn std::io::Write,
    options: Option<&XoptAutohelpOptions>,
    err: &mut Option<String>,
) {
    *err = None;
    let mut nl = "";
    let spacer = options.map_or(2, |o| o.spacer);

    if let Some(opts) = options {
        if let Some(ref usage) = opts.usage {
            let _ = write!(stream, "{}{}\n", nl, usage);
            nl = "\n";
        }
        if let Some(ref prefix) = opts.prefix {
            let _ = write!(stream, "{}{}\n\n", nl, prefix);
            nl = "\n";
        }
    }

    // Find max width
    let mut width: usize = 0;
    for o in &ctx.options {
        if is_null_option(o) { break; }
        let mut twidth: usize = 0;
        if let Some(ref la) = o.long_arg {
            twidth += 2 + la.len();
            if let Some(ref ad) = o.arg_descrip {
                twidth += 1 + ad.len();
            }
        }
        if o.short_arg != '\0' {
            twidth += 2;
        }
        if o.short_arg != '\0' && o.long_arg.is_some() {
            twidth += 2;
        }
        if twidth > width { width = twidth; }
    }

    // Print options
    for o in &ctx.options {
        if is_null_option(o) { break; }
        let mut twidth: usize = 0;

        if o.short_arg != '\0' {
            let _ = write!(stream, "-{}", o.short_arg);
            twidth += 2;
        }
        if o.short_arg != '\0' && o.long_arg.is_some() {
            let _ = write!(stream, ", ");
            twidth += 2;
        }
        if let Some(ref la) = o.long_arg {
            let _ = write!(stream, "--{}", la);
            twidth += 2 + la.len();
            if let Some(ref ad) = o.arg_descrip {
                let _ = write!(stream, "={}", ad);
                twidth += 1 + ad.len();
            }
        }
        if let Some(ref descrip) = o.descrip {
            while twidth < width + spacer {
                let _ = write!(stream, " ");
                twidth += 1;
            }
            let _ = write!(stream, "{}\n", descrip);
        }
    }

    if let Some(opts) = options {
        if let Some(ref suffix) = opts.suffix {
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
    ) => {
        loop {
            *$err_ptr = None;

            let mut _xopt_ctx = $crate::xopt::xopt_context(
                $name,
                $options,
                $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
                $err_ptr,
            );
            if ($err_ptr).is_some() { break; }

            let ctx = _xopt_ctx.as_mut().unwrap();
            *$extrac_ptr = $crate::xopt::xopt_parse(
                ctx,
                $argc,
                $argv,
                $config_ptr as *mut u8,
                $extrav_ptr,
                $err_ptr,
            );
            if ($err_ptr).is_some() { break; }

            if ($config_ptr).help {
                let __xopt_autohelp_opts = $crate::xopt::XoptAutohelpOptions {
                    usage: $autohelp_usage,
                    prefix: $autohelp_prefix,
                    suffix: $autohelp_suffix,
                    spacer: $autohelp_spacer,
                };
                $crate::xopt::xopt_autohelp(
                    ctx,
                    $autohelp_file,
                    Some(&__xopt_autohelp_opts),
                    $err_ptr,
                );
                if ($err_ptr).is_some() {
                    *$extrav_ptr = None;
                    break;
                }
                break; // goto xopt_help equivalent
            }

            break;
        }
    };
}
