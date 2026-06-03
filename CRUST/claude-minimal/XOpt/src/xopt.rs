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

// Helper: returns true if option is a "terminator" entry (no longArg and no shortArg).
fn is_terminator(opt: &XoptOption) -> bool {
    opt.long_arg.is_none() && opt.short_arg == '\0'
}

// Determines the "size" of an arg's leading dashes (0, 1, or 2).
fn _xopt_get_size(arg: &str) -> usize {
    let bytes = arg.as_bytes();
    let mut size = 0;
    while size < 2 && size < bytes.len() && bytes[size] == b'-' {
        size += 1;
    }
    size
}

// Tries to find the option matching `arg` (after dashes have been stripped).
// `size` is 1 (short) or 2 (long).
// Returns a (found_index, arg_requirement) tuple.
//
// arg_requirement values:
//   0 -> doesn't take a value (flag/bool or not found)
//   1 -> optional value
//   2 -> requires a value
fn _xopt_get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: usize,
) -> (Option<usize>, i32) {
    let mut found: Option<usize> = None;
    let arg_bytes = arg.as_bytes();

    for (i, opt) in options.iter().enumerate() {
        // Stop iteration once we reach the terminator entry.
        if is_terminator(opt) {
            break;
        }

        if size == 1 {
            // Match a short option by its first byte.
            if !arg_bytes.is_empty() && opt.short_arg != '\0' {
                let mut buf = [0u8; 4];
                let s = opt.short_arg.encode_utf8(&mut buf);
                if s.as_bytes().len() == 1 && s.as_bytes()[0] == arg_bytes[0] {
                    found = Some(i);
                    break;
                }
            }
        } else {
            // Long option: compare exactly `len` bytes.
            if let Some(ref la) = opt.long_arg {
                let la_bytes = la.as_bytes();
                if la_bytes.len() == len && len <= arg_bytes.len()
                    && la_bytes == &arg_bytes[..len]
                {
                    found = Some(i);
                    break;
                }
            }
        }
    }

    let req = match found {
        None => 0,
        Some(idx) => {
            let o = &options[idx];
            if o.options & XOPT_TYPE_BOOL != 0 {
                0
            } else if o.options & XOPT_OPTIONAL != 0 {
                1
            } else {
                2
            }
        }
    };

    (found, req)
}

// Parses an integer string with C-style `strtol(..., 0)` semantics:
// - leading "0x"/"0X" => hex
// - leading "0"       => octal
// - otherwise         => decimal
// Allows optional leading +/-.
fn parse_int_c(s: &str) -> Option<i64> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    let (sign, rest) = match s.as_bytes()[0] {
        b'-' => (-1i64, &s[1..]),
        b'+' => (1i64, &s[1..]),
        _ => (1i64, s),
    };
    if rest.is_empty() {
        return None;
    }
    let (base, digits) = if rest.starts_with("0x") || rest.starts_with("0X") {
        (16u32, &rest[2..])
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8u32, &rest[1..])
    } else {
        (10u32, rest)
    };
    if digits.is_empty() {
        return None;
    }
    let mut val: i64 = 0;
    for c in digits.chars() {
        let d = c.to_digit(base)?;
        val = val.checked_mul(base as i64)?.checked_add(d as i64)?;
    }
    Some(sign * val)
}

fn parse_float_c(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

// The default callback that applies parsed values to the data struct.
fn _xopt_default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    // C: `if ((!value || !strlen(value)) && !(option->options & XOPT_TYPE_BOOL)) return;`
    let value_present = match value {
        Some(v) => !v.is_empty(),
        None => false,
    };
    if !value_present && (option.options & XOPT_TYPE_BOOL == 0) {
        return;
    }

    if data.is_null() {
        return;
    }

    // SAFETY: data points to a struct of the appropriate layout, and `option.offset`
    // is a valid byte offset into that struct.
    let target = unsafe { data.add(option.offset) };

    let mut parse_failed = false;
    match option.options & 0x3F {
        x if x == XOPT_TYPE_BOOL => {
            unsafe {
                *(target as *mut bool) = true;
            }
        }
        x if x == XOPT_TYPE_STRING => {
            // Best-effort: write a *const u8 pointing at the value's bytes.
            // Lifetime of the value should outlive the parse, mirroring how argv
            // strings outlive xopt_parse calls in C.
            if let Some(v) = value {
                unsafe {
                    *(target as *mut *const u8) = v.as_ptr();
                }
            }
        }
        x if x == XOPT_TYPE_INT => {
            match value.and_then(parse_int_c) {
                Some(n) => unsafe {
                    *(target as *mut i32) = n as i32;
                },
                None => parse_failed = true,
            }
        }
        x if x == XOPT_TYPE_LONG => {
            match value.and_then(parse_int_c) {
                Some(n) => unsafe {
                    *(target as *mut i64) = n;
                },
                None => parse_failed = true,
            }
        }
        x if x == XOPT_TYPE_FLOAT => {
            match value.and_then(parse_float_c) {
                Some(f) => unsafe {
                    *(target as *mut f32) = f as f32;
                },
                None => parse_failed = true,
            }
        }
        x if x == XOPT_TYPE_DOUBLE => {
            match value.and_then(parse_float_c) {
                Some(f) => unsafe {
                    *(target as *mut f64) = f;
                },
                None => parse_failed = true,
            }
        }
        _ => {
            eprintln!(
                "warning: XOpt argument type invalid: {}",
                option.options & 0x2F
            );
        }
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

fn _xopt_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let cb = option.callback.unwrap_or(_xopt_default_callback);
    cb(val, data, option, long_arg, err);
}

// Parses a single argument from argv at *argi. Returns true if the argument
// was an "extra" (non-option), false if it was an option.
fn _xopt_parse_arg(
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
    let size = _xopt_get_size(arg_full);
    let arg = &arg_full[size..];
    let length_initial = arg.len();

    // Just a single dash -> extra.
    if size == 1 && length_initial == 0 {
        return true;
    }

    // `--` => everything after this is an extra.
    if size == 2 && length_initial == 0 {
        ctx.doubledash = true;
        return false;
    }

    match size {
        1 => {
            // Short option(s).
            if length_initial > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS
            {
                *err = Some(format!("short options cannot be combined: {}", arg_full));
                return false;
            }
            if length_initial > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS {
                let (found, arg_req) = _xopt_get_arg(arg, 1, &ctx.options, size);
                let found_idx = match found {
                    Some(i) => i,
                    None => {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            *err = Some(format!(
                                "invalid option: -{}",
                                arg.chars().next().unwrap_or(' ')
                            ));
                        }
                        return false;
                    }
                };
                if arg_req == 0 {
                    *err = Some(format!(
                        "option doesn't take a value: -{}",
                        arg.chars().next().unwrap_or(' ')
                    ));
                    return false;
                }
                let opt = ctx.options[found_idx].clone();
                _xopt_set(data, &opt, Some(&arg[1..]), false, err);
                return false;
            }

            // Parse condensed short options one character at a time.
            let arg_bytes = arg.as_bytes();
            let mut idx = 0usize;
            let mut length = length_initial;
            while length > 0 {
                let cur_byte = arg_bytes[idx];
                let cur_str = std::str::from_utf8(&arg_bytes[idx..idx + 1]).unwrap_or("");
                idx += 1;
                length -= 1;

                let (found, arg_req) = _xopt_get_arg(cur_str, 1, &ctx.options, size);
                let found_idx = match found {
                    Some(i) => i,
                    None => {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            *err = Some(format!("invalid option: -{}", cur_byte as char));
                        }
                        break;
                    }
                };

                let opt = ctx.options[found_idx].clone();

                match arg_req {
                    0 => {
                        _xopt_set(data, &opt, None, false, err);
                    }
                    1 => {
                        // optional argument: take next argv if it's not an option.
                        if (*argi + 1) < argc
                            && _xopt_get_size(argv[(*argi + 1) as usize]) == 0
                        {
                            *argi += 1;
                            let next = argv[*argi as usize];
                            _xopt_set(data, &opt, Some(next), false, err);
                        } else {
                            _xopt_set(data, &opt, None, false, err);
                        }
                    }
                    2 => {
                        // requires a value
                        if length == 0 {
                            // last in cluster
                            if (*argi + 1) < argc {
                                let next = argv[(*argi + 1) as usize];
                                if _xopt_get_size(next) != 0 {
                                    *err = Some(format!(
                                        "missing option value: -{}",
                                        opt.short_arg
                                    ));
                                } else {
                                    *argi += 1;
                                    _xopt_set(data, &opt, Some(next), false, err);
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
            false
        }
        2 => {
            // Long option.
            // Look for '='.
            let (name_part, val_part_opt): (&str, Option<&str>) = match arg.find('=') {
                Some(eq_pos) => {
                    let name = &arg[..eq_pos];
                    let after = &arg[eq_pos + 1..];
                    let val = if after.is_empty() { None } else { Some(after) };
                    (name, val)
                }
                None => (arg, None),
            };

            let length = name_part.len();
            let (found, arg_req) = _xopt_get_arg(name_part, length, &ctx.options, size);
            match found {
                None => {
                    *err = Some(format!("invalid option: --{}", name_part));
                }
                Some(idx) => {
                    let opt = ctx.options[idx].clone();
                    match arg_req {
                        0 => {
                            // flag - shouldn't have a value
                            if val_part_opt.is_some() {
                                *err = Some(format!(
                                    "option doesn't take a value: --{}",
                                    name_part
                                ));
                            }
                            // C calls _xopt_set unconditionally here, but only if no err
                            if err.is_none() {
                                _xopt_set(data, &opt, val_part_opt, true, err);
                            }
                        }
                        2 => {
                            if val_part_opt.is_none() {
                                *err = Some(format!("missing option value: --{}", name_part));
                            }
                            if err.is_none() {
                                _xopt_set(data, &opt, val_part_opt, true, err);
                            }
                        }
                        1 => {
                            if err.is_none() {
                                _xopt_set(data, &opt, val_part_opt, true, err);
                            }
                        }
                        _ => {}
                    }
                }
            }
            false
        }
        _ => {
            // size == 0 -> extra
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

    // Skip argv[0] unless KEEPFIRST is set.
    if (ctx.flags & XOPT_CTX_KEEPFIRST) == 0 {
        argi += 1;
    }

    while argi < argc {
        if (argi as usize) >= argv.len() {
            break;
        }
        let parse_result = _xopt_parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }

        if parse_result {
            // It's an extra.
            extras_vec.push(argv[argi as usize].to_string());
        } else {
            // It's an option. Enforce POSIXMEHARDER if set.
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
        if let Some(ref usage) = opts.usage {
            let _ = writeln!(stream, "{}{}", nl, usage);
            nl = "\n";
        }
        if let Some(ref prefix) = opts.prefix {
            let _ = writeln!(stream, "{}{}\n", nl, prefix);
            nl = "\n";
        }
    }

    // Find max width.
    let mut width: usize = 0;
    for o in ctx.options.iter() {
        if is_terminator(o) {
            break;
        }
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
        if twidth > width {
            width = twidth;
        }
    }

    // Print options.
    for o in ctx.options.iter() {
        if is_terminator(o) {
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
        if let Some(ref la) = o.long_arg {
            let _ = write!(stream, "--{}", la);
            twidth += 2 + la.len();
            if let Some(ref ad) = o.arg_descrip {
                let _ = write!(stream, "={}", ad);
                twidth += 1 + ad.len();
            }
        }
        if let Some(ref descrip) = o.descrip {
            while twidth < (width + spacer) {
                let _ = write!(stream, " ");
                twidth += 1;
            }
            let _ = writeln!(stream, "{}", descrip);
        }
    }

    if let Some(opts) = options {
        if let Some(ref suffix) = opts.suffix {
            let _ = writeln!(stream, "{}{}", nl, suffix);
        }
    }

    // Touch the snprintf module so the import isn't unused (parity with the C
    // codebase, which links snprintf for error formatting).
    let _ = snprintf::getnumsep;
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
        if $err_ptr.is_none() {
            if let Some(ref mut __ctx_box) = __xopt_ctx {
                let __count = $crate::xopt::xopt_parse(
                    __ctx_box,
                    $argc,
                    $argv,
                    $config_ptr as *mut u8,
                    $extrav_ptr,
                    $err_ptr,
                );
                *$extrac_ptr = __count;
                if $err_ptr.is_none() && (*$config_ptr).help {
                    let __opts = $crate::xopt::XoptAutohelpOptions {
                        usage: $autohelp_usage,
                        prefix: $autohelp_prefix,
                        suffix: $autohelp_suffix,
                        spacer: $autohelp_spacer,
                    };
                    $crate::xopt::xopt_autohelp(
                        __ctx_box,
                        $autohelp_file,
                        Some(&__opts),
                        $err_ptr,
                    );
                }
            }
        }
    }};
}
