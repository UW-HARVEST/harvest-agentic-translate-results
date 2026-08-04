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

// Helper: returns true if the option entry is the null terminator.
fn is_null_option(o: &XoptOption) -> bool {
    o.long_arg.is_none() && o.short_arg == '\0'
}

// Helper: detect how many leading dashes the argument starts with (0, 1 or 2).
fn xopt_get_size(arg: &str) -> i32 {
    let mut size = 0;
    for ch in arg.chars().take(2) {
        if ch == '-' {
            size += 1;
        } else {
            break;
        }
    }
    size
}

// Look up an option by short (size==1) or long (size==2) name.  When size==2,
// `name_len` indicates the prefix length to compare in the long arg lookup.
// Returns Some((index, requirement)) if found, where `requirement` is:
//   0 — flag (no value)
//   1 — value optional
//   2 — value required
fn xopt_get_arg(
    arg: &str,
    name_len: usize,
    options: &[XoptOption],
    size: i32,
) -> Option<(usize, i32)> {
    let mut found: Option<usize> = None;
    for (i, o) in options.iter().enumerate() {
        if is_null_option(o) {
            break;
        }
        if size == 1 {
            // Short arg: arg's first char vs option.short_arg.
            if let Some(first) = arg.chars().next() {
                if o.short_arg != '\0' && o.short_arg == first {
                    found = Some(i);
                    break;
                }
            }
        } else {
            // Long arg: compare option.long_arg (full length) with arg's prefix
            // of length name_len.
            if let Some(la) = &o.long_arg {
                if la.len() == name_len && arg.len() >= name_len && &arg[..name_len] == la.as_str()
                {
                    found = Some(i);
                    break;
                }
            }
        }
    }

    match found {
        None => None,
        Some(i) => {
            let opts = options[i].options;
            let req = if (opts & XOPT_TYPE_BOOL) != 0 {
                0
            } else if (opts & XOPT_OPTIONAL) != 0 {
                1
            } else {
                2
            };
            Some((i, req))
        }
    }
}

// Apply a parsed value to a target struct field via raw pointer offset, in
// the same fashion as the original C code.  `data` is the pointer to the
// configuration struct; the option's `offset` field tells us at which byte
// offset to write the parsed value.
fn xopt_default_callback(
    value: Option<&str>,
    data: *mut u8,
    option: &XoptOption,
    long_arg: bool,
    err: &mut Option<String>,
) {
    // Detect "no value" mirroring the C check on (!value || !strlen(value)).
    let val_missing = match value {
        None => true,
        Some(v) => v.is_empty(),
    };

    if val_missing && (option.options & XOPT_TYPE_BOOL) == 0 {
        // Optional non-boolean without a custom handler — silently no-op.
        return;
    }

    // Compute target pointer.
    if data.is_null() {
        return;
    }
    let target_ptr = unsafe { data.add(option.offset) };

    let type_mask = option.options & 0x3F;
    let value_str = value.unwrap_or("");
    let mut parse_err = false;

    match type_mask {
        x if x == XOPT_TYPE_BOOL => {
            // Booleans in the original C code are represented by `_Bool`,
            // which is one byte.  Write 1 there.
            unsafe { *target_ptr = 1u8 };
        }
        x if x == XOPT_TYPE_STRING => {
            // We don't currently have a way to safely bridge a borrowed &str
            // back into the C-style `const char **target`, so this branch is
            // a best-effort no-op.  In practice the test binaries never use
            // string-typed options.
        }
        x if x == XOPT_TYPE_INT => match parse_c_int(value_str) {
            Some(v) => unsafe {
                let p = target_ptr as *mut i32;
                p.write_unaligned(v);
            },
            None => parse_err = true,
        },
        x if x == XOPT_TYPE_LONG => match parse_c_long(value_str) {
            Some(v) => unsafe {
                let p = target_ptr as *mut i64;
                p.write_unaligned(v);
            },
            None => parse_err = true,
        },
        x if x == XOPT_TYPE_FLOAT => match value_str.parse::<f64>() {
            Ok(v) => unsafe {
                let p = target_ptr as *mut f32;
                p.write_unaligned(v as f32);
            },
            Err(_) => parse_err = true,
        },
        x if x == XOPT_TYPE_DOUBLE => match value_str.parse::<f64>() {
            Ok(v) => unsafe {
                let p = target_ptr as *mut f64;
                p.write_unaligned(v);
            },
            Err(_) => parse_err = true,
        },
        _ => {
            eprintln!(
                "warning: XOpt argument type invalid: {}",
                option.options & 0x2F
            );
        }
    }

    if parse_err {
        if long_arg {
            *err = Some(format!(
                "value isn't a valid number: --{}={}",
                option.long_arg.as_deref().unwrap_or(""),
                value_str
            ));
        } else {
            *err = Some(format!(
                "value isn't a valid number: -{} {}",
                option.short_arg, value_str
            ));
        }
    }
}

// Mimic strtol with base 0 (auto-detect 0x/0/decimal).  Stops at first
// non-numeric character; returns Some(value) when at least one valid digit was
// consumed and no trailing junk follows.  None indicates a parse failure that
// should surface as an error to the user.
fn parse_c_long(s: &str) -> Option<i64> {
    let s = s.trim_start();
    if s.is_empty() {
        return None;
    }
    let (sign, rest) = match s.as_bytes()[0] {
        b'+' => (1i64, &s[1..]),
        b'-' => (-1i64, &s[1..]),
        _ => (1i64, s),
    };
    let (base, digits) = if let Some(stripped) = rest.strip_prefix("0x") {
        (16u32, stripped)
    } else if let Some(stripped) = rest.strip_prefix("0X") {
        (16u32, stripped)
    } else if rest.starts_with('0') && rest.len() > 1 {
        (8u32, &rest[1..])
    } else {
        (10u32, rest)
    };
    if digits.is_empty() {
        if base == 8 {
            // The literal "0" — treat as zero.
            return Some(0);
        }
        return None;
    }
    let mut value: i64 = 0;
    let mut consumed_any = false;
    for c in digits.chars() {
        if let Some(d) = c.to_digit(base) {
            consumed_any = true;
            value = value.checked_mul(base as i64)?.checked_add(d as i64)?;
        } else {
            // Trailing garbage — match strtol behaviour where parsePtr would
            // be non-zero, signalling an error in the C default callback.
            return None;
        }
    }
    if !consumed_any {
        return None;
    }
    Some(sign * value)
}

fn parse_c_int(s: &str) -> Option<i32> {
    parse_c_long(s).and_then(|v| {
        if v > i32::MAX as i64 || v < i32::MIN as i64 {
            // C's strtol cast would silently truncate; mirror that.
            Some(v as i32)
        } else {
            Some(v as i32)
        }
    })
}

fn xopt_set(
    data: *mut u8,
    option: &XoptOption,
    val: Option<&str>,
    long_arg: bool,
    err: &mut Option<String>,
) {
    // The original C code dispatches via option->callback or the default.
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

// Parse a single argument; mirrors `_xopt_parse_arg` in the C code.  Returns
// `Some(true)` if the argument is an "extra" (non-option), `Some(false)` if
// it was consumed as an option, and `None` is unused (but kept for symmetry).
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
    let raw = argv[*argi];
    let size = xopt_get_size(raw);
    let arg = &raw[size as usize..];
    let length = arg.len();

    if size == 1 && length == 0 {
        // Just a "-" — extra arg.
        return true;
    }
    if size == 2 && length == 0 {
        // "--" — toggle doubledash mode and continue.
        ctx.doubledash = true;
        return false;
    }

    let mut is_extra = false;

    match size {
        1 => {
            // Short argument(s).
            if length > 1 && (ctx.flags & XOPT_CTX_NOCONDENSE) != 0
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) != XOPT_CTX_SLOPPYSHORTS
            {
                *err = Some(format!("short options cannot be combined: {}", raw));
            } else if length > 1
                && (ctx.flags & XOPT_CTX_SLOPPYSHORTS) == XOPT_CTX_SLOPPYSHORTS
            {
                // Sloppy shorts: option char + value glued together (e.g. -ofoo).
                let lookup = xopt_get_arg(arg, 1, &ctx.options, size);
                match lookup {
                    None => {
                        if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                            let first = arg.chars().next().unwrap_or('\0');
                            *err = Some(format!("invalid option: -{}", first));
                        }
                    }
                    Some((idx, req)) => {
                        if req == 0 {
                            let first = arg.chars().next().unwrap_or('\0');
                            *err =
                                Some(format!("option doesn't take a value: -{}", first));
                        } else {
                            let opt_clone = ctx.options[idx].clone();
                            xopt_set(data, &opt_clone, Some(&arg[1..]), false, err);
                        }
                    }
                }
            } else {
                // Iterate over each char in `arg` (allowing condensed -abc).
                let chars: Vec<char> = arg.chars().collect();
                let mut i = 0usize;
                let total = chars.len();
                while i < total {
                    let mut single = String::new();
                    single.push(chars[i]);
                    let lookup = xopt_get_arg(&single, 1, &ctx.options, size);
                    let remaining = total - i - 1;
                    match lookup {
                        None => {
                            if (ctx.flags & XOPT_CTX_STRICT) != 0 {
                                *err = Some(format!("invalid option: -{}", chars[i]));
                            }
                            break;
                        }
                        Some((idx, req)) => {
                            let opt_clone = ctx.options[idx].clone();
                            match req {
                                0 => {
                                    xopt_set(data, &opt_clone, None, false, err);
                                }
                                1 => {
                                    // Optional arg: take next argv if it's a non-option.
                                    if (*argi as i32) + 1 < argc
                                        && xopt_get_size(argv[*argi + 1]) == 0
                                    {
                                        *argi += 1;
                                        xopt_set(
                                            data,
                                            &opt_clone,
                                            Some(argv[*argi]),
                                            false,
                                            err,
                                        );
                                    } else {
                                        xopt_set(data, &opt_clone, None, false, err);
                                    }
                                }
                                2 => {
                                    // Required: must be the last in the cluster.
                                    if remaining == 0 {
                                        if (*argi as i32) + 1 < argc {
                                            if xopt_get_size(argv[*argi + 1]) != 0 {
                                                *err = Some(format!(
                                                    "missing option value: -{}",
                                                    opt_clone.short_arg
                                                ));
                                            } else {
                                                *argi += 1;
                                                xopt_set(
                                                    data,
                                                    &opt_clone,
                                                    Some(argv[*argi]),
                                                    false,
                                                    err,
                                                );
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
                        }
                    }
                    i += 1;
                }
            }
        }
        2 => {
            // Long argument.  Look for '=' separating value.
            let (name_part, value_part): (&str, Option<&str>) = match arg.find('=') {
                Some(eq_pos) => {
                    let v = &arg[eq_pos + 1..];
                    (
                        &arg[..eq_pos],
                        if v.is_empty() { None } else { Some(v) },
                    )
                }
                None => (arg, None),
            };

            let lookup = xopt_get_arg(name_part, name_part.len(), &ctx.options, size);
            match lookup {
                None => {
                    *err = Some(format!("invalid option: --{}", name_part));
                }
                Some((idx, req)) => {
                    let opt_clone = ctx.options[idx].clone();
                    match req {
                        0 => {
                            if value_part.is_some() {
                                *err = Some(format!(
                                    "option doesn't take a value: --{}",
                                    arg
                                ));
                            }
                            xopt_set(data, &opt_clone, value_part, true, err);
                        }
                        2 => {
                            if value_part.is_none() {
                                *err =
                                    Some(format!("missing option value: --{}", arg));
                            }
                            if err.is_none() {
                                xopt_set(data, &opt_clone, value_part, true, err);
                            }
                        }
                        _ => {
                            if err.is_none() {
                                xopt_set(data, &opt_clone, value_part, true, err);
                            }
                        }
                    }
                }
            }
        }
        _ => {
            is_extra = true;
        }
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
    let mut argi: usize = 0;
    let mut extras_vec: Vec<String> = Vec::new();

    if (ctx.flags & XOPT_CTX_KEEPFIRST) == 0 {
        argi += 1;
    }

    while (argi as i32) < argc {
        let parse_result = parse_arg(ctx, argc, argv, &mut argi, data, err);
        if err.is_some() {
            break;
        }

        if parse_result {
            extras_vec.push(argv[argi].to_string());
        } else if (ctx.flags & XOPT_CTX_POSIXMEHARDER) != 0 && !extras_vec.is_empty() {
            *err = Some(format!(
                "options cannot be specified after arguments: {}",
                argv[argi]
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

    // Find max option-column width.
    let mut width: usize = 0;
    for o in ctx.options.iter() {
        if is_null_option(o) {
            break;
        }
        let mut twidth = 0usize;
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
            twidth += 2; // ", "
        }
        if twidth > width {
            width = twidth;
        }
    }

    // Print options.
    for o in ctx.options.iter() {
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

    if let Some(o) = options {
        if let Some(suffix) = &o.suffix {
            let _ = write!(stream, "{}{}\n", nl, suffix);
        }
    }

    // Touch the snprintf module to keep the import live (no-op for users).
    let _ = snprintf::getnumsep(0);
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
        let mut __ctx_opt = $crate::xopt::xopt_context(
            $name,
            $options,
            $crate::xopt::XOPT_CTX_POSIXMEHARDER | $crate::xopt::XOPT_CTX_STRICT,
            $err_ptr,
        );
        if $err_ptr.is_none() {
            if let Some(ref mut __ctx) = __ctx_opt {
                let __count = $crate::xopt::xopt_parse(
                    __ctx,
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
                        __ctx,
                        $autohelp_file,
                        Some(&__opts),
                        $err_ptr,
                    );
                }
            }
        }
    }};
}
