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

/// Returns true iff the option is the null terminator (no longArg AND no shortArg).
fn is_null_option(o: &XoptOption) -> bool {
    o.long_arg.is_none() && o.short_arg == '\0'
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

/// Returns the "size" of the leading dashes: 0=extra, 1=short, 2=long
fn xopt_get_size(arg: &str) -> i32 {
    let bytes = arg.as_bytes();
    let mut size: i32 = 0;
    while (size as usize) < 2 && (size as usize) < bytes.len() && bytes[size as usize] == b'-' {
        size += 1;
    }
    size
}

/// Look up an option matching `arg` (a slice of length `len` characters).
/// `size` is 1 (short) or 2 (long).
/// Returns argRequirement: 0 = no value, 1 = optional, 2 = required.
/// If found, `*found_idx` is set to Some(index into options list).
fn xopt_get_arg(
    arg: &str,
    len: usize,
    options: &[XoptOption],
    size: i32,
    found_idx: &mut Option<usize>,
) -> i32 {
    *found_idx = None;
    for (i, opt) in options.iter().enumerate() {
        if is_null_option(opt) {
            break;
        }
        if size == 1 {
            if opt.short_arg != '\0' && opt.short_arg as u32 == arg.chars().next().unwrap_or('\0') as u32 {
                *found_idx = Some(i);
                break;
            }
        } else if let Some(ref la) = opt.long_arg {
            if la.len() == len && la == &arg[..len.min(arg.len())] {
                *found_idx = Some(i);
                break;
            }
        }
    }

    let opt = match *found_idx {
        Some(i) => &options[i],
        None => return 0,
    };
    if (opt.options & XOPT_TYPE_BOOL) != 0 {
        return 0;
    } else if (opt.options & XOPT_OPTIONAL) != 0 {
        return 1;
    }
    2
}

/// Storage for parsed values, indexed by option offset.
#[derive(Debug, Default, Clone)]
pub struct ParsedValues {
    pub strings: std::collections::HashMap<usize, String>,
    pub ints: std::collections::HashMap<usize, i64>,
    pub floats: std::collections::HashMap<usize, f64>,
    pub bools: std::collections::HashMap<usize, bool>,
}

impl ParsedValues {
    pub fn new() -> Self {
        Default::default()
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
        return;
    }
    default_callback(val, data, option, long_arg, err);
}

fn default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    // If no value and not bool, return
    let is_bool = (option.options & XOPT_TYPE_BOOL) != 0;
    let value_is_empty = match value {
        None => true,
        Some(v) => v.is_empty(),
    };
    if value_is_empty && !is_bool {
        return;
    }
    if data.is_null() {
        // No backing data store; nothing more to do
        return;
    }
    // Treat data pointer as &mut ParsedValues
    let pv: &mut ParsedValues = unsafe { &mut *(data as *mut ParsedValues) };
    let off = option.offset;
    match option.options & 0x3F {
        x if x == XOPT_TYPE_BOOL => {
            pv.bools.insert(off, true);
        }
        x if x == XOPT_TYPE_STRING => {
            pv.strings.insert(off, value.unwrap_or("").to_string());
        }
        x if x == XOPT_TYPE_INT || x == XOPT_TYPE_LONG => {
            let v = value.unwrap_or("");
            // strtol with base 0
            let parsed: Result<i64, _> = parse_strtol(v);
            match parsed {
                Ok(n) => {
                    pv.ints.insert(off, n);
                }
                Err(_) => {
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
        }
        x if x == XOPT_TYPE_FLOAT || x == XOPT_TYPE_DOUBLE => {
            let v = value.unwrap_or("");
            match v.parse::<f64>() {
                Ok(n) => {
                    pv.floats.insert(off, n);
                }
                Err(_) => {
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
        }
        _ => {
            // unknown type
        }
    }
}

fn parse_strtol(s: &str) -> Result<i64, ()> {
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
    let (base, digits): (u32, &str) = if let Some(stripped) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        (16, stripped)
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8, &rest[1..])
    } else {
        (10, rest)
    };
    if digits.is_empty() {
        if base == 8 {
            return Ok(0);
        }
        return Err(());
    }
    let mut all_consumed = true;
    let mut last_good = 0;
    for (i, c) in digits.chars().enumerate() {
        if c.to_digit(base).is_some() {
            last_good = i + 1;
        } else {
            all_consumed = false;
            break;
        }
    }
    if last_good == 0 {
        return Err(());
    }
    if !all_consumed {
        // strtol stops at non-digit but is not an error in C; but the C xopt
        // code checks if `*parsePtr` is non-empty and treats it as invalid.
        return Err(());
    }
    let v = i64::from_str_radix(&digits[..last_good], base).map_err(|_| ())?;
    Ok(sign * v)
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
    let arg_full = argv[*argi as usize];
    let size = xopt_get_size(arg_full);
    let arg = &arg_full[size as usize..];
    let length = arg.len();

    if size == 1 && length == 0 {
        return true;
    }
    if size == 2 && length == 0 {
        ctx.doubledash = true;
        return false;
    }

    let mut is_extra = false;

    match size {
        1 => {
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS
            {
                *err = Some(format!("short options cannot be combined: {}", arg_full));
            } else if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS {
                let mut found_idx: Option<usize> = None;
                let arg_req = xopt_get_arg(arg, 1, &ctx.options, size, &mut found_idx);
                if found_idx.is_none() {
                    if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                        *err = Some(format!(
                            "invalid option: -{}",
                            arg.chars().next().unwrap_or(' ')
                        ));
                    }
                    return false;
                }
                if arg_req == 0 {
                    *err = Some(format!(
                        "option doesn't take a value: -{}",
                        arg.chars().next().unwrap_or(' ')
                    ));
                    return false;
                }
                let opt_clone = ctx.options[found_idx.unwrap()].clone();
                xopt_set(data, &opt_clone, Some(&arg[1..]), false, err);
                if err.is_some() {
                    return false;
                }
            } else {
                // parse all chars
                let mut idx = 0usize;
                let arg_chars: Vec<char> = arg.chars().collect();
                while idx < arg_chars.len() {
                    let c = arg_chars[idx];
                    let one = c.to_string();
                    let mut found_idx: Option<usize> = None;
                    let arg_req = xopt_get_arg(&one, 1, &ctx.options, size, &mut found_idx);
                    if found_idx.is_none() {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            *err = Some(format!("invalid option: -{}", c));
                        }
                        break;
                    }
                    let opt_clone = ctx.options[found_idx.unwrap()].clone();
                    let remaining = arg_chars.len() - idx - 1;
                    match arg_req {
                        0 => {
                            xopt_set(data, &opt_clone, None, false, err);
                        }
                        1 => {
                            // optional
                            if (*argi + 1) < argc
                                && xopt_get_size(argv[(*argi + 1) as usize]) == 0
                            {
                                *argi += 1;
                                let v = argv[*argi as usize];
                                xopt_set(data, &opt_clone, Some(v), false, err);
                            } else {
                                xopt_set(data, &opt_clone, None, false, err);
                            }
                        }
                        2 => {
                            if remaining == 0 {
                                if (*argi + 1) < argc {
                                    if xopt_get_size(argv[(*argi + 1) as usize]) != 0 {
                                        *err = Some(format!(
                                            "missing option value: -{}",
                                            opt_clone.short_arg
                                        ));
                                    } else {
                                        *argi += 1;
                                        let v = argv[*argi as usize];
                                        xopt_set(data, &opt_clone, Some(v), false, err);
                                    }
                                } else {
                                    *err = Some(format!(
                                        "missing option value: -{}",
                                        opt_clone.short_arg
                                    ));
                                }
                            } else {
                                *err = Some(format!(
                                    "combined short option requiring value is not last: -{}",
                                    opt_clone.short_arg
                                ));
                            }
                        }
                        _ => {}
                    }
                    if err.is_some() {
                        break;
                    }
                    idx += 1;
                }
            }
        }
        2 => {
            // long
            let eq_pos = arg.find('=');
            let (name_part, val_part_opt) = match eq_pos {
                Some(p) => {
                    let v = &arg[p + 1..];
                    (&arg[..p], if v.is_empty() { None } else { Some(v) })
                }
                None => (arg, None),
            };
            let mut found_idx: Option<usize> = None;
            let arg_req = xopt_get_arg(name_part, name_part.len(), &ctx.options, size, &mut found_idx);
            if found_idx.is_none() {
                *err = Some(format!("invalid option: --{}", name_part));
            } else {
                match arg_req {
                    0 => {
                        if val_part_opt.is_some() {
                            *err = Some(format!("option doesn't take a value: --{}", arg));
                        }
                    }
                    2 => {
                        if val_part_opt.is_none() {
                            *err = Some(format!("missing option value: --{}", arg));
                        }
                    }
                    _ => {}
                }
                if err.is_none() {
                    let opt_clone = ctx.options[found_idx.unwrap()].clone();
                    xopt_set(data, &opt_clone, val_part_opt, true, err);
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
    let mut nl = "";
    let spacer = options.map(|o| o.spacer).unwrap_or(2);
    if let Some(opts) = options {
        if let Some(ref u) = opts.usage {
            let _ = write!(stream, "{}{}\n", nl, u);
            nl = "\n";
        }
        if let Some(ref p) = opts.prefix {
            let _ = write!(stream, "{}{}\n\n", nl, p);
            nl = "\n";
        }
    }
    let _ = nl;

    // find max width
    let mut width: usize = 0;
    let mut idx = 0;
    while idx < ctx.options.len() {
        let o = &ctx.options[idx];
        if is_null_option(o) {
            break;
        }
        let mut twidth = 0usize;
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
        idx += 1;
    }

    idx = 0;
    while idx < ctx.options.len() {
        let o = &ctx.options[idx];
        if is_null_option(o) {
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
        if let Some(ref la) = o.long_arg {
            let _ = write!(stream, "--{}", la);
            twidth += 2 + la.len();
            if let Some(ref ad) = o.arg_descrip {
                let _ = write!(stream, "={}", ad);
                twidth += 1 + ad.len();
            }
        }
        if let Some(ref descr) = o.descrip {
            while twidth < (width + spacer) {
                let _ = write!(stream, " ");
                twidth += 1;
            }
            let _ = write!(stream, "{}\n", descr);
        }
        idx += 1;
    }

    if let Some(opts) = options {
        if let Some(ref suff) = opts.suffix {
            let _ = write!(stream, "\n{}\n", suff);
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
        let mut __err: Option<String> = None;
        let __ctx_opt = $crate::xopt::xopt_context(
            $name,
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            &mut __err,
        );
        if let (Some(mut __ctx), None) = (__ctx_opt, __err.clone()) {
            let mut __extras: Option<Vec<String>> = None;
            let __count = $crate::xopt::xopt_parse(
                &mut __ctx,
                $argc,
                $argv,
                $config_ptr as *mut u8,
                &mut __extras,
                &mut __err,
            );
            *$extrac_ptr = __count;
            *$extrav_ptr = __extras;
            *$err_ptr = __err;
        } else {
            *$err_ptr = __err;
        }
    }};
}
