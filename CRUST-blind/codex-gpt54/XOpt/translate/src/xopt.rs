use crate::snprintf;
use std::cmp::min;
use std::io::Write;

pub const XOPT_TYPE_STRING: i64 = 0x1; // const char * type
pub const XOPT_TYPE_INT: i64 = 0x2; // int type
pub const XOPT_TYPE_LONG: i64 = 0x4; // long type
pub const XOPT_TYPE_FLOAT: i64 = 0x8; // float type
pub const XOPT_TYPE_DOUBLE: i64 = 0x10; // double type
pub const XOPT_TYPE_BOOL: i64 = 0x20; // boolean (int) type
/// Indicates that the argument value is optional.
pub const XOPT_OPTIONAL: i64 = 0x40;
/// Bitmask constants for context flags.
pub const XOPT_CTX_KEEPFIRST: i64 = 0x1;
pub const XOPT_CTX_POSIXMEHARDER: i64 = 0x2;
pub const XOPT_CTX_NOCONDENSE: i64 = 0x4;
/// `XOPT_CTX_SLOPPYSHORTS` is defined as 0x8 | 0x4 in C
pub const XOPT_CTX_SLOPPYSHORTS: i64 = 0x8 | XOPT_CTX_NOCONDENSE;
pub const XOPT_CTX_STRICT: i64 = 0x10;

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
        name: name.map(str::to_owned),
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
    *extras = None;

    let argc = if argc <= 0 { 0 } else { min(argc as usize, argv.len()) };
    let mut argi = if ctx.flags & XOPT_CTX_KEEPFIRST == 0 {
        1usize
    } else {
        0usize
    };
    let mut collected = Vec::new();

    while argi < argc {
        let is_extra = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            return 0;
        }

        if is_extra {
            collected.push(argv[argi].to_owned());
        } else if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !collected.is_empty() {
            *err = Some(format!(
                "options cannot be specified after arguments: {}",
                argv[argi]
            ));
            return 0;
        }

        argi += 1;
    }

    *extras = Some(collected);
    extras.as_ref().map_or(0, |v| v.len() as i32)
}

pub fn xopt_autohelp(
    ctx: &mut XoptContext,
    stream: &mut dyn Write,
    options: Option<&XoptAutohelpOptions>,
    err: &mut Option<String>,
) {
    *err = None;

    let spacer = options.map_or(2, |o| o.spacer);
    let mut nl = "";

    if let Some(usage) = options.and_then(|o| o.usage.as_deref()) {
        if stream.write_all(format!("{nl}{usage}\n").as_bytes()).is_err() {
            *err = Some("failed to write help output".to_string());
            return;
        }
        nl = "\n";
    }

    if let Some(prefix) = options.and_then(|o| o.prefix.as_deref()) {
        if stream
            .write_all(format!("{nl}{prefix}\n\n").as_bytes())
            .is_err()
        {
            *err = Some("failed to write help output".to_string());
            return;
        }
        nl = "\n";
    }

    let mut width = 0usize;
    for option in iter_options(&ctx.options) {
        let mut twidth = 0usize;
        if let Some(long_arg) = option.long_arg.as_deref() {
            twidth += 2 + long_arg.len();
            if let Some(arg_descrip) = option.arg_descrip.as_deref() {
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

    for option in iter_options(&ctx.options) {
        let mut line = String::new();
        let mut twidth = 0usize;

        if option.short_arg != '\0' {
            line.push('-');
            line.push(option.short_arg);
            twidth += 2;
        }

        if option.short_arg != '\0' && option.long_arg.is_some() {
            line.push_str(", ");
            twidth += 2;
        }

        if let Some(long_arg) = option.long_arg.as_deref() {
            line.push_str("--");
            line.push_str(long_arg);
            twidth += 2 + long_arg.len();
            if let Some(arg_descrip) = option.arg_descrip.as_deref() {
                line.push('=');
                line.push_str(arg_descrip);
                twidth += 1 + arg_descrip.len();
            }
        }

        if let Some(description) = option.descrip.as_deref() {
            while twidth < width + spacer {
                line.push(' ');
                twidth += 1;
            }
            line.push_str(description);
        }
        line.push('\n');

        if stream.write_all(line.as_bytes()).is_err() {
            *err = Some("failed to write help output".to_string());
            return;
        }
    }

    if let Some(suffix) = options.and_then(|o| o.suffix.as_deref()) {
        if stream.write_all(format!("{nl}{suffix}\n").as_bytes()).is_err() {
            *err = Some("failed to write help output".to_string());
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
        if let Some(mut __xopt_ctx) = $crate::xopt::xopt_context(
            Some($name),
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        ) {
            *$extrac_ptr = $crate::xopt::xopt_parse(
                &mut __xopt_ctx,
                $argc,
                $argv,
                ($config_ptr as *mut _) as *mut u8,
                $extrav_ptr,
                $err_ptr,
            );

            if $err_ptr.is_none() && $config_ptr.help {
                let __xopt_autohelp_opts = $crate::xopt::XoptAutohelpOptions {
                    usage: Some(($autohelp_usage).to_string()),
                    prefix: Some(($autohelp_prefix).to_string()),
                    suffix: Some(($autohelp_suffix).to_string()),
                    spacer: $autohelp_spacer,
                };
                $crate::xopt::xopt_autohelp(
                    &mut __xopt_ctx,
                    $autohelp_file,
                    Some(&__xopt_autohelp_opts),
                    $err_ptr,
                );
            }
        }
    }};
}

fn iter_options(options: &[XoptOption]) -> impl Iterator<Item = &XoptOption> {
    options
        .iter()
        .take_while(|o| o.long_arg.is_some() || o.short_arg != '\0')
}

fn parse_arg(
    ctx: &mut XoptContext,
    argc: usize,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    let original = argv[*argi];
    if ctx.doubledash {
        return true;
    }

    let size = get_size(original);
    let arg = &original[size as usize..];
    let length = arg.len();

    if size == 1 && length == 0 {
        return true;
    }

    if size == 2 && length == 0 {
        ctx.doubledash = true;
        return false;
    }

    match size {
        1 => parse_short_arg(ctx, argc, argv, argi, data, arg, err),
        2 => parse_long_arg(ctx, data, arg, err),
        _ => true,
    }
}

fn parse_short_arg(
    ctx: &mut XoptContext,
    argc: usize,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    arg: &str,
    err: &mut Option<String>,
) -> bool {
    let chars: Vec<char> = arg.chars().collect();
    let length = chars.len();

    if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0 {
        *err = Some(format!("short options cannot be combined: {}", argv[*argi]));
        return false;
    } else if length > 1 && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != 0 {
        let Some(first) = chars.first().copied() else {
            return false;
        };
        let (requirement, option) = get_arg_short(first, &ctx.options);
        if option.is_none() {
            if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                *err = Some(format!("invalid option: -{first}"));
            }
            return false;
        }
        let option = option.expect("checked above");
        if requirement == 0 {
            *err = Some(format!("option doesn't take a value: -{first}"));
            return false;
        }
        let value = &arg[first.len_utf8()..];
        set_value(data, option, Some(value), false, err);
        return false;
    }

    for (index, ch) in chars.iter().copied().enumerate() {
        let (requirement, option) = get_arg_short(ch, &ctx.options);
        let Some(option) = option else {
            if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                *err = Some(format!("invalid option: -{ch}"));
            }
            break;
        };

        match requirement {
            0 => {
                set_value(data, option, None, false, err);
            }
            1 => {
                if *argi + 1 < argc && get_size(argv[*argi + 1]) == 0 {
                    *argi += 1;
                    set_value(data, option, Some(argv[*argi]), false, err);
                } else {
                    set_value(data, option, None, false, err);
                }
            }
            2 => {
                if index + 1 == length {
                    if *argi + 1 < argc {
                        if get_size(argv[*argi + 1]) != 0 {
                            *err = Some(format!("missing option value: -{}", option.short_arg));
                        } else {
                            *argi += 1;
                            set_value(data, option, Some(argv[*argi]), false, err);
                        }
                    } else {
                        *err = Some(format!("missing option value: -{}", option.short_arg));
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
    }

    false
}

fn parse_long_arg(
    ctx: &mut XoptContext,
    data: *mut u8,
    arg: &str,
    err: &mut Option<String>,
) -> bool {
    let (name, value) = match arg.split_once('=') {
        Some((name, value)) if value.is_empty() => (name, None),
        Some((name, value)) => (name, Some(value)),
        None => (arg, None),
    };

    let (requirement, option) = get_arg_long(name, &ctx.options);
    let Some(option) = option else {
        *err = Some(format!("invalid option: --{name}"));
        return false;
    };

    match requirement {
        0 => {
            if value.is_some() {
                *err = Some(format!("option doesn't take a value: --{arg}"));
            }
            if err.is_none() {
                set_value(data, option, value, true, err);
                if err.is_none() {
                    set_value(data, option, value, true, err);
                }
            }
        }
        2 => {
            if value.is_none() {
                *err = Some(format!("missing option value: --{arg}"));
            } else {
                set_value(data, option, value, true, err);
            }
        }
        _ => {
            set_value(data, option, value, true, err);
        }
    }

    false
}

fn get_size(arg: &str) -> i32 {
    let bytes = arg.as_bytes();
    let mut size = 0;
    while size < 2 && size < bytes.len() && bytes[size] == b'-' {
        size += 1;
    }
    size as i32
}

fn get_arg_short(ch: char, options: &[XoptOption]) -> (i32, Option<&XoptOption>) {
    let option = iter_options(options).find(|o| o.short_arg == ch);
    (arg_requirement(option), option)
}

fn get_arg_long<'a>(name: &str, options: &'a [XoptOption]) -> (i32, Option<&'a XoptOption>) {
    let option = iter_options(options).find(|o| o.long_arg.as_deref() == Some(name));
    (arg_requirement(option), option)
}

fn arg_requirement(option: Option<&XoptOption>) -> i32 {
    match option {
        None => 0,
        Some(option) if (option.options & XOPT_TYPE_BOOL) != 0 => 0,
        Some(option) if (option.options & XOPT_OPTIONAL) != 0 => 1,
        Some(_) => 2,
    }
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
    if value.is_none_or(str::is_empty) && (option.options & XOPT_TYPE_BOOL) == 0 {
        return;
    }

    let value = value.unwrap_or("");
    let target = unsafe { data.add(option.offset) };

    match option.options & 0x3F {
        XOPT_TYPE_BOOL => {
            unsafe { *(target as *mut bool) = true };
            return;
        }
        XOPT_TYPE_STRING => {
            unsafe { *(target as *mut String) = value.to_owned() };
            return;
        }
        XOPT_TYPE_INT => match parse_i32_c_style(value) {
            Some(parsed) => unsafe { *(target as *mut i32) = parsed },
            None => {}
        },
        XOPT_TYPE_LONG => match parse_i64_c_style(value) {
            Some(parsed) => unsafe { *(target as *mut i64) = parsed },
            None => {}
        },
        XOPT_TYPE_FLOAT => match value.parse::<f32>() {
            Ok(parsed) => unsafe { *(target as *mut f32) = parsed },
            Err(_) => {
                set_number_error(option, long_arg, value, err);
                return;
            }
        },
        XOPT_TYPE_DOUBLE => match value.parse::<f64>() {
            Ok(parsed) => unsafe { *(target as *mut f64) = parsed },
            Err(_) => {
                set_number_error(option, long_arg, value, err);
                return;
            }
        },
        other => {
            eprintln!("warning: XOpt argument type invalid: {other}");
            return;
        }
    }

    let parse_failed = matches!(option.options & 0x3F, XOPT_TYPE_INT | XOPT_TYPE_LONG)
        && match option.options & 0x3F {
            XOPT_TYPE_INT => parse_i32_c_style(value).is_none(),
            XOPT_TYPE_LONG => parse_i64_c_style(value).is_none(),
            _ => false,
        };

    if parse_failed {
        set_number_error(option, long_arg, value, err);
    }
}

fn set_number_error(option: &XoptOption, long_arg: bool, value: &str, err: &mut Option<String>) {
    *err = Some(if long_arg {
        format!(
            "value isn't a valid number: --{}={value}",
            option.long_arg.as_deref().unwrap_or("")
        )
    } else {
        format!(
            "value isn't a valid number: -{} {value}",
            option.short_arg
        )
    });
}

fn parse_i32_c_style(value: &str) -> Option<i32> {
    parse_i64_c_style(value).and_then(|v| i32::try_from(v).ok())
}

fn parse_i64_c_style(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (negative, rest) = match trimmed.as_bytes()[0] {
        b'+' => (false, &trimmed[1..]),
        b'-' => (true, &trimmed[1..]),
        _ => (false, trimmed),
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

    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }

    let value = i64::from_str_radix(digits, radix).ok()?;
    Some(if negative { -value } else { value })
}

#[allow(dead_code)]
fn _touch_snprintf() {
    let _ = snprintf::mypow10(0);
}
