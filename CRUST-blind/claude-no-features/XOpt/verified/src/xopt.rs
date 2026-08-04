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

/// Helper: parse a number string with strtol-like semantics (auto-detect base
/// from prefix `0x`/`0X` for hex, `0` for octal, otherwise decimal). Returns
/// the parsed value and a flag indicating whether any unparsed trailing
/// non-digit characters were present (i.e. parse failed to consume the whole
/// string).
fn strtol_auto(s: &str) -> (i64, bool) {
    let bytes = s.as_bytes();
    let mut idx = 0usize;

    // Skip leading whitespace (matches strtol behaviour).
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_whitespace() {
        idx += 1;
    }

    let mut neg = false;
    if idx < bytes.len() {
        match bytes[idx] {
            b'-' => {
                neg = true;
                idx += 1;
            }
            b'+' => {
                idx += 1;
            }
            _ => {}
        }
    }

    // Detect base.
    let base: u32;
    if idx + 1 < bytes.len()
        && bytes[idx] == b'0'
        && (bytes[idx + 1] == b'x' || bytes[idx + 1] == b'X')
    {
        base = 16;
        idx += 2;
    } else if idx < bytes.len() && bytes[idx] == b'0' {
        base = 8;
        // do not skip the 0 - if it's just "0", the loop will still parse it
    } else {
        base = 10;
    }

    let start = idx;
    let mut val: i64 = 0;
    while idx < bytes.len() {
        let c = bytes[idx];
        let d: i64 = match c {
            b'0'..=b'9' => (c - b'0') as i64,
            b'a'..=b'z' => (c - b'a') as i64 + 10,
            b'A'..=b'Z' => (c - b'A') as i64 + 10,
            _ => break,
        };
        if d >= base as i64 {
            break;
        }
        val = val.saturating_mul(base as i64).saturating_add(d);
        idx += 1;
    }

    let parse_failed = if start == idx {
        // No digits at all -> failure (parse pointer at start)
        true
    } else {
        // Failure only if any non-digit remains
        idx < bytes.len()
    };

    if neg {
        val = -val;
    }

    (val, parse_failed)
}

/// Helper: parse a floating point number. Returns (value, parse_failed).
fn strtod_auto(s: &str) -> (f64, bool) {
    let trimmed = s.trim_start();
    // Try to parse the longest valid prefix.
    let bytes = trimmed.as_bytes();
    let mut end = 0usize;
    // Optional sign
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let mut saw_digit = false;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
        saw_digit = true;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
            saw_digit = true;
        }
    }
    if saw_digit && end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        end += 1;
        if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
            end += 1;
        }
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }

    if !saw_digit {
        return (0.0, true);
    }

    let head = &trimmed[..end];
    match head.parse::<f64>() {
        Ok(v) => (v, end < bytes.len()),
        Err(_) => (0.0, true),
    }
}

/// Helper: count leading dashes in an arg, capped at 2.
fn xopt_get_size(arg: &str) -> i32 {
    let bytes = arg.as_bytes();
    let mut size = 0i32;
    while (size as usize) < bytes.len() && size < 2 {
        if bytes[size as usize] != b'-' {
            break;
        }
        size += 1;
    }
    size
}

/// Helper: find a matching option for the given argument text.
/// Returns (arg_requirement, matched_option_index) where:
/// - arg_requirement: 0 = no argument (flag), 1 = optional, 2 = required
/// - matched_option_index: None if no option found
fn xopt_get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: i32,
) -> (i32, Option<usize>) {
    let bytes = arg.as_bytes();
    let mut found: Option<usize> = None;

    for (i, opt) in options.iter().enumerate() {
        // Stop at terminator (no long_arg and short_arg == '\0')
        if opt.long_arg.is_none() && opt.short_arg == '\0' {
            break;
        }

        if size == 1 {
            // Short option: match first character.
            if !bytes.is_empty() && opt.short_arg != '\0' && opt.short_arg as u32 == bytes[0] as u32 {
                found = Some(i);
                break;
            }
        } else {
            // Long option: full string match limited to len bytes.
            if let Some(la) = &opt.long_arg {
                if la.len() == len && la.as_bytes() == &bytes[..len.min(bytes.len())] {
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

/// Default callback: writes the parsed value into the data buffer at the
/// option's offset. Mirrors `_xopt_default_callback` in xopt.c.
fn xopt_default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let is_bool = (option.options & XOPT_TYPE_BOOL) != 0;
    let value_missing = value.map(|v| v.is_empty()).unwrap_or(true);

    if value_missing && !is_bool {
        return;
    }

    if data.is_null() {
        return;
    }

    let target = unsafe { data.add(option.offset) };
    let ty = option.options & 0x3F;
    let mut parse_failed = false;

    if ty == XOPT_TYPE_BOOL {
        // booleans are a single byte (Rust's bool is 1 byte)
        unsafe {
            *target = 1u8;
        }
    } else if ty == XOPT_TYPE_STRING {
        // Storing a Rust &str through a raw pointer is not generally safe
        // because of lifetimes; the C version stores `const char *`. We do
        // not write anything for strings - the consumer is expected to use
        // a custom callback for string storage when needed.
    } else if ty == XOPT_TYPE_INT {
        if let Some(v) = value {
            let (parsed, failed) = strtol_auto(v);
            parse_failed = failed;
            let val = parsed as i32;
            unsafe {
                let p = target as *mut i32;
                p.write_unaligned(val);
            }
        }
    } else if ty == XOPT_TYPE_LONG {
        if let Some(v) = value {
            let (parsed, failed) = strtol_auto(v);
            parse_failed = failed;
            unsafe {
                let p = target as *mut i64;
                p.write_unaligned(parsed);
            }
        }
    } else if ty == XOPT_TYPE_FLOAT {
        if let Some(v) = value {
            let (parsed, failed) = strtod_auto(v);
            parse_failed = failed;
            unsafe {
                let p = target as *mut f32;
                p.write_unaligned(parsed as f32);
            }
        }
    } else if ty == XOPT_TYPE_DOUBLE {
        if let Some(v) = value {
            let (parsed, failed) = strtod_auto(v);
            parse_failed = failed;
            unsafe {
                let p = target as *mut f64;
                p.write_unaligned(parsed);
            }
        }
    } else {
        // Unknown / multiple types specified.
        eprintln!(
            "warning: XOpt argument type invalid: {}",
            option.options & 0x2F
        );
    }

    if parse_failed {
        let v = value.unwrap_or("");
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

/// Dispatches the resolved option to either the user-provided callback or the
/// default callback.
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

/// Parses a single argument. Returns true if it's an "extra" (non-option) and
/// false if it's an option. `argi` may be advanced when an argument's value
/// is taken from the next argv entry.
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

    let arg_full = argv[*argi as usize];
    let size = xopt_get_size(arg_full);
    let arg = &arg_full[size as usize..];
    let length = arg.len();

    if size == 1 && length == 0 {
        // Single dash -> treat as extra
        return true;
    }
    if size == 2 && length == 0 {
        // Double dash -> from now on, everything is extra
        ctx.doubledash = true;
        return false;
    }

    let mut is_extra = false;

    match size {
        1 => {
            // Short option(s)
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0 {
                *err = Some(format!(
                    "short options cannot be combined: {}",
                    arg_full
                ));
            } else if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != 0 {
                let (arg_req, opt_idx) =
                    xopt_get_arg(arg, 1, &ctx.options, size);
                match opt_idx {
                    None => {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            *err = Some(format!(
                                "invalid option: -{}",
                                arg.chars().next().unwrap_or('\0')
                            ));
                        }
                    }
                    Some(i) => {
                        if arg_req == 0 {
                            *err = Some(format!(
                                "option doesn't take a value: -{}",
                                arg.chars().next().unwrap_or('\0')
                            ));
                        } else {
                            let val = &arg[1..];
                            let opt = ctx.options[i].clone();
                            xopt_set(data, &opt, Some(val), false, err);
                        }
                    }
                }
            } else {
                // Parse all condensed short options
                let arg_bytes = arg.as_bytes();
                let mut k = 0usize;
                let mut remaining = length;
                while remaining > 0 {
                    let cur = std::str::from_utf8(&arg_bytes[k..k + 1])
                        .unwrap_or("");
                    let (arg_req, opt_idx) =
                        xopt_get_arg(cur, 1, &ctx.options, size);
                    k += 1;
                    remaining -= 1;
                    match opt_idx {
                        None => {
                            if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                                *err = Some(format!(
                                    "invalid option: -{}",
                                    arg_bytes[k - 1] as char
                                ));
                            }
                            break;
                        }
                        Some(i) => {
                            let opt = ctx.options[i].clone();
                            match arg_req {
                                0 => {
                                    xopt_set(data, &opt, None, false, err);
                                }
                                1 => {
                                    // optional argument
                                    if *argi + 1 < argc
                                        && xopt_get_size(
                                            argv[(*argi + 1) as usize],
                                        ) == 0
                                    {
                                        *argi += 1;
                                        xopt_set(
                                            data,
                                            &opt,
                                            Some(argv[*argi as usize]),
                                            false,
                                            err,
                                        );
                                    } else {
                                        xopt_set(data, &opt, None, false, err);
                                    }
                                }
                                2 => {
                                    if remaining == 0 {
                                        if *argi + 1 < argc {
                                            if xopt_get_size(
                                                argv[(*argi + 1) as usize],
                                            ) != 0
                                            {
                                                *err = Some(format!(
                                                    "missing option value: -{}",
                                                    opt.short_arg
                                                ));
                                            } else {
                                                *argi += 1;
                                                xopt_set(
                                                    data,
                                                    &opt,
                                                    Some(argv[*argi as usize]),
                                                    false,
                                                    err,
                                                );
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
                        }
                    }
                    if err.is_some() {
                        break;
                    }
                }
            }
        }
        2 => {
            // Long option
            let mut name = arg;
            let mut val_start: Option<&str> = None;
            if let Some(eq_pos) = arg.find('=') {
                name = &arg[..eq_pos];
                let v = &arg[eq_pos + 1..];
                if v.is_empty() {
                    val_start = None;
                } else {
                    val_start = Some(v);
                }
            }

            let (arg_req, opt_idx) =
                xopt_get_arg(name, name.len(), &ctx.options, size);
            match opt_idx {
                None => {
                    *err = Some(format!("invalid option: --{}", name));
                }
                Some(i) => {
                    let opt = ctx.options[i].clone();
                    match arg_req {
                        0 => {
                            if val_start.is_some() {
                                *err = Some(format!(
                                    "option doesn't take a value: --{}",
                                    name
                                ));
                            } else {
                                xopt_set(data, &opt, val_start, true, err);
                            }
                        }
                        2 => {
                            if val_start.is_none() {
                                *err = Some(format!(
                                    "missing option value: --{}",
                                    name
                                ));
                            } else if err.is_none() {
                                xopt_set(data, &opt, val_start, true, err);
                            }
                        }
                        _ => {
                            if err.is_none() {
                                xopt_set(data, &opt, val_start, true, err);
                            }
                        }
                    }
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

    // Skip argv[0] unless KEEPFIRST is set.
    if (ctx.flags & XOPT_CTX_KEEPFIRST) == 0 {
        argi += 1;
    }

    while argi < argc && (argi as usize) < argv.len() {
        let parse_result =
            xopt_parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }

        if parse_result {
            extras_vec.push(argv[argi as usize].to_string());
        } else if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0
            && !extras_vec.is_empty()
        {
            *err = Some(format!(
                "options cannot be specified after arguments: {}",
                argv[argi as usize]
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

    // Find max width
    let mut width: usize = 0;
    for opt in &ctx.options {
        if opt.long_arg.is_none() && opt.short_arg == '\0' {
            break;
        }
        let mut twidth: usize = 0;
        if let Some(la) = &opt.long_arg {
            twidth += 2 + la.len();
            if let Some(ad) = &opt.arg_descrip {
                twidth += 1 + ad.len();
            }
        }
        if opt.short_arg != '\0' {
            twidth += 2;
        }
        if opt.short_arg != '\0' && opt.long_arg.is_some() {
            twidth += 2;
        }
        if twidth > width {
            width = twidth;
        }
    }

    // Print options
    for opt in &ctx.options {
        if opt.long_arg.is_none() && opt.short_arg == '\0' {
            break;
        }
        let mut twidth: usize = 0;
        if opt.short_arg != '\0' {
            let _ = write!(stream, "-{}", opt.short_arg);
            twidth += 2;
        }
        if opt.short_arg != '\0' && opt.long_arg.is_some() {
            let _ = write!(stream, ", ");
            twidth += 2;
        }
        if let Some(la) = &opt.long_arg {
            let _ = write!(stream, "--{}", la);
            twidth += 2 + la.len();
            if let Some(ad) = &opt.arg_descrip {
                let _ = write!(stream, "={}", ad);
                twidth += 1 + ad.len();
            }
        }
        if let Some(d) = &opt.descrip {
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

    // Suppress unused-import warning when format helpers aren't referenced.
    let _ = snprintf::getexponent as fn(f64) -> i32;
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
        let __ctx_opt = $crate::xopt::xopt_context(
            Some($name),
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        );
        if let Some(mut __ctx) = __ctx_opt {
            if $err_ptr.is_none() {
                let __count = $crate::xopt::xopt_parse(
                    &mut *__ctx,
                    $argc,
                    $argv,
                    $config_ptr as *mut u8,
                    $extrav_ptr,
                    $err_ptr,
                );
                *$extrac_ptr = __count;
                if $err_ptr.is_none() {
                    let __opts = $crate::xopt::XoptAutohelpOptions {
                        usage: $autohelp_usage,
                        prefix: $autohelp_prefix,
                        suffix: $autohelp_suffix,
                        spacer: $autohelp_spacer,
                    };
                    $crate::xopt::xopt_autohelp(
                        &mut *__ctx,
                        $autohelp_file,
                        Some(&__opts),
                        $err_ptr,
                    );
                }
            }
        }
    }};
}
