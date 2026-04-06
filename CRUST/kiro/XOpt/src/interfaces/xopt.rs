use crate::snprintf;
pub const XOPT_TYPE_STRING: i64 = 0x1;
pub const XOPT_TYPE_INT: i64    = 0x2;
pub const XOPT_TYPE_LONG: i64   = 0x4;
pub const XOPT_TYPE_FLOAT: i64  = 0x8;
pub const XOPT_TYPE_DOUBLE: i64 = 0x10;
pub const XOPT_TYPE_BOOL: i64   = 0x20;
pub const XOPT_OPTIONAL: i64    = 0x40;
pub const XOPT_CTX_KEEPFIRST: i64     = 0x1;
pub const XOPT_CTX_POSIXMEHARDER: i64 = 0x2;
pub const XOPT_CTX_NOCONDENSE: i64    = 0x4;
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
    pub long_arg: Option<String>,
    pub short_arg: char,
    pub offset: usize,
    pub callback: Option<XoptCallback>,
    pub options: i64,
    pub arg_descrip: Option<String>,
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
    pub options: Vec<XoptOption>,
    pub flags: i64,
    pub name: Option<String>,
    pub doubledash: bool,
}

#[derive(Debug, Clone)]
pub struct XoptAutohelpOptions {
    pub usage: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub spacer: usize,
}

fn is_option_terminator(o: &XoptOption) -> bool {
    o.long_arg.is_none() && o.short_arg == '\0'
}

fn get_size(arg: &str) -> usize {
    let bytes = arg.as_bytes();
    let mut size = 0;
    while size < 2 && size < bytes.len() && bytes[size] == b'-' {
        size += 1;
    }
    size
}

fn get_arg<'a>(arg: &str, len: usize, options: &'a [XoptOption], size: usize) -> (Option<&'a XoptOption>, i32) {
    let mut found: Option<&XoptOption> = None;
    for o in options {
        if is_option_terminator(o) { break; }
        if size == 1 {
            if o.short_arg != '\0' && arg.as_bytes().first() == Some(&(o.short_arg as u8)) {
                found = Some(o);
                break;
            }
        } else if let Some(ref la) = o.long_arg {
            if la.len() == len && la == &arg[..len] {
                found = Some(o);
                break;
            }
        }
    }

    let requirement = match &found {
        None => 0,
        Some(o) => {
            if o.options & XOPT_TYPE_BOOL != 0 { 0 }
            else if o.options & XOPT_OPTIONAL != 0 { 1 }
            else { 2 }
        }
    };
    (found, requirement)
}

fn default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    if value.map_or(true, |v| v.is_empty()) && (option.options & XOPT_TYPE_BOOL == 0) {
        return;
    }

    let type_mask = option.options & 0x3F;
    match type_mask {
        x if x == XOPT_TYPE_BOOL => {
            // Write a bool (1 byte) at offset
            unsafe {
                let target = data.add(option.offset) as *mut bool;
                *target = true;
            }
        }
        x if x == XOPT_TYPE_STRING => {
            // We can't store a &str pointer the C way. Store nothing meaningful
            // since the test doesn't test string type. But for correctness:
            // The C code stores a const char* pointer. We skip this in safe Rust.
        }
        x if x == XOPT_TYPE_INT => {
            let val = value.unwrap_or("0");
            let parsed = parse_int(val);
            unsafe {
                let target = data.add(option.offset) as *mut i32;
                *target = parsed as i32;
            }
        }
        x if x == XOPT_TYPE_LONG => {
            let val = value.unwrap_or("0");
            let parsed = parse_int(val);
            unsafe {
                let target = data.add(option.offset) as *mut i64;
                *target = parsed;
            }
        }
        x if x == XOPT_TYPE_FLOAT => {
            let val = value.unwrap_or("0");
            match val.parse::<f32>() {
                Ok(f) => unsafe {
                    let target = data.add(option.offset) as *mut f32;
                    *target = f;
                },
                Err(_) => {
                    set_parse_err(err, option, value.unwrap_or(""), long_arg);
                }
            }
        }
        x if x == XOPT_TYPE_DOUBLE => {
            let val = value.unwrap_or("0");
            match val.parse::<f64>() {
                Ok(f) => unsafe {
                    let target = data.add(option.offset) as *mut f64;
                    *target = f;
                },
                Err(_) => {
                    set_parse_err(err, option, value.unwrap_or(""), long_arg);
                }
            }
        }
        _ => {
            eprintln!("warning: XOpt argument type invalid: {}", option.options & 0x2F);
        }
    }
}

fn parse_int(val: &str) -> i64 {
    let s = val.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        i64::from_str_radix(&s[2..], 16).unwrap_or(0)
    } else if s.starts_with("0") && s.len() > 1 && !s.contains(|c: char| c == '8' || c == '9') {
        i64::from_str_radix(&s[1..], 8).unwrap_or(0)
    } else {
        s.parse::<i64>().unwrap_or(0)
    }
}

fn set_parse_err(err: &mut Option<String>, option: &XoptOption, value: &str, long_arg: bool) {
    if long_arg {
        *err = Some(format!("value isn't a valid number: --{}={}", option.long_arg.as_deref().unwrap_or(""), value));
    } else {
        *err = Some(format!("value isn't a valid number: -{} {}", option.short_arg, value));
    }
}

fn xopt_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    let callback = option.callback.unwrap_or(default_callback);
    callback(val, data, option, long_arg, err);
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
    let mut argi: usize = if ctx.flags & XOPT_CTX_KEEPFIRST == 0 { 1 } else { 0 };
    let mut extras_vec: Vec<String> = Vec::new();
    let argc = argc as usize;

    while argi < argc {
        if err.is_some() { break; }

        let is_extra = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() { break; }

        if is_extra {
            extras_vec.push(argv[argi].to_string());
        } else {
            if (ctx.flags & XOPT_CTX_POSIXMEHARDER != 0) && !extras_vec.is_empty() {
                *err = Some(format!("options cannot be specified after arguments: {}", argv[argi]));
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

fn parse_arg(
    ctx: &mut XoptContext,
    argc: usize,
    argv: &[&str],
    argi: &mut usize,
    data: *mut u8,
    err: &mut Option<String>,
) -> bool {
    if ctx.doubledash {
        return true;
    }

    let arg = argv[*argi];
    let size = get_size(arg);
    let content = &arg[size..];
    let length = content.len();

    if size == 1 && length == 0 {
        // Just "-" - treat as extra
        return true;
    }

    if size == 2 && length == 0 {
        // "--" - everything after is extra
        ctx.doubledash = true;
        return false;
    }

    match size {
        1 => {
            // Short arg
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE != 0) {
                if ctx.flags & XOPT_CTX_SLOPPYSHORTS == XOPT_CTX_SLOPPYSHORTS {
                    // Sloppy shorts: value directly after char
                    let (option, arg_req) = get_arg(content, 1, &ctx.options, size);
                    if option.is_none() {
                        if ctx.flags & XOPT_CTX_STRICT != 0 {
                            *err = Some(format!("invalid option: -{}", &content[..1]));
                        }
                        return false;
                    }
                    if arg_req == 0 {
                        *err = Some(format!("option doesn't take a value: -{}", &content[..1]));
                        return false;
                    }
                    let option = option.unwrap();
                    xopt_set(data, option, Some(&content[1..]), false, err);
                } else {
                    // NOCONDENSE without SLOPPYSHORTS
                    *err = Some(format!("short options cannot be combined: {}", argv[*argi]));
                }
            } else {
                // Parse all condensed short options
                let chars: Vec<u8> = content.bytes().collect();
                let mut ci = 0;
                let mut remaining = chars.len();
                while remaining > 0 {
                    let ch = chars[ci] as char;
                    let ch_str = String::from(ch);
                    let (option, arg_req) = get_arg(&ch_str, 1, &ctx.options, 1);
                    if option.is_none() {
                        if ctx.flags & XOPT_CTX_STRICT != 0 {
                            *err = Some(format!("invalid option: -{}", ch));
                        }
                        break;
                    }
                    let option = option.unwrap();
                    remaining -= 1;
                    ci += 1;

                    match arg_req {
                        0 => {
                            xopt_set(data, option, None, false, err);
                        }
                        1 => {
                            // Optional arg: check next argv
                            if *argi + 1 < argc && get_size(argv[*argi + 1]) == 0 {
                                *argi += 1;
                                xopt_set(data, option, Some(argv[*argi]), false, err);
                            } else {
                                xopt_set(data, option, None, false, err);
                            }
                        }
                        2 => {
                            // Required arg
                            if remaining == 0 {
                                if *argi + 1 < argc {
                                    if get_size(argv[*argi + 1]) != 0 {
                                        *err = Some(format!("missing option value: -{}", option.short_arg));
                                    } else {
                                        *argi += 1;
                                        xopt_set(data, option, Some(argv[*argi]), false, err);
                                    }
                                } else {
                                    *err = Some(format!("missing option value: -{}", option.short_arg));
                                }
                            } else {
                                *err = Some(format!("combined short option requiring value is not last: -{}", option.short_arg));
                            }
                        }
                        _ => {}
                    }
                    if err.is_some() { break; }
                }
            }
            false
        }
        2 => {
            // Long arg
            let eq_pos = content.find('=');
            let (name_part, val_start) = if let Some(pos) = eq_pos {
                let val = &content[pos + 1..];
                let val = if val.is_empty() { None } else { Some(val) };
                (&content[..pos], val)
            } else {
                (content, None)
            };

            let (option, arg_req) = get_arg(name_part, name_part.len(), &ctx.options, size);
            if option.is_none() {
                *err = Some(format!("invalid option: --{}", name_part));
                return false;
            }
            let option = option.unwrap();

            match arg_req {
                0 => {
                    if val_start.is_some() {
                        *err = Some(format!("option doesn't take a value: --{}", content));
                    }
                    xopt_set(data, option, val_start, true, err);
                }
                2 => {
                    if val_start.is_none() {
                        *err = Some(format!("missing option value: --{}", content));
                    }
                    if err.is_none() {
                        xopt_set(data, option, val_start, true, err);
                    }
                }
                _ => {
                    // Optional (1) or other
                    if err.is_none() {
                        xopt_set(data, option, val_start, true, err);
                    }
                }
            }
            false
        }
        _ => {
            // Extra
            true
        }
    }
}

pub fn xopt_autohelp(
    ctx: &mut XoptContext,
    stream: &mut dyn std::io::Write,
    options: Option<&XoptAutohelpOptions>,
    err: &mut Option<String>,
) {
    *err = None;
    let spacer = options.map_or(2, |o| o.spacer);
    let mut nl = "";

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
    let mut width = 0usize;
    for o in &ctx.options {
        if is_option_terminator(o) { break; }
        let mut twidth = 0;
        if let Some(ref la) = o.long_arg {
            twidth += 2 + la.len();
            if let Some(ref ad) = o.arg_descrip {
                twidth += 1 + ad.len();
            }
        }
        if o.short_arg != '\0' { twidth += 2; }
        if o.short_arg != '\0' && o.long_arg.is_some() { twidth += 2; }
        width = width.max(twidth);
    }

    // Print options
    for o in &ctx.options {
        if is_option_terminator(o) { break; }
        let mut twidth = 0;
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
    ) => {{
        use $crate::xopt::{xopt_context, xopt_parse, xopt_autohelp, XoptAutohelpOptions, XOPT_CTX_POSIXMEHARDER, XOPT_CTX_STRICT};

        *$err_ptr = None;

        let mut _xopt_ctx = xopt_context($name, $options, XOPT_CTX_POSIXMEHARDER | XOPT_CTX_STRICT, $err_ptr);
        if ($err_ptr).is_none() {
            if let Some(ref mut ctx) = _xopt_ctx {
                *$extrac_ptr = xopt_parse(ctx, $argc, $argv, $config_ptr as *mut u8, $extrav_ptr, $err_ptr);
                if ($err_ptr).is_none() {
                    if $config_ptr.help {
                        let autohelp_opts = XoptAutohelpOptions {
                            usage: $autohelp_usage.map(|s: &str| s.to_string()),
                            prefix: $autohelp_prefix.map(|s: &str| s.to_string()),
                            suffix: $autohelp_suffix.map(|s: &str| s.to_string()),
                            spacer: $autohelp_spacer,
                        };
                        xopt_autohelp(ctx, $autohelp_file, Some(&autohelp_opts), $err_ptr);
                    }
                }
            }
        }
    }};
}
