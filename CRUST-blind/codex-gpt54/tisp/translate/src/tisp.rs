pub const TSP_REC_MAX_PRINT: usize = 64;
pub const TSP_SYM_CHARS: &str = "_!?@#$%&~*-";
pub const TSP_REC_FACTOR: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TspType {
    TspNone = 1 << 0,
    TspNil = 1 << 1,
    TspInt = 1 << 2,
    TspDec = 1 << 3,
    TspRatio = 1 << 4,
    TspStr = 1 << 5,
    TspSym = 1 << 6,
    TspPrim = 1 << 7,
    TspForm = 1 << 8,
    TspFunc = 1 << 9,
    TspMacro = 1 << 10,
    TspPair = 1 << 11,
    TspRec = 1 << 12,
}

pub const TSP_EXPR: u32 = TSP_NUM | TspType::TspSym as u32 | TspType::TspPair as u32;
pub const TSP_RATIONAL: u32 = TspType::TspInt as u32 | TspType::TspRatio as u32;
pub const TSP_OP_CHARS: &str = "_+-*/\\|=^<>.:";
pub const TSP_NUM: u32 = TSP_RATIONAL | TspType::TspDec as u32;

pub struct Entry {
    pub key: String,
    pub val: Val,
}

pub type Prim = fn(&mut Tsp, &mut Rec, Val) -> Val;

pub struct Rec {
    pub size: i32,
    pub cap: i32,
    pub items: Vec<Entry>,
    pub next: Option<Box<Rec>>,
}

pub struct Tsp {
    pub file: String,
    pub filec: usize,
    pub none: Val,
    pub nil: Val,
    pub t: Val,
    pub env: Rec,
    pub strs: Rec,
    pub syms: Rec,
    pub libh: Vec<*mut std::ffi::c_void>,
    pub libhc: usize,
}

pub struct Val {
    pub t: TspType,
    pub v: ValUnion,
}

pub enum ValUnion {
    S(String),
    N { num: f64, den: f64 },
    Pr { name: String, pr: Prim },
    F { name: String, args: Box<Val>, body: Box<Val>, env: Rec },
    P { car: Box<Val>, cdr: Box<Val> },
    R(Rec),
}

impl Clone for Entry {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            val: self.val.clone(),
        }
    }
}

impl Clone for Rec {
    fn clone(&self) -> Self {
        Self {
            size: self.size,
            cap: self.cap,
            items: self.items.clone(),
            next: self.next.clone(),
        }
    }
}

impl Clone for Val {
    fn clone(&self) -> Self {
        Self {
            t: self.t,
            v: self.v.clone(),
        }
    }
}

impl Clone for ValUnion {
    fn clone(&self) -> Self {
        match self {
            Self::S(s) => Self::S(s.clone()),
            Self::N { num, den } => Self::N {
                num: *num,
                den: *den,
            },
            Self::Pr { name, pr } => Self::Pr {
                name: name.clone(),
                pr: *pr,
            },
            Self::F { name, args, body, env } => Self::F {
                name: name.clone(),
                args: Box::new((**args).clone()),
                body: Box::new((**body).clone()),
                env: env.clone(),
            },
            Self::P { car, cdr } => Self::P {
                car: Box::new((**car).clone()),
                cdr: Box::new((**cdr).clone()),
            },
            Self::R(rec) => Self::R(rec.clone()),
        }
    }
}

fn dummy_prim(_st: &mut Tsp, _env: &mut Rec, _args: Val) -> Val {
    mk_val(TspType::TspNone)
}

fn type_bits(t: TspType) -> u32 {
    t as u32
}

pub(crate) fn type_matches(t: TspType, mask: u32) -> bool {
    type_bits(t) & mask != 0
}

pub(crate) fn val_is_nil(v: &Val) -> bool {
    v.t == TspType::TspNil
}

pub(crate) fn val_is_pair(v: &Val) -> bool {
    v.t == TspType::TspPair
}

pub(crate) fn val_num(v: &Val) -> f64 {
    match &v.v {
        ValUnion::N { num, .. } => *num,
        _ => 0.0,
    }
}

pub(crate) fn val_den(v: &Val) -> f64 {
    match &v.v {
        ValUnion::N { den, .. } => *den,
        _ => 1.0,
    }
}

pub(crate) fn val_str(v: &Val) -> Option<&str> {
    match &v.v {
        ValUnion::S(s) => Some(s.as_str()),
        _ => None,
    }
}

pub(crate) fn pair_car(v: &Val) -> &Val {
    match &v.v {
        ValUnion::P { car, .. } => car.as_ref(),
        _ => nil_sentinel(),
    }
}

pub(crate) fn pair_cdr(v: &Val) -> &Val {
    match &v.v {
        ValUnion::P { cdr, .. } => cdr.as_ref(),
        _ => nil_sentinel(),
    }
}

fn current_char(st: &Tsp) -> Option<char> {
    st.file.get(st.filec..)?.chars().next()
}

fn char_at_offset(st: &Tsp, offset: isize) -> Option<char> {
    let pos = st.filec.checked_add_signed(offset)?;
    st.file.get(pos..)?.chars().next()
}

fn advance_char(st: &mut Tsp) {
    if let Some(ch) = current_char(st) {
        st.filec += ch.len_utf8();
    }
}

fn advance_bytes(st: &mut Tsp, n: usize) {
    st.filec = st.filec.saturating_add(n);
}

fn slice_from_cursor(st: &Tsp) -> &str {
    st.file.get(st.filec..).unwrap_or("")
}

pub(crate) fn list_to_vec(v: &Val) -> Vec<Val> {
    let mut cur = v;
    let mut out = Vec::new();
    while cur.t == TspType::TspPair {
        out.push(pair_car(cur).clone());
        cur = pair_cdr(cur);
    }
    if !val_is_nil(cur) {
        out.push(cur.clone());
    }
    out
}

pub(crate) fn nth_arg(v: &Val, idx: usize) -> Option<Val> {
    let mut cur = v;
    let mut i = 0usize;
    while cur.t == TspType::TspPair {
        if i == idx {
            return Some(pair_car(cur).clone());
        }
        i += 1;
        cur = pair_cdr(cur);
    }
    None
}

pub(crate) fn render_val(v: &Val) -> String {
    match v.t {
        TspType::TspNone => "Void".to_string(),
        TspType::TspNil => "Nil".to_string(),
        TspType::TspInt => format!("{}", val_num(v) as i32),
        TspType::TspDec => {
            let mut s = format!("{:.15}", val_num(v));
            while s.contains('.') && s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.push('0');
            }
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                s.push_str(".0");
            }
            s
        }
        TspType::TspRatio => format!("{}/{}", val_num(v) as i32, val_den(v) as i32),
        TspType::TspStr | TspType::TspSym => val_str(v).unwrap_or_default().to_string(),
        TspType::TspFunc | TspType::TspMacro => {
            if let ValUnion::F { name, .. } = &v.v {
                let kind = if v.t == TspType::TspFunc { "function" } else { "macro" };
                if name.is_empty() {
                    format!("#<{}>", kind)
                } else {
                    format!("#<{}:{}>", kind, name)
                }
            } else {
                "#<function>".to_string()
            }
        }
        TspType::TspPrim => {
            if let ValUnion::Pr { name, .. } = &v.v {
                format!("#<primitive:{}>", name)
            } else {
                "#<primitive>".to_string()
            }
        }
        TspType::TspForm => {
            if let ValUnion::Pr { name, .. } = &v.v {
                format!("#<form:{}>", name)
            } else {
                "#<form>".to_string()
            }
        }
        TspType::TspRec => {
            if let ValUnion::R(rec) = &v.v {
                let mut parts = Vec::new();
                let mut printed = 0usize;
                let mut cur = Some(rec);
                while let Some(r) = cur {
                    for entry in &r.items {
                        if !entry.key.is_empty() {
                            parts.push(format!("{}: {}", entry.key, render_val(&entry.val)));
                            printed += 1;
                            if printed == TSP_REC_MAX_PRINT {
                                parts.push("...".to_string());
                                cur = None;
                                break;
                            }
                        }
                    }
                    if let Some(next) = &r.next {
                        cur = Some(next.as_ref());
                    } else {
                        cur = None;
                    }
                }
                if parts.is_empty() {
                    "{ }".to_string()
                } else {
                    format!("{{ {} }}", parts.join(" "))
                }
            } else {
                "{ }".to_string()
            }
        }
        TspType::TspPair => {
            let mut out = String::from("(");
            let mut cur = v;
            let mut first = true;
            loop {
                if cur.t != TspType::TspPair {
                    if !val_is_nil(cur) {
                        if !first {
                            out.push_str(" . ");
                        }
                        out.push_str(&render_val(cur));
                    }
                    break;
                }
                if !first {
                    out.push(' ');
                }
                out.push_str(&render_val(pair_car(cur)));
                let next = pair_cdr(cur);
                if val_is_nil(next) {
                    break;
                }
                if next.t != TspType::TspPair {
                    out.push_str(" . ");
                    out.push_str(&render_val(next));
                    break;
                }
                cur = next;
                first = false;
            }
            out.push(')');
            out
        }
    }
}

pub(crate) fn warn_none(st: &Tsp, msg: &str) -> Val {
    eprintln!("; tisp: error: {}", msg);
    st.none.clone()
}

pub(crate) fn expect_len(st: &Tsp, args: &Val, name: &str, nargs: i32) -> bool {
    let len = tsp_lstlen(args);
    if nargs > -1 && len != nargs {
        eprintln!(
            "; tisp: error: {}: expected {} argument{}, received {}",
            name,
            nargs,
            if nargs == 1 { "" } else { "s" },
            len
        );
        return false;
    }
    true
}

pub(crate) fn expect_min_len(st: &Tsp, args: &Val, name: &str, nargs: i32) -> bool {
    let len = tsp_lstlen(args);
    if len < nargs {
        eprintln!(
            "; tisp: error: {}: expected at least {} argument{}, received {}",
            name,
            nargs,
            if nargs == 1 { "" } else { "s" },
            len
        );
        return false;
    }
    true
}

pub(crate) fn expect_max_len(st: &Tsp, args: &Val, name: &str, nargs: i32) -> bool {
    let len = tsp_lstlen(args);
    if len > nargs {
        eprintln!(
            "; tisp: error: {}: expected at no more than {} argument{}, received {}",
            name,
            nargs,
            if nargs == 1 { "" } else { "s" },
            len
        );
        return false;
    }
    true
}

pub(crate) fn expect_type(st: &Tsp, arg: &Val, name: &str, mask: u32) -> bool {
    if !type_matches(arg.t, mask) {
        eprintln!(
            "; tisp: error: {}: expected {}, received {}",
            name,
            if mask == TSP_EXPR {
                "Expr"
            } else if mask == TSP_RATIONAL {
                "Rational"
            } else if mask == TSP_NUM {
                "Num"
            } else {
                tsp_type_str(mask_to_type(mask))
            },
            tsp_type_str(arg.t)
        );
        return false;
    }
    true
}

fn mask_to_type(mask: u32) -> TspType {
    match mask {
        x if x == TspType::TspNone as u32 => TspType::TspNone,
        x if x == TspType::TspNil as u32 => TspType::TspNil,
        x if x == TspType::TspInt as u32 => TspType::TspInt,
        x if x == TspType::TspDec as u32 => TspType::TspDec,
        x if x == TspType::TspRatio as u32 => TspType::TspRatio,
        x if x == TspType::TspStr as u32 => TspType::TspStr,
        x if x == TspType::TspSym as u32 => TspType::TspSym,
        x if x == TspType::TspPrim as u32 => TspType::TspPrim,
        x if x == TspType::TspForm as u32 => TspType::TspForm,
        x if x == TspType::TspFunc as u32 => TspType::TspFunc,
        x if x == TspType::TspMacro as u32 => TspType::TspMacro,
        x if x == TspType::TspPair as u32 => TspType::TspPair,
        _ => TspType::TspRec,
    }
}

pub fn rec_add(rec: &mut Rec, key: &str, val: Val) {
    let idx = find_entry_index(rec, key);
    let is_new = rec.items[idx].key.is_empty();
    rec.items[idx].val = val;
    if is_new {
        rec.items[idx].key = key.to_string();
        rec.size += 1;
        if rec.size > rec.cap / TSP_REC_FACTOR as i32 {
            rec_grow(rec);
        }
    }
}

pub fn mk_rat(num: i32, den: i32) -> Option<Val> {
    if den == 0 {
        return None;
    }
    let mut n = num;
    let mut d = den;
    frac_reduce(&mut n, &mut d);
    if d < 0 {
        d = d.abs();
        n = -n;
    }
    if d == 1 {
        return Some(mk_int(n));
    }
    Some(Val {
        t: TspType::TspRatio,
        v: ValUnion::N {
            num: n as f64,
            den: d as f64,
        },
    })
}

pub fn mk_val(t: TspType) -> Val {
    let v = match t {
        TspType::TspNone | TspType::TspNil | TspType::TspInt | TspType::TspDec | TspType::TspRatio => {
            ValUnion::N { num: 0.0, den: 1.0 }
        }
        TspType::TspStr | TspType::TspSym => ValUnion::S(String::new()),
        TspType::TspPrim | TspType::TspForm => ValUnion::Pr {
            name: String::new(),
            pr: dummy_prim,
        },
        TspType::TspFunc | TspType::TspMacro => ValUnion::F {
            name: String::new(),
            args: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            }),
            body: Box::new(Val {
                t: TspType::TspNil,
                v: ValUnion::N { num: 0.0, den: 1.0 },
            }),
            env: rec_new(1, None),
        },
        TspType::TspPair => ValUnion::P {
            car: Box::new(mk_val(TspType::TspNil)),
            cdr: Box::new(mk_val(TspType::TspNil)),
        },
        TspType::TspRec => ValUnion::R(rec_new(1, None)),
    };
    Val { t, v }
}

pub fn tsp_lstlen(v: &Val) -> i32 {
    let mut len = 0;
    let mut cur = v;
    while cur.t == TspType::TspPair {
        len += 1;
        cur = pair_cdr(cur);
    }
    if val_is_nil(cur) { len } else { -(len + 1) }
}

pub fn tisp_env_init(cap: usize) -> Tsp {
    let strs = rec_new(cap, None);
    let syms = rec_new(cap, None);
    let mut st = Tsp {
        file: String::new(),
        filec: 0,
        none: mk_val(TspType::TspNone),
        nil: mk_val(TspType::TspNil),
        t: Val {
            t: TspType::TspSym,
            v: ValUnion::S("True".to_string()),
        },
        env: rec_new(cap, None),
        strs,
        syms,
        libh: Vec::new(),
        libhc: 0,
    };
    let true_val = st.t.clone();
    let nil_val = st.nil.clone();
    let none_val = st.none.clone();
    tisp_env_add(&mut st, "True", true_val);
    tisp_env_add(&mut st, "Nil", nil_val.clone());
    tisp_env_add(&mut st, "Void", none_val);
    tisp_env_add(&mut st, "bt", nil_val);
    if let Some(version) = mk_str(&mut st, "0.1") {
        tisp_env_add(&mut st, "version", version);
    }
    st
}

pub fn tib_env_os(st: &mut Tsp) {
    crate::os::tib_env_os(st);
}

pub fn read_num(st: &mut Tsp) -> Val {
    let sign = read_sign(st);
    let whole = read_int(st);
    match current_char(st) {
        Some('/') => {
            advance_char(st);
            if !isnum(slice_from_cursor(st)) {
                return st.none.clone();
            }
            mk_rat(sign * whole, read_sign(st) * read_int(st)).unwrap_or_else(|| st.none.clone())
        }
        Some('.') => {
            advance_char(st);
            let old = st.filec;
            let mut d = read_int(st) as f64;
            let mut size = st.filec.saturating_sub(old);
            while size > 0 {
                d /= 10.0;
                size -= 1;
            }
            read_sci(st, sign as f64 * (whole as f64 + d), 0).unwrap_or_else(|| st.none.clone())
        }
        _ => read_sci(st, (sign * whole) as f64, 1).unwrap_or_else(|| st.none.clone()),
    }
}

pub fn entry_get<'a>(rec: &'a Rec, key: &'a str) -> Option<&'a Entry> {
    let mut i = hash(key) as usize % rec.cap.max(1) as usize;
    loop {
        let entry = &rec.items[i];
        if entry.key.is_empty() {
            return None;
        }
        if entry.key == key {
            return Some(entry);
        }
        i += 1;
        if i == rec.cap as usize {
            i = 0;
        }
    }
}

pub fn tib_env_string(st: &mut Tsp) {
    crate::string::tib_env_string(st);
}

pub fn prepend_bt(st: &mut Tsp, env: &mut Rec, f: Val) {
    let name = if let ValUnion::F { name, .. } = &f.v {
        name.clone()
    } else {
        String::new()
    };
    if name.is_empty() {
        return;
    }
    let base = deepest_rec_mut(env);
    if let Some(existing) = entry_get(base, "bt") {
        if existing.val.t == TspType::TspPair {
            if let Some(sym) = val_str(pair_car(&existing.val)) {
                if sym == name {
                    return;
                }
            }
        }
    }
    if let Some(bt) = rec_get(base, "bt") {
        let sym = mk_sym(st, &name).unwrap_or_else(|| st.none.clone());
        if let Some(pair) = mk_pair(sym, bt) {
            rec_add(base, "bt", pair);
        }
    }
}

pub fn rec_get(rec: &Rec, key: &str) -> Option<Val> {
    let mut cur = Some(rec);
    while let Some(r) = cur {
        if let Some(entry) = entry_get(r, key) {
            return Some(entry.val.clone());
        }
        cur = r.next.as_deref();
    }
    None
}

pub fn tisp_env_add(st: &mut Tsp, key: &str, v: Val) {
    rec_add(&mut st.env, key, v);
}

pub fn mk_pair(a: Val, b: Val) -> Option<Val> {
    Some(Val {
        t: TspType::TspPair,
        v: ValUnion::P {
            car: Box::new(a),
            cdr: Box::new(b),
        },
    })
}

pub fn read_pair(st: &mut Tsp, endchar: char) -> Option<Val> {
    let mut values = Vec::new();
    let mut improper: Option<Val> = None;
    let skipnl = endchar != '\n';
    skip_ws(st, if skipnl { 1 } else { 0 });
    while let Some(ch) = current_char(st) {
        if ch == endchar {
            break;
        }
        let v = tisp_read(st)?;
        if v.t == TspType::TspSym && val_str(&v) == Some(".") {
            skip_ws(st, if skipnl { 1 } else { 0 });
            improper = Some(tisp_read(st)?);
            break;
        }
        values.push(v);
        skip_ws(st, if skipnl { 1 } else { 0 });
    }
    skip_ws(st, if skipnl { 1 } else { 0 });
    if skipnl && current_char(st) != Some(endchar) {
        return None;
    }
    if current_char(st) == Some(endchar) {
        advance_char(st);
    }
    let mut tail = improper.unwrap_or_else(|| st.nil.clone());
    for v in values.into_iter().rev() {
        tail = mk_pair(v, tail)?;
    }
    Some(tail)
}

pub fn tisp_read_sexpr(st: &mut Tsp) -> Option<Val> {
    let prefix = [
        ("'", "quote"),
        ("`", "quasiquote"),
        (",@", "unquote-splice"),
        (",", "unquote"),
        ("@", "Func"),
        ("f\"", "strformat"),
    ];
    skip_ws(st, 1);
    if slice_from_cursor(st).is_empty() {
        return Some(st.none.clone());
    }
    if isnum(slice_from_cursor(st)) {
        return Some(read_num(st));
    }
    if current_char(st) == Some('"') {
        return read_str(st, mk_str_val);
    }
    if current_char(st) == Some('~') {
        return read_str(st, mk_sym_val);
    }
    for (pfx, name) in prefix {
        if slice_from_cursor(st).starts_with(pfx) {
            let bump = pfx.len() - usize::from(pfx.ends_with('"'));
            advance_bytes(st, bump);
            let v = tisp_read(st)?;
            let sym = mk_sym(st, name)?;
            return mk_list(st, 2, vec![sym, v]);
        }
    }
    if let Some(ch) = current_char(st) {
        if is_op(ch) {
            return read_sym(st, is_op);
        }
        if is_sym(ch) {
            return read_sym(st, is_sym);
        }
        if ch == '(' {
            advance_char(st);
            return read_pair(st, ')');
        }
        if ch == '[' {
            advance_char(st);
            return mk_pair(mk_sym(st, "list")?, read_pair(st, ']')?);
        }
        if ch == '{' {
            advance_char(st);
            let v = read_pair(st, '}')?;
            return mk_pair(mk_sym(st, "Rec")?, v);
        }
    }
    None
}

pub fn is_sym(c: char) -> bool {
    c.is_ascii_alphanumeric() || TSP_SYM_CHARS.contains(c)
}

pub fn mk_sym(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.syms, s) {
        return Some(v);
    }
    let ret = Val {
        t: TspType::TspSym,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.syms, s, ret.clone());
    Some(ret)
}

pub fn frac_reduce(num: &mut i32, den: &mut i32) {
    let mut a = num.abs();
    let mut b = den.abs();
    if b == 0 {
        return;
    }
    let mut c = a % b;
    while c > 0 {
        a = b;
        b = c;
        c = a % b;
    }
    *num /= b;
    *den /= b;
}

pub fn tisp_read_line(st: &mut Tsp, level: i32) -> Option<Val> {
    let mut ret = read_pair(st, '\n')?;
    if ret.t != TspType::TspPair {
        ret = mk_pair(ret, st.nil.clone())?;
    }
    let mut elems = list_to_vec(&ret);
    while let Some(slice) = st.file.get(st.filec..) {
        let indent = slice.chars().take_while(|ch| *ch == '\t' || *ch == ' ').count() as i32;
        if indent <= level {
            break;
        }
        advance_bytes(st, indent as usize);
        elems.push(tisp_read_line(st, indent)?);
    }
    if elems.len() == 1 {
        return elems.into_iter().next();
    }
    mk_list(st, elems.len() as i32, elems)
}

pub fn mk_prim(t: TspType, pr: Prim, name: &str) -> Option<Val> {
    Some(Val {
        t,
        v: ValUnion::Pr {
            name: name.to_string(),
            pr,
        },
    })
}

pub fn isnum(str: &str) -> bool {
    let mut chars = str.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => true,
        Some('.') => chars.next().is_some_and(|c| c.is_ascii_digit()),
        Some('-') | Some('+') => {
            let next = chars.next();
            next.is_some_and(|c| c.is_ascii_digit() || c == '.')
        }
        _ => false,
    }
}

pub fn tsp_type_str(t: TspType) -> &'static str {
    match t {
        TspType::TspNone => "Void",
        TspType::TspNil => "Nil",
        TspType::TspInt => "Int",
        TspType::TspDec => "Dec",
        TspType::TspRatio => "Ratio",
        TspType::TspStr => "Str",
        TspType::TspSym => "Sym",
        TspType::TspPrim => "Prim",
        TspType::TspForm => "Form",
        TspType::TspFunc => "Func",
        TspType::TspMacro => "Macro",
        TspType::TspPair => "Pair",
        TspType::TspRec => "Rec",
    }
}

pub fn mk_str(st: &mut Tsp, s: &str) -> Option<Val> {
    if let Some(v) = rec_get(&st.strs, s) {
        return Some(v);
    }
    let ret = Val {
        t: TspType::TspStr,
        v: ValUnion::S(s.to_string()),
    };
    rec_add(&mut st.strs, s, ret.clone());
    Some(ret)
}

pub fn is_op(c: char) -> bool {
    TSP_OP_CHARS.contains(c)
}

pub fn esc_str(s: &str, len: i32, do_esc: i32) -> String {
    let mut ret = String::new();
    let mut chars = s.chars();
    let mut remaining = len.max(0) as usize;
    while remaining > 0 {
        if let Some(ch) = chars.next() {
            if ch == '\\' && do_esc != 0 {
                if let Some(next) = chars.next() {
                    ret.push(esc_char(next));
                }
            } else {
                ret.push(ch);
            }
        } else {
            break;
        }
        remaining -= 1;
    }
    ret
}

pub fn tib_env_core(st: &mut Tsp) {
    crate::core::tib_env_core(st);
}

pub fn skip_ws(st: &mut Tsp, skipnl: i32) {
    let allowed = if skipnl != 0 { " \t\n\r" } else { " \t" };
    loop {
        while let Some(ch) = current_char(st) {
            if allowed.contains(ch) {
                advance_char(st);
            } else {
                break;
            }
        }
        if current_char(st) != Some(';') {
            break;
        }
        while current_char(st).is_some() && current_char(st) != Some('\n') {
            advance_char(st);
        }
        if skipnl == 0 && current_char(st) == Some('\n') {
            break;
        }
    }
}

pub fn rec_extend(rec: &mut Rec, args: Val, vals: Val) -> Rec {
    let argnum = TSP_REC_FACTOR as i32 * tsp_lstlen(&args);
    let cap = if argnum > 0 { argnum } else { -argnum + 1 } as usize;
    let mut ret = rec_new(cap, Some(Box::new(rec.clone())));
    let mut a_cur = args;
    let mut v_cur = vals;
    loop {
        if a_cur.t == TspType::TspNil {
            break;
        }
        let (arg, val, done) = if a_cur.t == TspType::TspPair {
            let arg = pair_car(&a_cur).clone();
            let val = if v_cur.t == TspType::TspPair {
                pair_car(&v_cur).clone()
            } else {
                v_cur.clone()
            };
            let done = pair_cdr(&a_cur).t != TspType::TspPair && pair_cdr(&a_cur).t != TspType::TspNil;
            a_cur = pair_cdr(&a_cur).clone();
            v_cur = if v_cur.t == TspType::TspPair {
                pair_cdr(&v_cur).clone()
            } else {
                mk_val(TspType::TspNil)
            };
            (arg, val, done)
        } else {
            (a_cur.clone(), v_cur.clone(), true)
        };
        if arg.t == TspType::TspSym {
            if let Some(name) = val_str(&arg) {
                rec_add(&mut ret, name, val);
            }
        }
        if done {
            break;
        }
    }
    ret
}

pub fn hash(key: &str) -> u32 {
    let mut h = 0u32;
    for b in key.bytes() {
        if h == u32::MAX {
            break;
        }
        h = h.wrapping_mul(33).wrapping_add(b as u32);
    }
    h
}

pub fn mk_rec(st: &mut Tsp, env: Rec, assoc: Val) -> Option<Val> {
    if assoc.t == TspType::TspNil {
        return Some(Val {
            t: TspType::TspRec,
            v: ValUnion::R(env),
        });
    }
    let cap = TSP_REC_FACTOR as i32 * tsp_lstlen(&assoc);
    let mut record = rec_new(if cap > 0 { cap as usize } else { (-cap + 1) as usize }, None);
    let mut eval_env = rec_new(4, Some(Box::new(env.clone())));
    rec_add(&mut eval_env, "this", mk_val(TspType::TspNone));
    let mut cur = assoc;
    while cur.t == TspType::TspPair {
        let item = pair_car(&cur).clone();
        if item.t == TspType::TspPair {
            let key = pair_car(&item).clone();
            if type_matches(key.t, TspType::TspSym as u32 | TspType::TspStr as u32) {
                let val_expr = pair_car(pair_cdr(&item)).clone();
                let value = eval_in_env(st, &mut eval_env, val_expr)?;
                rec_add(&mut record, val_str(&key).unwrap_or_default(), value);
            } else {
                return None;
            }
        } else if item.t == TspType::TspSym {
            let name = val_str(&item).unwrap_or_default().to_string();
            let value = eval_in_env(st, &mut eval_env, item.clone())?;
            rec_add(&mut record, &name, value);
        } else {
            return None;
        }
        cur = pair_cdr(&cur).clone();
    }
    Some(Val {
        t: TspType::TspRec,
        v: ValUnion::R(record),
    })
}

pub fn tisp_read(st: &mut Tsp) -> Option<Val> {
    let mut v = tisp_read_sexpr(st)?;
    while matches!(current_char(st), Some('(') | Some(':') | Some('>') | Some('{')) {
        v = tisp_read_sugar(st, v)?;
    }
    Some(v)
}

pub fn mk_int(i: i32) -> Val {
    Val {
        t: TspType::TspInt,
        v: ValUnion::N {
            num: i as f64,
            den: 1.0,
        },
    }
}

pub fn tib_env_math(st: &mut Tsp) {
    crate::math::tib_env_math(st);
}

pub fn tisp_eval_list(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut evaluated = Vec::new();
    let mut cur = v;
    while cur.t == TspType::TspPair {
        evaluated.push(eval_in_env(st, env, pair_car(&cur).clone())?);
        cur = pair_cdr(&cur).clone();
    }
    let mut tail = if val_is_nil(&cur) {
        st.nil.clone()
    } else {
        eval_in_env(st, env, cur)?
    };
    for v in evaluated.into_iter().rev() {
        tail = mk_pair(v, tail)?;
    }
    Some(tail)
}

pub fn read_sci(st: &mut Tsp, val: f64, isint: i32) -> Option<Val> {
    let mut out = val;
    if current_char(st).is_some_and(|c| c.eq_ignore_ascii_case(&'e')) {
        advance_char(st);
        let sign = if read_sign(st) == 1 { 10.0 } else { 0.1 };
        let expo = read_int(st);
        for _ in 0..expo {
            out *= sign;
        }
    }
    if isint != 0 {
        Some(mk_int(out as i32))
    } else {
        mk_dec(out)
    }
}

pub fn read_int(st: &mut Tsp) -> i32 {
    let mut ret = 0i32;
    while let Some(ch) = current_char(st) {
        if !ch.is_ascii_digit() {
            break;
        }
        ret = ret * 10 + (ch as i32 - '0' as i32);
        advance_char(st);
    }
    ret
}

pub fn rec_new(cap: usize, next: Option<Box<Rec>>) -> Rec {
    let cap = cap.max(1);
    Rec {
        size: 0,
        cap: cap as i32,
        items: (0..cap)
            .map(|_| Entry {
                key: String::new(),
                val: mk_int(0),
            })
            .collect(),
        next,
    }
}

pub fn read_str(st: &mut Tsp, mk_fn: fn(&mut Tsp, &str) -> Val) -> Option<Val> {
    let start = current_char(st)?;
    advance_char(st);
    let s = slice_from_cursor(st).to_string();
    let endchar = if start == '"' { '"' } else { '~' };
    let mut len = 0usize;
    let mut prev = '\0';
    for ch in s.chars() {
        if ch == endchar && prev != '\\' {
            break;
        }
        len += ch.len_utf8();
        if ch == '\\' && prev != '\\' {
            prev = ch;
            continue;
        }
        prev = ch;
    }
    let raw = s[..len].to_string();
    advance_bytes(st, len);
    if current_char(st) == Some(endchar) {
        advance_char(st);
    } else {
        return None;
    }
    let do_esc = i32::from(start == '"');
    Some(mk_fn(st, &esc_str(&raw, raw.chars().count() as i32, do_esc)))
}

pub fn read_sym(st: &mut Tsp, is_char: fn(char) -> bool) -> Option<Val> {
    let start = st.filec;
    while let Some(ch) = current_char(st) {
        if !is_char(ch) {
            break;
        }
        advance_char(st);
    }
    let text = st.file[start..st.filec].to_string();
    mk_sym(st, &text)
}

pub fn mk_dec(d: f64) -> Option<Val> {
    Some(Val {
        t: TspType::TspDec,
        v: ValUnion::N { num: d, den: 1.0 },
    })
}

pub fn tisp_eval_body(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    let mut ret = st.none.clone();
    let mut cur = v;
    while cur.t == TspType::TspPair {
        ret = eval_in_env(st, env, pair_car(&cur).clone())?;
        cur = pair_cdr(&cur).clone();
    }
    Some(ret)
}

pub fn tib_env_io(st: &mut Tsp) {
    crate::io::tib_env_io(st);
}

pub fn tisp_read_sugar(st: &mut Tsp, v: Val) -> Option<Val> {
    if current_char(st) == Some('(') {
        advance_char(st);
        let lst = read_pair(st, ')')?;
        return mk_pair(v, lst);
    }
    if current_char(st) == Some('{') {
        advance_char(st);
        let lst = read_pair(st, '}')?;
        let recmerge = mk_sym(st, "recmerge")?;
        let rec_sym = mk_sym(st, "Rec")?;
        let rec_call = mk_pair(rec_sym, lst)?;
        return mk_list(st, 3, vec![recmerge, v, rec_call]);
    }
    if current_char(st) == Some(':') {
        advance_char(st);
        match current_char(st) {
            Some('(') => {
                advance_char(st);
                let w = read_pair(st, ')')?;
                return mk_pair(mk_sym(st, "map")?, mk_pair(v, w)?);
            }
            Some(':') => {
                advance_char(st);
                let w = read_sym(st, is_sym)?;
                let quote = mk_sym(st, "quote")?;
                let quoted = mk_list(st, 2, vec![quote, w])?;
                return mk_list(st, 2, vec![v, quoted]);
            }
            _ => {
                skip_ws(st, 1);
                let w = tisp_read(st)?;
                return mk_list(st, 2, vec![v, w]);
            }
        }
    }
    if current_char(st) == Some('>') && char_at_offset(st, 1) == Some('>') {
        advance_char(st);
        advance_char(st);
        let w = tisp_read(st)?;
        if w.t != TspType::TspPair {
            return None;
        }
        return mk_pair(pair_car(&w).clone(), mk_pair(v, pair_cdr(&w).clone())?);
    }
    Some(v)
}

pub fn tisp_env_lib(st: &mut Tsp, lib: &str) {
    let old_file = st.file.clone();
    let old_filec = st.filec;
    st.file = lib.to_string();
    st.filec = 0;
    skip_ws(st, 1);
    if let Some(v) = tisp_read(st) {
        let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
        let _ = tisp_eval_body(st, &mut env, v);
        st.env = env;
    }
    st.file = old_file;
    st.filec = old_filec;
}

pub fn mk_list(st: &mut Tsp, n: i32, args: Vec<Val>) -> Option<Val> {
    if n <= 0 || args.is_empty() {
        return Some(st.nil.clone());
    }
    let mut tail = st.nil.clone();
    for arg in args.into_iter().take(n as usize).rev() {
        tail = mk_pair(arg, tail)?;
    }
    Some(tail)
}

pub fn vals_eq(a: &Val, b: &Val) -> bool {
    if type_matches(a.t, TSP_NUM) && type_matches(b.t, TSP_NUM) {
        return val_num(a) == val_num(b) && val_den(a) == val_den(b);
    }
    if a.t != b.t {
        return false;
    }
    match (&a.v, &b.v) {
        (ValUnion::P { car: ac, cdr: ad }, ValUnion::P { car: bc, cdr: bd }) => {
            vals_eq(ac, bc) && vals_eq(ad, bd)
        }
        (
            ValUnion::F {
                args: aa,
                body: ab,
                ..
            },
            ValUnion::F {
                args: ba,
                body: bb,
                ..
            },
        ) => vals_eq(aa, ba) && vals_eq(ab, bb),
        (ValUnion::S(as_), ValUnion::S(bs)) => as_ == bs,
        (ValUnion::N { .. }, ValUnion::N { .. }) => true,
        (ValUnion::Pr { name: an, .. }, ValUnion::Pr { name: bn, .. }) => an == bn,
        (ValUnion::R(ar), ValUnion::R(br)) => std::ptr::eq(ar, br),
        _ => false,
    }
}

pub fn esc_char(c: char) -> char {
    match c {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '\n' => ' ',
        '\\' | '"' => c,
        _ => c,
    }
}

pub fn read_sign(st: &mut Tsp) -> i32 {
    match current_char(st) {
        Some('-') => {
            advance_char(st);
            -1
        }
        Some('+') => {
            advance_char(st);
            1
        }
        _ => 1,
    }
}

pub fn tisp_print(f: &mut std::fs::File, v: &Val) {
    let _ = std::io::Write::write_all(f, render_val(v).as_bytes());
}

pub fn eval_proc(st: &mut Tsp, env: &mut Rec, f: Val, args: Val) -> Option<Val> {
    match f.v {
        ValUnion::Pr { pr, .. } => {
            let evaled = if f.t == TspType::TspPrim {
                tisp_eval_list(st, env, args)?
            } else {
                args
            };
            Some(pr(st, env, evaled))
        }
        ValUnion::F {
            args: fn_args,
            body,
            env: fn_env,
            ..
        } => {
            let incoming = if f.t == TspType::TspFunc {
                tisp_eval_list(st, env, args)?
            } else {
                args
            };
            let mut fenv = rec_extend(&mut fn_env.clone(), (*fn_args).clone(), incoming);
            let mut ret = tisp_eval_body(st, &mut fenv, (*body).clone())?;
            if f.t == TspType::TspMacro {
                ret = eval_in_env(st, env, ret)?;
            }
            Some(ret)
        }
        ValUnion::R(rec) => {
            let evaled = tisp_eval_list(st, env, args)?;
            if !expect_len(st, &evaled, "record", 1) {
                return Some(st.none.clone());
            }
            let key = nth_arg(&evaled, 0)?;
            if !expect_type(st, &key, "record", TspType::TspSym as u32) {
                return Some(st.none.clone());
            }
            let name = val_str(&key).unwrap_or_default();
            rec_get(&rec, name).or_else(|| rec_get(&rec, "else")).or_else(|| Some(st.none.clone()))
        }
        _ => None,
    }
}

pub fn tisp_eval(st: &mut Tsp, v: Val) -> Option<Val> {
    let mut env = std::mem::replace(&mut st.env, rec_new(1, None));
    let ret = eval_in_env(st, &mut env, v);
    st.env = env;
    ret
}

pub fn mk_func(t: TspType, name: &str, args: Val, body: Val, env: Rec) -> Option<Val> {
    Some(Val {
        t,
        v: ValUnion::F {
            name: name.to_string(),
            args: Box::new(args),
            body: Box::new(body),
            env,
        },
    })
}

pub fn rec_grow(rec: &mut Rec) {
    let old_items = rec.items.clone();
    rec.cap *= TSP_REC_FACTOR as i32;
    rec.items = (0..rec.cap)
        .map(|_| Entry {
            key: String::new(),
            val: mk_int(0),
        })
        .collect();
    rec.size = 0;
    for entry in old_items {
        if !entry.key.is_empty() {
            rec_add(rec, &entry.key, entry.val);
        }
    }
}

pub(crate) fn eval_in_env(st: &mut Tsp, env: &mut Rec, v: Val) -> Option<Val> {
    match v.t {
        TspType::TspSym => rec_get(env, val_str(&v).unwrap_or_default()),
        TspType::TspPair => {
            let f = eval_in_env(st, env, pair_car(&v).clone())?;
            eval_proc(st, env, f, pair_cdr(&v).clone())
        }
        _ => Some(v),
    }
}

fn deepest_rec_mut(rec: &mut Rec) -> &mut Rec {
    match rec.next {
        Some(ref mut next) => deepest_rec_mut(next),
        None => rec,
    }
}

fn find_entry_index(rec: &Rec, key: &str) -> usize {
    let mut i = hash(key) as usize % rec.cap.max(1) as usize;
    loop {
        let entry = &rec.items[i];
        if entry.key.is_empty() || entry.key == key {
            return i;
        }
        i += 1;
        if i == rec.cap as usize {
            i = 0;
        }
    }
}

fn mk_str_val(st: &mut Tsp, s: &str) -> Val {
    mk_str(st, s).unwrap_or_else(|| mk_val(TspType::TspNone))
}

fn mk_sym_val(st: &mut Tsp, s: &str) -> Val {
    mk_sym(st, s).unwrap_or_else(|| mk_val(TspType::TspNone))
}

fn nil_sentinel() -> &'static Val {
    static NIL: std::sync::OnceLock<Val> = std::sync::OnceLock::new();
    NIL.get_or_init(|| Val {
        t: TspType::TspNil,
        v: ValUnion::N { num: 0.0, den: 1.0 },
    })
}
