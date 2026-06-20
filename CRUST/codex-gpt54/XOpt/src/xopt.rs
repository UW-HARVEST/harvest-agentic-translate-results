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
    let _ = &snprintf::rpl_vsnprintf;
    *err = None;
    Some(Box::new(XoptContext {
        options: options.to_vec(),
        flags,
        name: name.map(str::to_string),
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

    let argc = argc.max(0) as usize;
    let limit = argc.min(argv.len());
    let mut argi = if ctx.flags & XOPT_CTX_KEEPFIRST == 0 { 1 } else { 0 };
    let mut extra_args = Vec::new();

    while argi < limit {
        match parse_arg(ctx, argc, argv, &mut argi, data, err) {
            ParseResult::Extra => extra_args.push(argv[argi].to_string()),
            ParseResult::Option => {
                if ctx.flags & XOPT_CTX_POSIXMEHARDER != 0 && !extra_args.is_empty() {
                    *err = Some(format!(
                        "options cannot be specified after arguments: {}",
                        argv[argi]
                    ));
                    break;
                }
            }
        }

        if err.is_some() {
            break;
        }

        argi += 1;
    }

    if err.is_some() {
        *extras = None;
        0
    } else {
        let count = extra_args.len() as i32;
        *extras = Some(extra_args);
        count
    }
}

pub fn xopt_autohelp(
    ctx: &mut XoptContext,
    stream: &mut dyn std::io::Write,
    options: Option<&XoptAutohelpOptions>,
    err: &mut Option<String>,
) {
    *err = None;

    let spacer = options.map(|o| o.spacer).unwrap_or(2);
    let active_options: Vec<&XoptOption> = ctx
        .options
        .iter()
        .take_while(|o| o.long_arg.is_some() || o.short_arg != '\0')
        .collect();

    let mut width = 0usize;
    for option in &active_options {
        let mut twidth = 0usize;
        if let Some(long_arg) = &option.long_arg {
            twidth += 2 + long_arg.len();
            if let Some(arg_descrip) = &option.arg_descrip {
                twidth += 1 + arg_descrip.len();
            }
        }
        if option.short_arg != '\0' {
            twidth += 2;
        }
        if option.short_arg != '\0' && option.long_arg.is_some() {
            twidth += 2;
        }
        width = width.max(twidth);
    }

    let mut nl = "";
    if let Some(opts) = options {
        if let Some(usage) = &opts.usage {
            if write!(stream, "{}{}\n", nl, usage).is_err() {
                *err = Some("failed writing autohelp output".to_string());
                return;
            }
            nl = "\n";
        }

        if let Some(prefix) = &opts.prefix {
            if write!(stream, "{}{}\n\n", nl, prefix).is_err() {
                *err = Some("failed writing autohelp output".to_string());
                return;
            }
            nl = "\n";
        }
    }

    for option in active_options {
        let mut twidth = 0usize;
        if option.short_arg != '\0' {
            if write!(stream, "-{}", option.short_arg).is_err() {
                *err = Some("failed writing autohelp output".to_string());
                return;
            }
            twidth += 2;
        }

        if option.short_arg != '\0' && option.long_arg.is_some() {
            if write!(stream, ", ").is_err() {
                *err = Some("failed writing autohelp output".to_string());
                return;
            }
            twidth += 2;
        }

        if let Some(long_arg) = &option.long_arg {
            if write!(stream, "--{}", long_arg).is_err() {
                *err = Some("failed writing autohelp output".to_string());
                return;
            }
            twidth += 2 + long_arg.len();
            if let Some(arg_descrip) = &option.arg_descrip {
                if write!(stream, "={}", arg_descrip).is_err() {
                    *err = Some("failed writing autohelp output".to_string());
                    return;
                }
                twidth += 1 + arg_descrip.len();
            }
        }

        if let Some(descrip) = &option.descrip {
            for _ in twidth..(width + spacer) {
                if write!(stream, " ").is_err() {
                    *err = Some("failed writing autohelp output".to_string());
                    return;
                }
            }
            if writeln!(stream, "{}", descrip).is_err() {
                *err = Some("failed writing autohelp output".to_string());
                return;
            }
        }
    }

    if let Some(opts) = options {
        if let Some(suffix) = &opts.suffix {
            if write!(stream, "{}{}\n", nl, suffix).is_err() {
                *err = Some("failed writing autohelp output".to_string());
            }
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
        {
            *$err_ptr = None;

            let mut __xopt_ctx = $crate::xopt::xopt_context(
                Some($name),
                $options,
                $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
                $err_ptr,
            );

            if (*$err_ptr).is_none() {
                if let Some(__xopt_ctx_ref) = __xopt_ctx.as_mut() {
                    *$extrac_ptr = $crate::xopt::xopt_parse(
                        __xopt_ctx_ref,
                        $argc,
                        $argv,
                        ($config_ptr as *mut _) as *mut u8,
                        $extrav_ptr,
                        $err_ptr,
                    );

                    if (*$err_ptr).is_none() && ($config_ptr).help {
                        let __xopt_autohelp_opts = $crate::xopt::XoptAutohelpOptions {
                            usage: Some(($autohelp_usage).to_string()),
                            prefix: Some(($autohelp_prefix).to_string()),
                            suffix: Some(($autohelp_suffix).to_string()),
                            spacer: $autohelp_spacer,
                        };
                        $crate::xopt::xopt_autohelp(
                            __xopt_ctx_ref,
                            $autohelp_file,
                            Some(&__xopt_autohelp_opts),
                            $err_ptr,
                        );
                    }
                }
            }
        }
    };
}

enum ParseResult {
    Extra,
    Option,
}

fn parse_arg(
    ctx: &mut XoptContext,
    argc: usize,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
) -> ParseResult {
    let full_arg = argv[*argi];

    if ctx.doubledash {
        return ParseResult::Extra;
    }

    let size = get_size(full_arg);
    let arg = &full_arg[size..];
    let mut length = arg.len();

    if size == 1 && length == 0 {
        return ParseResult::Extra;
    }

    if size == 2 && length == 0 {
        ctx.doubledash = true;
        return ParseResult::Option;
    }

    match size {
        1 => {
            if length > 1 && ctx.flags & XOPT_CTX_NOCONDENSE != 0 {
                *err = Some(format!("short options cannot be combined: {}", full_arg));
            } else if length > 1 && ctx.flags & XOPT_CTX_SLOPPYSHORTS != 0 {
                let key = &arg[..1];
                let (arg_requirement, option) = get_arg(key, 1, &ctx.options, size);
                if let Some(option) = option {
                    if arg_requirement == 0 {
                        *err = Some(format!("option doesn't take a value: -{}", key));
                    } else {
                        set_value(data, option, Some(&arg[1..]), false, err);
                    }
                } else if ctx.flags & XOPT_CTX_STRICT != 0 {
                    *err = Some(format!("invalid option: -{}", key));
                }
            } else {
                let chars: Vec<char> = arg.chars().collect();
                let mut idx = 0usize;
                while idx < chars.len() {
                    let ch = chars[idx];
                    let (arg_requirement, option) =
                        get_arg(&arg[idx..], 1, &ctx.options, size);

                    let Some(option) = option else {
                        if ctx.flags & XOPT_CTX_STRICT != 0 {
                            *err = Some(format!("invalid option: -{}", ch));
                        }
                        break;
                    };

                    match arg_requirement {
                        0 => set_value(data, option, None, false, err),
                        1 => {
                            if *argi + 1 < argc && get_size(argv[*argi + 1]) == 0 {
                                *argi += 1;
                                set_value(data, option, Some(argv[*argi]), false, err);
                            } else {
                                set_value(data, option, None, false, err);
                            }
                        }
                        2 => {
                            if idx + 1 == chars.len() {
                                if *argi + 1 < argc {
                                    if get_size(argv[*argi + 1]) != 0 {
                                        *err = Some(format!(
                                            "missing option value: -{}",
                                            option.short_arg
                                        ));
                                    } else {
                                        *argi += 1;
                                        set_value(data, option, Some(argv[*argi]), false, err);
                                    }
                                } else {
                                    *err = Some(format!(
                                        "missing option value: -{}",
                                        option.short_arg
                                    ));
                                }
                            } else {
                                *err = Some(format!(
                                    "combined short option requiring value is not last: -{}",
                                    option.short_arg
                                ));
                            }
                        }
                        _ => {}
                    }

                    if err.is_some() {
                        break;
                    }

                    idx += 1;
                    length = length.saturating_sub(1);
                    let _ = length;
                }
            }
            ParseResult::Option
        }
        2 => {
            let mut opt_name = arg;
            let mut val_start = None;
            if let Some(eq_idx) = arg.find('=') {
                opt_name = &arg[..eq_idx];
                let value = &arg[eq_idx + 1..];
                if !value.is_empty() {
                    val_start = Some(value);
                }
                length = opt_name.len();
            }

            let (arg_requirement, option) = get_arg(opt_name, length, &ctx.options, size);
            if let Some(option) = option {
                match arg_requirement {
                    0 => {
                        if val_start.is_some() {
                            *err = Some(format!("option doesn't take a value: --{}", arg));
                        }
                        if err.is_none() {
                            set_value(data, option, val_start, true, err);
                        }
                    }
                    2 => {
                        if val_start.is_none() {
                            *err = Some(format!("missing option value: --{}", arg));
                        }
                        if err.is_none() {
                            set_value(data, option, val_start, true, err);
                        }
                    }
                    _ => {
                        set_value(data, option, val_start, true, err);
                    }
                }
            } else {
                *err = Some(format!("invalid option: --{}", opt_name));
            }
            ParseResult::Option
        }
        _ => ParseResult::Extra,
    }
}

fn get_size(arg: &str) -> usize {
    arg.chars().take(2).take_while(|&ch| ch == '-').count()
}

fn get_arg<'a>(
    arg: &str,
    len: usize,
    options: &'a [XoptOption],
    size: usize,
) -> (i32, Option<&'a XoptOption>) {
    let option = options
        .iter()
        .take_while(|o| o.long_arg.is_some() || o.short_arg != '\0')
        .find(|opt| {
            if size == 1 {
                opt.short_arg != '\0' && arg.chars().next() == Some(opt.short_arg)
            } else {
                opt.long_arg
                    .as_deref()
                    .map(|long_arg| long_arg.len() == len && &arg[..len] == long_arg)
                    .unwrap_or(false)
            }
        });

    let requirement = match option {
        None => 0,
        Some(opt) if opt.options & XOPT_TYPE_BOOL != 0 => 0,
        Some(opt) if opt.options & XOPT_OPTIONAL != 0 => 1,
        Some(_) => 2,
    };

    (requirement, option)
}

fn set_value(
    data: *mut u8,
    option: &XoptOption,
    value: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    if let Some(callback) = option.callback {
        callback(value, data, option, long_arg, err);
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
    if option.options & XOPT_TYPE_BOOL == 0 && value.unwrap_or("").is_empty() {
        return;
    }

    let target = data.wrapping_add(option.offset);

    match option.options & 0x3F {
        XOPT_TYPE_BOOL => {
            unsafe { *(target as *mut bool) = true };
        }
        XOPT_TYPE_STRING => {
            unsafe { *(target as *mut String) = value.unwrap_or("").to_string() };
        }
        XOPT_TYPE_INT => match parse_c_int(value.unwrap_or("")) {
            Some(parsed) => unsafe { *(target as *mut i32) = parsed as i32 },
            None => set_number_error(option, value.unwrap_or(""), long_arg, err),
        },
        XOPT_TYPE_LONG => match parse_c_long(value.unwrap_or("")) {
            Some(parsed) => unsafe { *(target as *mut i64) = parsed },
            None => set_number_error(option, value.unwrap_or(""), long_arg, err),
        },
        XOPT_TYPE_FLOAT => match value.unwrap_or("").parse::<f32>() {
            Ok(parsed) => unsafe { *(target as *mut f32) = parsed },
            Err(_) => set_number_error(option, value.unwrap_or(""), long_arg, err),
        },
        XOPT_TYPE_DOUBLE => match value.unwrap_or("").parse::<f64>() {
            Ok(parsed) => unsafe { *(target as *mut f64) = parsed },
            Err(_) => set_number_error(option, value.unwrap_or(""), long_arg, err),
        },
        _ => {}
    }
}

fn set_number_error(option: &XoptOption, value: &str, long_arg: bool, err: &mut Option<String>) {
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

fn parse_c_int(value: &str) -> Option<i64> {
    parse_c_long(value)
}

fn parse_c_long(value: &str) -> Option<i64> {
    if value.is_empty() {
        return None;
    }

    let (negative, rest) = if let Some(stripped) = value.strip_prefix('-') {
        (true, stripped)
    } else if let Some(stripped) = value.strip_prefix('+') {
        (false, stripped)
    } else {
        (false, value)
    };

    if rest.is_empty() {
        return None;
    }

    let (radix, digits) = if rest.starts_with("0x") || rest.starts_with("0X") {
        (16, &rest[2..])
    } else if rest.len() > 1 && rest.starts_with('0') {
        (8, &rest[1..])
    } else {
        (10, rest)
    };

    if digits.is_empty() {
        return Some(0);
    }

    i64::from_str_radix(digits, radix)
        .ok()
        .map(|parsed| if negative { -parsed } else { parsed })
}
