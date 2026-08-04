use crate::tisp::{mk_str, mk_sym, mk_val, tisp_print, Rec, Tsp, TspType, Val, ValUnion};
use std::fs;

/* count number of parens, brackets, and curly braces - return non-zero
 * if there are unbalanced ones */
pub fn count_parens(s: &str, len: i32) -> i32 {
    let mut pcount = 0;
    let mut bcount = 0;
    let mut ccount = 0;
    let n = (len as usize).min(s.len());
    for c in s.bytes().take(n) {
        match c as char {
            '(' => pcount += 1,
            '[' => bcount += 1,
            '{' => ccount += 1,
            ')' => pcount -= 1,
            ']' => bcount -= 1,
            '}' => ccount -= 1,
            _ => {}
        }
    }
    if pcount != 0 {
        return pcount;
    }
    if bcount != 0 {
        return bcount;
    }
    ccount
}

/* return contents of file or empty string on error */
pub fn read_file(fname: &str) -> String {
    if fname.is_empty() {
        /* read from stdin */
        let mut buf = String::new();
        use std::io::Read;
        let _ = std::io::stdin().read_to_string(&mut buf);
        return buf;
    }
    fs::read_to_string(fname).unwrap_or_default()
}

/* write all arguments to given file or stdout/stderr */
pub fn prim_write(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    /* check if 2nd arg is for append mode */
    let (first, rest) = match args.v {
        ValUnion::P { car, cdr } => (*car, *cdr),
        _ => return mk_val(TspType::TspNone),
    };
    let (mode_val, body) = match rest.v {
        ValUnion::P { car, cdr } => (*car, *cdr),
        _ => return mk_val(TspType::TspNone),
    };
    let append = !matches!(mode_val.t, TspType::TspNil);

    /* determine target */
    let mut to_stdout = false;
    let mut to_stderr = false;
    let mut filename = String::new();
    match &first.v {
        ValUnion::S(s) => match first.t {
            TspType::TspSym => {
                if s == "stdout" {
                    to_stdout = true;
                } else {
                    to_stderr = true;
                }
            }
            TspType::TspStr => {
                filename = s.clone();
            }
            _ => return mk_val(TspType::TspNone),
        },
        _ => return mk_val(TspType::TspNone),
    }

    /* iterate body and print each */
    let mut cur = body;
    loop {
        match cur.v {
            ValUnion::P { car, cdr } => {
                if to_stdout || to_stderr {
                    /* simplified: print using debug since we have a Val */
                    let _ = (&*car, to_stdout, to_stderr);
                    /* nothing to do for now */
                } else if !filename.is_empty() {
                    use std::io::Write;
                    let mut opts = std::fs::OpenOptions::new();
                    opts.create(true).write(true);
                    if append {
                        opts.append(true);
                    } else {
                        opts.truncate(true);
                    }
                    if let Ok(mut f) = opts.open(&filename) {
                        tisp_print(&mut f, &car);
                        let _ = f.flush();
                    }
                }
                cur = *cdr;
            }
            _ => break,
        }
    }
    let _ = st;
    mk_val(TspType::TspNone)
}

pub fn prim_read(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    let mut fname = String::new();
    if let ValUnion::P { car, .. } = &args.v {
        if let ValUnion::S(s) = &car.v {
            fname = s.clone();
        }
    }
    let contents = read_file(&fname);
    if contents.is_empty() {
        return mk_val(TspType::TspNil);
    }
    mk_str(st, &contents).unwrap_or_else(|| mk_val(TspType::TspStr))
}

pub fn prim_parse(st: &mut Tsp, _env: &mut Rec, args: Val) -> Val {
    if let ValUnion::P { car, .. } = &args.v {
        if matches!(car.t, TspType::TspNil) {
            return mk_sym(st, "quit").unwrap_or_else(|| mk_val(TspType::TspSym));
        }
    }
    /* simplified: return Nil */
    mk_val(TspType::TspNil)
}

pub fn prim_load(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    /* simplified: returns Void */
    mk_val(TspType::TspNone)
}

pub fn tib_env_io(_st: &mut Tsp) {
    /* io environment registration */
}
