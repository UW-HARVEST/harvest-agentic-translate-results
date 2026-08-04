use std::f32::consts::PI;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}

impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            MapperSignalValue::F(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MapperSignalValue::I32(v) => Some(*v),
            _ => None,
        }
    }
}

const STACK_SIZE: usize = 256;
const TRACING: bool = false;

#[allow(unused_macros)]
macro_rules! trace {
    ($($arg:tt)*) => {
        if TRACING {
            println!("-- {}", format!($($arg)*));
        }
    };
}
#[allow(unused_macros)]
macro_rules! die_unless {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            println!("-- {}", format!($($arg)*));
            assert!($cond);
        }
    };
}

fn minf(x: f32, y: f32) -> f32 {
    if y < x { y } else { x }
}
fn maxf(x: f32, y: f32) -> f32 {
    if y > x { y } else { x }
}
fn pif() -> f32 {
    PI
}

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprFunc {
    Unknown = -1,
    Pow = 0,
    Sin,
    Cos,
    Tan,
    Abs,
    Sqrt,
    Log,
    Log10,
    Exp,
    Floor,
    Round,
    Ceil,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Logb,
    Exp2,
    Log2,
    Hypot,
    Cbrt,
    Trunc,
    Min,
    Max,
    Pi,
    NFuncs,
}

#[derive(Debug, Clone, Copy)]
struct FunctionEntry {
    name: &'static str,
    arity: u32,
    func: fn(f32, f32) -> f32,
}

// Wrappers to fit into fn(f32, f32) -> f32 signature
fn fn_pow(a: f32, b: f32) -> f32 { a.powf(b) }
fn fn_sin(a: f32, _b: f32) -> f32 { a.sin() }
fn fn_cos(a: f32, _b: f32) -> f32 { a.cos() }
fn fn_tan(a: f32, _b: f32) -> f32 { a.tan() }
fn fn_abs(a: f32, _b: f32) -> f32 { a.abs() }
fn fn_sqrt(a: f32, _b: f32) -> f32 { a.sqrt() }
fn fn_log(a: f32, _b: f32) -> f32 { a.ln() }
fn fn_log10(a: f32, _b: f32) -> f32 { a.log10() }
fn fn_exp(a: f32, _b: f32) -> f32 { a.exp() }
fn fn_floor(a: f32, _b: f32) -> f32 { a.floor() }
fn fn_round(a: f32, _b: f32) -> f32 {
    // C's roundf rounds half-away-from-zero
    if a >= 0.0 { (a + 0.5).floor() } else { -((-a + 0.5).floor()) }
}
fn fn_ceil(a: f32, _b: f32) -> f32 { a.ceil() }
fn fn_asin(a: f32, _b: f32) -> f32 { a.asin() }
fn fn_acos(a: f32, _b: f32) -> f32 { a.acos() }
fn fn_atan(a: f32, _b: f32) -> f32 { a.atan() }
fn fn_atan2(a: f32, b: f32) -> f32 { a.atan2(b) }
fn fn_sinh(a: f32, _b: f32) -> f32 { a.sinh() }
fn fn_cosh(a: f32, _b: f32) -> f32 { a.cosh() }
fn fn_tanh(a: f32, _b: f32) -> f32 { a.tanh() }
fn fn_logb(a: f32, _b: f32) -> f32 {
    // C's logbf returns floor(log2(|a|))
    if a == 0.0 {
        f32::NEG_INFINITY
    } else {
        a.abs().log2().floor()
    }
}
fn fn_exp2(a: f32, _b: f32) -> f32 { a.exp2() }
fn fn_log2(a: f32, _b: f32) -> f32 { a.log2() }
fn fn_hypot(a: f32, b: f32) -> f32 { a.hypot(b) }
fn fn_cbrt(a: f32, _b: f32) -> f32 { a.cbrt() }
fn fn_trunc(a: f32, _b: f32) -> f32 { a.trunc() }
fn fn_min(a: f32, b: f32) -> f32 { minf(a, b) }
fn fn_max(a: f32, b: f32) -> f32 { maxf(a, b) }
fn fn_pi(_a: f32, _b: f32) -> f32 { pif() }

// Ordered list to preserve C's function_table order (used for index lookup)
const FUNCTION_LIST: &[(i32, &'static str, u32, fn(f32, f32) -> f32)] = &[
    (0,  "pow",   2, fn_pow),
    (1,  "sin",   1, fn_sin),
    (2,  "cos",   1, fn_cos),
    (3,  "tan",   1, fn_tan),
    (4,  "abs",   1, fn_abs),
    (5,  "sqrt",  1, fn_sqrt),
    (6,  "log",   1, fn_log),
    (7,  "log10", 1, fn_log10),
    (8,  "exp",   1, fn_exp),
    (9,  "floor", 1, fn_floor),
    (10, "round", 1, fn_round),
    (11, "ceil",  1, fn_ceil),
    (12, "asin",  1, fn_asin),
    (13, "acos",  1, fn_acos),
    (14, "atan",  1, fn_atan),
    (15, "atan2", 2, fn_atan2),
    (16, "sinh",  1, fn_sinh),
    (17, "cosh",  1, fn_cosh),
    (18, "tanh",  1, fn_tanh),
    (19, "logb",  1, fn_logb),
    (20, "exp2",  1, fn_exp2),
    (21, "log2",  1, fn_log2),
    (22, "hypot", 2, fn_hypot),
    (23, "cbrt",  1, fn_cbrt),
    (24, "trunc", 1, fn_trunc),
    (25, "min",   2, fn_min),
    (26, "max",   2, fn_max),
    (27, "pi",    0, fn_pi),
];

lazy_static::lazy_static! {
    static ref FUNCTION_TABLE: HashMap<&'static str, FunctionEntry> = {
        let mut m = HashMap::new();
        for (_idx, name, arity, func) in FUNCTION_LIST {
            m.insert(*name, FunctionEntry { name: *name, arity: *arity, func: *func });
        }
        m
    };
    // Map from function index (i32) to entry
    static ref FUNCTION_BY_INDEX: HashMap<i32, FunctionEntry> = {
        let mut m = HashMap::new();
        for (idx, name, arity, func) in FUNCTION_LIST {
            m.insert(*idx, FunctionEntry { name: *name, arity: *arity, func: *func });
        }
        m
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Float,
    Int,
    Op,
    OpenParen,
    CloseParen,
    Var,
    OpenSquare,
    CloseSquare,
    OpenCurly,
    CloseCurly,
    Func,
    Comma,
    End,
    ToFloat,
    ToInt32,
}

#[derive(Debug, Clone, Copy)]
struct Token {
    token_type: TokenType,
    value: Option<f32>,
    int_value: Option<i32>,
    var: Option<char>,
    op: Option<char>,
    func_idx: Option<i32>,
}

impl Default for Token {
    fn default() -> Self {
        Token {
            token_type: TokenType::End,
            value: None,
            int_value: None,
            var: None,
            op: None,
            func_idx: None,
        }
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    // C uses strncmp prefix matching with first match wins; we mimic by checking
    // each entry in order using prefix-match semantics (s prefix-matches name)
    for (_idx, name, arity, func) in FUNCTION_LIST {
        // strncmp(s, name, len(s)): returns 0 (match) when name's first len(s) chars == s
        if name.len() >= s.len() && name.as_bytes()[..s.len()] == s.as_bytes()[..s.len()] {
            // Reuse a static by using the FUNCTION_TABLE entry
            // We rely on FUNCTION_TABLE having the same data
            // Need 'static reference; store a static slice
            let _ = arity;
            let _ = func;
            // Since FUNCTION_TABLE has &'static FunctionEntry (because lazy_static)
            return FUNCTION_TABLE.get(name);
        }
    }
    None
}

#[allow(dead_code)]
fn expr_lex(_s: Vec<&str>) -> Vec<Token> {
    // Provided helper signature is awkward; we use an internal lexer instead.
    Vec::new()
}

pub struct ExprNode {
    pub tok: Token,
    pub is_float: i32,
    pub history_index: i32,
    pub vector_index: i32,
    pub next: Option<Arc<ExprNode>>,
}

pub struct MapperExpr {
    pub node: ExprNode,
    pub vector_size: i32,
    pub history_size: i32,
    pub history_pos: i32,
    pub input_history: Vec<MapperSignalValue>,
    pub output_history: Vec<MapperSignalValue>,
}

#[allow(non_camel_case_types)]
pub enum state_t {
    YEQUAL_Y,
    YEQUAL_EQ,
    EXPR,
    EXPR_RIGHT,
    TERM,
    TERM_RIGHT,
    VALUE,
    NEGATE,
    VAR_RIGHT,
    VAR_VECTINDEX,
    VAR_HISTINDEX,
    CLOSE_VECTINDEX,
    CLOSE_HISTINDEX,
    OPEN_PAREN,
    CLOSE_PAREN,
    COMMA,
    END,
}

#[allow(non_camel_case_types, dead_code)]
enum stack_obj_t {
    State(state_t),
    Node(ExprNode),
}

impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: Token::default(),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self) {
        // No-op: Rust handles cleanup via Drop.
    }
}

#[allow(dead_code)]
fn printtoken(_t: &Token) {}
#[allow(dead_code)]
fn printexprnode(_s: &str, _list: &ExprNode) {}
#[allow(dead_code)]
fn printexpr(_s: &str, _list: &MapperExpr) {}
#[allow(dead_code)]
fn printstack(_stack: &stack_obj_t, _stack_size: i32) {}
#[allow(dead_code)]
fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32) {
    // The provided signature is incomplete (missing rhs parameter).
    // Real merging is done by the internal `collapse_internal` helper used by
    // mapper_expr_new_from_string; this stub satisfies the unused signature.
}

// ===========================================================================
// Internal mutable representation used by the parser.
// ===========================================================================

#[derive(Clone, Debug)]
struct MNode {
    tok: Token,
    is_float: bool,
    history_index: i32,
    vector_index: i32,
}

impl MNode {
    fn new(tok: Token, is_float: bool) -> Self {
        MNode {
            tok,
            is_float,
            history_index: 0,
            vector_index: 0,
        }
    }
}

// Internal dual-typed value to mimic the C union.
#[derive(Clone, Copy, Default, Debug)]
struct Val {
    f: f32,
    i32: i32,
}

fn val_from_signal(v: MapperSignalValue) -> Val {
    match v {
        MapperSignalValue::F(f) => Val { f, i32: 0 },
        MapperSignalValue::I32(i) => Val { f: 0.0, i32: i },
    }
}

// ===========================================================================
// Internal lexer that mirrors expr_lex from the C code.
// ===========================================================================

struct LexState<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> LexState<'a> {
    fn new(s: &'a str) -> Self {
        LexState { bytes: s.as_bytes(), pos: 0 }
    }
    fn peek(&self) -> u8 {
        if self.pos < self.bytes.len() { self.bytes[self.pos] } else { 0 }
    }
    fn advance(&mut self) -> u8 {
        self.pos += 1;
        self.peek()
    }
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}
fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

// Simple atoi/atof translations on a slice
fn parse_int_prefix(bytes: &[u8]) -> i32 {
    let mut n: i32 = 0;
    for &b in bytes {
        if !is_digit(b) { break; }
        n = n.wrapping_mul(10).wrapping_add((b - b'0') as i32);
    }
    n
}

// Returns (token, advance)
fn lex_one(state: &mut LexState) -> Result<Token, String> {
    let mut tok = Token::default();
    let c = state.peek();

    if c == 0 {
        tok.token_type = TokenType::End;
        return Ok(tok);
    }

    // Skip whitespace
    let mut c = c;
    loop {
        match c {
            b' ' | b'\t' | b'\r' | b'\n' => {
                c = state.advance();
            }
            _ => break,
        }
    }

    if c == 0 {
        tok.token_type = TokenType::End;
        return Ok(tok);
    }

    let mut integer_found = false;
    let mut n: i32 = 0;
    let mut int_start = state.pos;

    if is_digit(c) {
        let s_start = state.pos;
        while is_digit(c) {
            c = state.advance();
        }
        // atoi from s_start
        let int_bytes = &state.bytes[s_start..state.pos];
        n = parse_int_prefix(int_bytes);
        int_start = s_start;
        integer_found = true;
        if c != b'.' {
            tok.token_type = TokenType::Int;
            tok.int_value = Some(n);
            return Ok(tok);
        }
    }

    match c {
        b'.' => {
            let s_start = state.pos; // points at '.'
            let cc = state.advance();
            if !is_digit(cc) && integer_found {
                // Plain integer followed by lone dot: treat as float n
                tok.token_type = TokenType::Float;
                tok.value = Some(n as f32);
                return Ok(tok);
            }
            if !is_digit(cc) {
                return Err(format!("unknown character '{}' in lexer", c as char));
            }
            let mut cc = cc;
            while is_digit(cc) {
                cc = state.advance();
            }
            // The C code does: tok->f = (float)n + atof(s);
            // where s is the start of the '.' (so atof reads ".XXX")
            // We mimic that: parse from int_start to current pos as a full float
            let full_start = if integer_found { int_start } else { s_start };
            let full_bytes = &state.bytes[full_start..state.pos];
            let s = std::str::from_utf8(full_bytes).unwrap_or("0");
            let f: f32 = s.parse().unwrap_or(0.0);
            tok.token_type = TokenType::Float;
            tok.value = Some(f);
            Ok(tok)
        }
        b'+' | b'-' | b'/' | b'*' | b'=' => {
            tok.token_type = TokenType::Op;
            tok.op = Some(c as char);
            state.pos += 1;
            Ok(tok)
        }
        b'(' => {
            tok.token_type = TokenType::OpenParen;
            state.pos += 1;
            Ok(tok)
        }
        b')' => {
            tok.token_type = TokenType::CloseParen;
            state.pos += 1;
            Ok(tok)
        }
        b'x' | b'y' => {
            tok.token_type = TokenType::Var;
            tok.var = Some(c as char);
            state.pos += 1;
            Ok(tok)
        }
        b'[' => {
            tok.token_type = TokenType::OpenSquare;
            state.pos += 1;
            Ok(tok)
        }
        b']' => {
            tok.token_type = TokenType::CloseSquare;
            state.pos += 1;
            Ok(tok)
        }
        b'{' => {
            tok.token_type = TokenType::OpenCurly;
            state.pos += 1;
            Ok(tok)
        }
        b'}' => {
            tok.token_type = TokenType::CloseCurly;
            state.pos += 1;
            Ok(tok)
        }
        b',' => {
            tok.token_type = TokenType::Comma;
            state.pos += 1;
            Ok(tok)
        }
        _ => {
            if !is_alpha(c) {
                return Err(format!("unknown character '{}' in lexer", c as char));
            }
            let s_start = state.pos;
            let mut cc = c;
            while cc != 0 && (is_alpha(cc) || is_digit(cc)) {
                cc = state.advance();
            }
            let name = std::str::from_utf8(&state.bytes[s_start..state.pos])
                .unwrap_or("");
            tok.token_type = TokenType::Func;
            // Find function index via prefix-match (matches C's strncmp behavior)
            let mut idx: i32 = -1;
            for (i, fname, _arity, _func) in FUNCTION_LIST {
                if fname.len() >= name.len()
                    && fname.as_bytes()[..name.len()] == *name.as_bytes()
                {
                    idx = *i;
                    break;
                }
            }
            tok.func_idx = Some(idx);
            Ok(tok)
        }
    }
}

// ===========================================================================
// Parser
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum PState {
    YEqualY,
    YEqualEq,
    Expr,
    ExprRight,
    Term,
    TermRight,
    Value,
    Negate,
    VarRight,
    VarVectIndex,
    VarHistIndex,
    CloseVectIndex,
    CloseHistIndex,
    OpenParen,
    ClosePareN,
    Comma,
    End,
}

enum StackEntry {
    State(PState),
    Node(Vec<MNode>),
}

fn collapse_internal(
    lhs: &mut Vec<MNode>,
    mut rhs: Vec<MNode>,
    constant_folding: bool,
) {
    // Track variable refs across both lists
    let mut refvar = false;
    for n in lhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
        }
    }
    for n in rhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
        }
    }

    let lhs_tail_idx = lhs.len() - 1;
    let rhs_last_idx = rhs.len() - 1;
    let lhs_tail_is_float = lhs[lhs_tail_idx].is_float;
    let rhs_last_is_float = rhs[rhs_last_idx].is_float;
    let is_float = lhs_tail_is_float || rhs_last_is_float;

    if lhs_tail_is_float && !rhs_last_is_float {
        // Append (float) coercion to rhs (after rhs_last)
        let mut t = Token::default();
        t.token_type = TokenType::ToFloat;
        rhs.push(MNode::new(t, true));
    } else if !lhs_tail_is_float && rhs_last_is_float {
        // Insert (float) coercion in lhs right BEFORE its tail node,
        // and set tail's is_float=true.
        let mut t = Token::default();
        t.token_type = TokenType::ToFloat;
        let new_node = MNode::new(t, true);
        lhs.insert(lhs_tail_idx, new_node);
        // The tail moved to lhs_tail_idx + 1
        let new_tail_idx = lhs_tail_idx + 1;
        lhs[new_tail_idx].is_float = true;
    }

    // Insert rhs before the lhs's tail node.
    let tail = lhs.pop().unwrap();
    lhs.extend(rhs);
    lhs.push(tail);

    // If no variable references and constant folding requested, evaluate now.
    if constant_folding && !refvar {
        let v = eval_const(lhs);
        lhs.clear();
        let mut t = Token::default();
        if is_float {
            t.token_type = TokenType::Float;
            t.value = Some(v.f);
            lhs.push(MNode::new(t, true));
        } else {
            t.token_type = TokenType::Int;
            t.int_value = Some(v.i32);
            lhs.push(MNode::new(t, false));
        }
    }
}

fn eval_const(nodes: &[MNode]) -> Val {
    let mut stack: [Val; STACK_SIZE] = [Val::default(); STACK_SIZE];
    let mut top: i32 = -1;
    for node in nodes {
        match node.tok.token_type {
            TokenType::Int => {
                top += 1;
                stack[top as usize] = Val { f: 0.0, i32: node.tok.int_value.unwrap_or(0) };
            }
            TokenType::Float => {
                top += 1;
                stack[top as usize] = Val { f: node.tok.value.unwrap_or(0.0), i32: 0 };
            }
            TokenType::ToFloat => {
                let v = stack[top as usize];
                stack[top as usize] = Val { f: v.i32 as f32, i32: v.i32 };
            }
            TokenType::ToInt32 => {
                let v = stack[top as usize];
                stack[top as usize] = Val { f: v.f, i32: v.f as i32 };
            }
            TokenType::Op => {
                let right = stack[top as usize];
                top -= 1;
                let left = stack[top as usize];
                top -= 1;
                let result = if node.is_float {
                    match node.tok.op.unwrap_or('+') {
                        '+' => Val { f: left.f + right.f, i32: 0 },
                        '-' => Val { f: left.f - right.f, i32: 0 },
                        '*' => Val { f: left.f * right.f, i32: 0 },
                        '/' => Val { f: left.f / right.f, i32: 0 },
                        _ => Val::default(),
                    }
                } else {
                    match node.tok.op.unwrap_or('+') {
                        '+' => Val { f: 0.0, i32: left.i32.wrapping_add(right.i32) },
                        '-' => Val { f: 0.0, i32: left.i32.wrapping_sub(right.i32) },
                        '*' => Val { f: 0.0, i32: left.i32.wrapping_mul(right.i32) },
                        '/' => Val { f: 0.0, i32: if right.i32 == 0 { 0 } else { left.i32 / right.i32 } },
                        _ => Val::default(),
                    }
                };
                top += 1;
                stack[top as usize] = result;
            }
            TokenType::Func => {
                let idx = node.tok.func_idx.unwrap_or(-1);
                let entry = FUNCTION_BY_INDEX.get(&idx);
                if let Some(entry) = entry {
                    let f = entry.func;
                    match entry.arity {
                        0 => {
                            top += 1;
                            stack[top as usize] = Val { f: f(0.0, 0.0), i32: 0 };
                        }
                        1 => {
                            let v = stack[top as usize];
                            stack[top as usize] = Val { f: f(v.f, 0.0), i32: 0 };
                        }
                        2 => {
                            let right = stack[top as usize];
                            top -= 1;
                            let left = stack[top as usize];
                            stack[top as usize] = Val { f: f(left.f, right.f), i32: 0 };
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if top < 0 { Val::default() } else { stack[0] }
}

fn vec_to_node_chain(mut nodes: Vec<MNode>) -> ExprNode {
    if nodes.is_empty() {
        return ExprNode::new();
    }
    // Build chain from tail to head using Arc.
    let last = nodes.pop().unwrap();
    let mut current: Option<Arc<ExprNode>> = Some(Arc::new(ExprNode {
        tok: last.tok,
        is_float: if last.is_float { 1 } else { 0 },
        history_index: last.history_index,
        vector_index: last.vector_index,
        next: None,
    }));
    while let Some(m) = nodes.pop() {
        let node = ExprNode {
            tok: m.tok,
            is_float: if m.is_float { 1 } else { 0 },
            history_index: m.history_index,
            vector_index: m.vector_index,
            next: current.take(),
        };
        current = Some(Arc::new(node));
    }
    // Now unwrap the head (move its contents out of the Arc)
    // Since the head Arc has 1 owner, we can use Arc::try_unwrap.
    let arc = current.unwrap();
    match Arc::try_unwrap(arc) {
        Ok(node) => node,
        Err(arc) => {
            // Should not happen, but clone if necessary.
            let n = &*arc;
            ExprNode {
                tok: n.tok,
                is_float: n.is_float,
                history_index: n.history_index,
                vector_index: n.vector_index,
                next: n.next.clone(),
            }
        }
    }
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    let parsed = parse_expr(s, input_is_float, output_is_float, vector_size);
    match parsed {
        Some(expr) => expr,
        None => {
            // Return an empty MapperExpr to avoid panicking.
            MapperExpr {
                node: ExprNode::new(),
                vector_size: 0,
                history_size: 0,
                history_pos: -1,
                input_history: vec![],
                output_history: vec![],
            }
        }
    }
}

fn parse_expr(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> Option<MapperExpr> {
    let mut lex = LexState::new(s);
    let mut stack: Vec<StackEntry> = Vec::with_capacity(STACK_SIZE);

    // Push initial states
    stack.push(StackEntry::State(PState::Expr));
    stack.push(StackEntry::State(PState::YEqualEq));
    stack.push(StackEntry::State(PState::YEqualY));

    let mut tok: Token = Token::default();
    let mut next_token = true;
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    let mut result: Option<Vec<MNode>> = None;
    let mut error_message: Option<String> = None;

    'outer: while !stack.is_empty() {
        if next_token {
            match lex_one(&mut lex) {
                Ok(t) => tok = t,
                Err(_e) => {
                    error_message = Some("Error in lexical analysis.".to_string());
                    break 'outer;
                }
            }
            next_token = false;
        }

        let top_idx = stack.len() - 1;
        let top_is_node = matches!(stack[top_idx], StackEntry::Node(_));

        if top_is_node {
            if top_idx == 0 {
                // SUCCESS
                if let StackEntry::Node(v) = stack.remove(0) {
                    result = Some(v);
                }
                break 'outer;
            }
            // top-1 is state? In our representation we've ensured this.
            // Check based on stack[top-1] type
            let one_down_is_state = matches!(stack[top_idx - 1], StackEntry::State(_));
            if one_down_is_state {
                if top_idx >= 2 && matches!(stack[top_idx - 2], StackEntry::Node(_)) {
                    // We need to check the state to decide which collapse to do
                    let state_val = if let StackEntry::State(s) = stack[top_idx - 1] { s } else { unreachable!() };
                    match state_val {
                        PState::ExprRight | PState::TermRight | PState::ClosePareN => {
                            // collapse top into top-2 with constant_folding=1
                            // pop top
                            let top_node = if let StackEntry::Node(v) = stack.pop().unwrap() { v } else { unreachable!() };
                            // pop the state (we'll restore it after collapse)
                            let st = stack.pop().unwrap();
                            // collapse into stack[top-2] which is now stack[top-1] (= last)
                            if let Some(StackEntry::Node(lhs)) = stack.last_mut() {
                                collapse_internal(lhs, top_node, true);
                            }
                            // restore state
                            stack.push(st);
                        }
                        PState::CloseHistIndex => {
                            // pop top
                            let top_node = if let StackEntry::Node(v) = stack.pop().unwrap() { v } else { unreachable!() };
                            let st = stack.pop().unwrap();
                            // expected: top.node is single INT or FLOAT, top-2 is VAR
                            if let Some(StackEntry::Node(target)) = stack.last_mut() {
                                if !target.is_empty()
                                    && target[0].tok.token_type == TokenType::Var
                                    && top_node.len() == 1
                                {
                                    let n = &top_node[0];
                                    let hist_idx = match n.tok.token_type {
                                        TokenType::Float => n.tok.value.unwrap_or(0.0) as i32,
                                        TokenType::Int => n.tok.int_value.unwrap_or(0),
                                        _ => 0,
                                    };
                                    target[0].history_index = hist_idx;
                                    if oldest_samps > hist_idx as f32 {
                                        oldest_samps = hist_idx as f32;
                                    }
                                }
                            }
                            stack.push(st);
                        }
                        PState::CloseVectIndex => {
                            let top_node = if let StackEntry::Node(v) = stack.pop().unwrap() { v } else { unreachable!() };
                            let st = stack.pop().unwrap();
                            let mut fail_msg: Option<&'static str> = None;
                            if let Some(StackEntry::Node(target)) = stack.last_mut() {
                                if !target.is_empty()
                                    && target[0].tok.token_type == TokenType::Var
                                    && top_node.len() == 1
                                {
                                    let n = &top_node[0];
                                    let vec_idx = match n.tok.token_type {
                                        TokenType::Float => n.tok.value.unwrap_or(0.0) as i32,
                                        TokenType::Int => n.tok.int_value.unwrap_or(0),
                                        _ => 0,
                                    };
                                    target[0].vector_index = vec_idx;
                                    if vec_idx > 0 {
                                        fail_msg = Some("Vector indexing not yet implemented.");
                                    } else if vec_idx < 0 || vec_idx >= vector_size {
                                        fail_msg = Some("Vector index outside input size.");
                                    }
                                }
                            }
                            stack.push(st);
                            if let Some(m) = fail_msg {
                                error_message = Some(m.to_string());
                                break 'outer;
                            }
                        }
                        _ => {
                            // Not a collapse case — swap
                            let top_entry = stack.pop().unwrap();
                            let state_entry = stack.pop().unwrap();
                            stack.push(top_entry);
                            stack.push(state_entry);
                        }
                    }
                } else {
                    // top-2 is not ST_NODE (or no top-2): swap
                    let top_entry = stack.pop().unwrap();
                    let state_entry = stack.pop().unwrap();
                    stack.push(top_entry);
                    stack.push(state_entry);
                }
            }
            continue;
        }

        // Top is a state
        let state_val = if let StackEntry::State(s) = stack[top_idx] { s } else { unreachable!() };

        match state_val {
            PState::YEqualY => {
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.".to_string());
                    break 'outer;
                }
                next_token = true;
            }
            PState::YEqualEq => {
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.".to_string());
                    break 'outer;
                }
                next_token = true;
            }
            PState::Expr => {
                stack.pop();
                stack.push(StackEntry::State(PState::ExprRight));
                stack.push(StackEntry::State(PState::Term));
            }
            PState::ExprRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('+') || tok.op == Some('-') {
                        // APPEND_OP to the top node (after popping)
                        // Find the topmost ST_NODE in the stack and append op
                        for i in (0..stack.len()).rev() {
                            if let StackEntry::Node(ref mut nodes) = stack[i] {
                                let mut new_tok = Token::default();
                                new_tok.token_type = TokenType::Op;
                                new_tok.op = tok.op;
                                let last_is_float = nodes.last().map(|n| n.is_float).unwrap_or(false);
                                let mut newn = MNode::new(new_tok, false);
                                newn.is_float = last_is_float;
                                nodes.push(newn);
                                break;
                            }
                        }
                        stack.push(StackEntry::State(PState::Expr));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            PState::Term => {
                stack.pop();
                stack.push(StackEntry::State(PState::TermRight));
                stack.push(StackEntry::State(PState::Value));
            }
            PState::TermRight => {
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('*') || tok.op == Some('/') {
                        for i in (0..stack.len()).rev() {
                            if let StackEntry::Node(ref mut nodes) = stack[i] {
                                let mut new_tok = Token::default();
                                new_tok.token_type = TokenType::Op;
                                new_tok.op = tok.op;
                                let last_is_float = nodes.last().map(|n| n.is_float).unwrap_or(false);
                                let mut newn = MNode::new(new_tok, false);
                                newn.is_float = last_is_float;
                                nodes.push(newn);
                                break;
                            }
                        }
                        stack.push(StackEntry::State(PState::Term));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            PState::Value => {
                if tok.token_type == TokenType::Int {
                    stack.pop();
                    stack.push(StackEntry::Node(vec![MNode::new(tok, false)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Float {
                    stack.pop();
                    stack.push(StackEntry::Node(vec![MNode::new(tok, true)]));
                    next_token = true;
                } else if tok.token_type == TokenType::Var {
                    if var_allowed {
                        stack.pop();
                        let is_float = input_is_float != 0;
                        stack.push(StackEntry::Node(vec![MNode::new(tok, is_float)]));
                        stack.push(StackEntry::State(PState::VarRight));
                        next_token = true;
                    } else {
                        error_message = Some("Unexpected variable reference.".to_string());
                        break 'outer;
                    }
                } else if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    stack.push(StackEntry::State(PState::ClosePareN));
                    stack.push(StackEntry::State(PState::Expr));
                    next_token = true;
                } else if tok.token_type == TokenType::Func {
                    stack.pop();
                    let idx = tok.func_idx.unwrap_or(-1);
                    if idx < 0 {
                        error_message = Some("Unknown function.".to_string());
                        break 'outer;
                    }
                    let arity = FUNCTION_BY_INDEX.get(&idx).map(|e| e.arity).unwrap_or(0);
                    stack.push(StackEntry::Node(vec![MNode::new(tok, true)]));
                    if arity > 0 {
                        stack.push(StackEntry::State(PState::ClosePareN));
                        stack.push(StackEntry::State(PState::Expr));
                        for _ in 1..arity {
                            stack.push(StackEntry::State(PState::Comma));
                            stack.push(StackEntry::State(PState::Expr));
                        }
                        stack.push(StackEntry::State(PState::OpenParen));
                    }
                    next_token = true;
                } else if tok.token_type == TokenType::Op && tok.op == Some('-') {
                    stack.pop();
                    stack.push(StackEntry::State(PState::Negate));
                    stack.push(StackEntry::State(PState::Value));
                    next_token = true;
                } else {
                    error_message = Some("Expected value.".to_string());
                    break 'outer;
                }
            }
            PState::Negate => {
                stack.pop();
                // Top should now be a node with the value to negate.
                let top_idx2 = stack.len() - 1;
                if let StackEntry::Node(_) = stack[top_idx2] {
                    let mut e: Vec<MNode> = Vec::new();
                    let mut t = Token::default();
                    t.token_type = TokenType::Int;
                    t.int_value = Some(0);
                    e.push(MNode::new(t, false));
                    let mut t2 = Token::default();
                    t2.token_type = TokenType::Op;
                    t2.op = Some('-');
                    let mut op_node = MNode::new(t2, false);
                    op_node.is_float = false; // is_float of [0] is false
                    e.push(op_node);
                    let rhs = if let StackEntry::Node(v) = stack.pop().unwrap() { v } else { unreachable!() };
                    collapse_internal(&mut e, rhs, true);
                    stack.push(StackEntry::Node(e));
                } else {
                    error_message = Some("Expected to negate an expression.".to_string());
                    break 'outer;
                }
            }
            PState::VarRight => {
                if tok.token_type == TokenType::OpenSquare {
                    stack.pop();
                    stack.push(StackEntry::State(PState::VarVectIndex));
                } else if tok.token_type == TokenType::OpenCurly {
                    stack.pop();
                    stack.push(StackEntry::State(PState::VarHistIndex));
                } else {
                    stack.pop();
                }
            }
            PState::VarVectIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(StackEntry::State(PState::CloseVectIndex));
                    stack.push(StackEntry::State(PState::Expr));
                    next_token = true;
                }
            }
            PState::VarHistIndex => {
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(StackEntry::State(PState::CloseHistIndex));
                    stack.push(StackEntry::State(PState::Expr));
                    next_token = true;
                }
            }
            PState::CloseVectIndex => {
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackEntry::State(PState::VarHistIndex));
                    next_token = true;
                } else {
                    error_message = Some("Expected ']'.".to_string());
                    break 'outer;
                }
            }
            PState::CloseHistIndex => {
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackEntry::State(PState::VarVectIndex));
                    next_token = true;
                } else {
                    error_message = Some("Expected '}'.".to_string());
                    break 'outer;
                }
            }
            PState::ClosePareN => {
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected ')'.".to_string());
                    break 'outer;
                }
            }
            PState::Comma => {
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    // After popping COMMA, look for previous node on stack
                    // Find the last ST_NODE before the (now) top.
                    let top_idx2 = stack.len() - 1;
                    // top is the current node (stack[top_idx2])
                    // search from top_idx2 - 1 downward for first ST_NODE
                    let mut found: Option<usize> = None;
                    for i in (0..top_idx2).rev() {
                        if matches!(stack[i], StackEntry::Node(_)) {
                            found = Some(i);
                            break;
                        }
                    }
                    if let Some(i) = found {
                        let top_node = if let StackEntry::Node(v) = stack.pop().unwrap() { v } else { unreachable!() };
                        if let StackEntry::Node(ref mut lhs) = stack[i] {
                            collapse_internal(lhs, top_node, false);
                        }
                    }
                    next_token = true;
                } else {
                    error_message = Some("Expected ','.".to_string());
                    break 'outer;
                }
            }
            PState::OpenParen => {
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected '('.".to_string());
                    break 'outer;
                }
            }
            PState::End => {
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    error_message = Some("Expected END.".to_string());
                    break 'outer;
                }
            }
        }
    }

    let mut result_nodes = match result {
        Some(r) => r,
        None => {
            if let Some(m) = error_message {
                println!("{}", m);
            }
            return None;
        }
    };

    // History-reference sanity check
    if oldest_samps < -100.0 {
        return None;
    }

    // Coerce final output if necessary
    if !result_nodes.is_empty() {
        let last = result_nodes.last().unwrap();
        let last_is_float = last.is_float;
        if last_is_float && output_is_float == 0 {
            let mut t = Token::default();
            t.token_type = TokenType::ToInt32;
            result_nodes.push(MNode::new(t, false));
        } else if !last_is_float && output_is_float != 0 {
            let mut t = Token::default();
            t.token_type = TokenType::ToFloat;
            result_nodes.push(MNode::new(t, true));
        }
    }

    // Disallow vector indexing > 0 if vector_size > 1 (not implemented)
    if vector_size > 1 {
        for n in &result_nodes {
            if n.tok.token_type == TokenType::Var && n.vector_index > 0 {
                return None;
            }
        }
    }

    let history_size = (-oldest_samps).ceil() as i32 + 1;
    let history_size_usize = history_size.max(1) as usize;
    let vector_size_usize = vector_size.max(1) as usize;

    let input_history = vec![MapperSignalValue::I32(0); vector_size_usize * history_size_usize];
    let output_history = vec![MapperSignalValue::I32(0); history_size_usize];

    let head = vec_to_node_chain(result_nodes);
    Some(MapperExpr {
        node: head,
        vector_size,
        history_size,
        history_pos: -1,
        input_history,
        output_history,
    })
}

// ===========================================================================
// Evaluator
// ===========================================================================

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    // Convert input to internal Val
    let input_val = val_from_signal(*input);
    let vector_size = mapper.vector_size.max(1) as usize;
    let history_size = mapper.history_size.max(1) as i32;

    // Update history
    mapper.history_pos = (mapper.history_pos + 1).rem_euclid(history_size);

    // Convert history vectors of MapperSignalValue to internal Val arrays.
    let mut input_history: Vec<Val> = mapper
        .input_history
        .iter()
        .copied()
        .map(val_from_signal)
        .collect();
    let mut output_history: Vec<Val> = mapper
        .output_history
        .iter()
        .copied()
        .map(val_from_signal)
        .collect();

    // Make sure they're correctly sized
    let total_input = vector_size * history_size as usize;
    if input_history.len() < total_input {
        input_history.resize(total_input, Val::default());
    }
    if output_history.len() < history_size as usize {
        output_history.resize(history_size as usize, Val::default());
    }

    // Place new input at current history position (we treat input as a single
    // value — vector_size is always 1 for the Rust API).
    let pos = mapper.history_pos as usize;
    if vector_size >= 1 {
        let idx = pos * vector_size; // vector_index defaults to 0
        if idx < input_history.len() {
            input_history[idx] = input_val;
        }
    }

    // Evaluate
    let mut stack: [Val; STACK_SIZE] = [Val::default(); STACK_SIZE];
    let mut top: i32 = -1;

    let mut node_opt: Option<&ExprNode> = Some(&mapper.node);
    let mut error = false;

    while let Some(node) = node_opt {
        match node.tok.token_type {
            TokenType::Int => {
                top += 1;
                stack[top as usize] = Val { f: 0.0, i32: node.tok.int_value.unwrap_or(0) };
            }
            TokenType::Float => {
                top += 1;
                stack[top as usize] = Val { f: node.tok.value.unwrap_or(0.0), i32: 0 };
            }
            TokenType::Var => {
                let idx = ((node.history_index + mapper.history_pos
                    + history_size)
                    .rem_euclid(history_size)) as usize;
                match node.tok.var.unwrap_or('?') {
                    'x' => {
                        let i = idx * vector_size + node.vector_index as usize;
                        top += 1;
                        if i < input_history.len() {
                            stack[top as usize] = input_history[i];
                        } else {
                            stack[top as usize] = Val::default();
                        }
                    }
                    'y' => {
                        top += 1;
                        if idx < output_history.len() {
                            stack[top as usize] = output_history[idx];
                        } else {
                            stack[top as usize] = Val::default();
                        }
                    }
                    _ => {
                        error = true;
                        break;
                    }
                }
            }
            TokenType::ToFloat => {
                let v = stack[top as usize];
                stack[top as usize] = Val { f: v.i32 as f32, i32: v.i32 };
            }
            TokenType::ToInt32 => {
                let v = stack[top as usize];
                stack[top as usize] = Val { f: v.f, i32: v.f as i32 };
            }
            TokenType::Op => {
                let right = stack[top as usize];
                top -= 1;
                let left = stack[top as usize];
                top -= 1;
                let result = if node.is_float != 0 {
                    match node.tok.op.unwrap_or('+') {
                        '+' => Val { f: left.f + right.f, i32: 0 },
                        '-' => Val { f: left.f - right.f, i32: 0 },
                        '*' => Val { f: left.f * right.f, i32: 0 },
                        '/' => Val { f: left.f / right.f, i32: 0 },
                        _ => { error = true; break; }
                    }
                } else {
                    match node.tok.op.unwrap_or('+') {
                        '+' => Val { f: 0.0, i32: left.i32.wrapping_add(right.i32) },
                        '-' => Val { f: 0.0, i32: left.i32.wrapping_sub(right.i32) },
                        '*' => Val { f: 0.0, i32: left.i32.wrapping_mul(right.i32) },
                        '/' => Val { f: 0.0, i32: if right.i32 == 0 { 0 } else { left.i32 / right.i32 } },
                        _ => { error = true; break; }
                    }
                };
                top += 1;
                stack[top as usize] = result;
            }
            TokenType::Func => {
                let idx = node.tok.func_idx.unwrap_or(-1);
                if let Some(entry) = FUNCTION_BY_INDEX.get(&idx) {
                    let f = entry.func;
                    match entry.arity {
                        0 => {
                            top += 1;
                            stack[top as usize] = Val { f: f(0.0, 0.0), i32: 0 };
                        }
                        1 => {
                            let v = stack[top as usize];
                            stack[top as usize] = Val { f: f(v.f, 0.0), i32: 0 };
                        }
                        2 => {
                            let right = stack[top as usize];
                            top -= 1;
                            let left = stack[top as usize];
                            stack[top as usize] = Val { f: f(left.f, right.f), i32: 0 };
                        }
                        _ => { error = true; break; }
                    }
                } else {
                    error = true;
                    break;
                }
            }
            _ => {
                error = true;
                break;
            }
        }
        node_opt = node.next.as_ref().map(|a| a.as_ref());
    }

    if error {
        // On error, write output_history pos with zeros and return zero.
        // Save back history.
        mapper.input_history = input_history.iter().map(|v| MapperSignalValue::I32(v.i32)).collect();
        return MapperSignalValue::I32(0);
    }

    // Save back input history
    mapper.input_history = input_history
        .iter()
        .enumerate()
        .map(|(i, v)| {
            // Heuristic: positions corresponding to current pos are the most-recent input.
            // We don't know expected variant here; we just round-trip whatever input we got.
            // Use the input variant for the current position; default I32(0) elsewhere.
            if i == pos * vector_size {
                *input
            } else {
                // Preserve old slot's variant by inspecting the original input_history.
                if i < mapper.input_history.len() {
                    mapper.input_history[i]
                } else {
                    MapperSignalValue::I32(v.i32)
                }
            }
        })
        .collect();

    // Determine output variant from final node's is_float (or last token)
    // To know if output is float, check the last node in chain.
    let mut last_is_float = false;
    {
        let mut nopt = Some(&mapper.node);
        while let Some(n) = nopt {
            if n.next.is_none() {
                last_is_float = n.is_float != 0
                    || n.tok.token_type == TokenType::Float
                    || n.tok.token_type == TokenType::ToFloat;
                if n.tok.token_type == TokenType::ToInt32 {
                    last_is_float = false;
                }
                break;
            }
            nopt = n.next.as_ref().map(|a| a.as_ref());
        }
    }

    let result_val = stack[0];
    let output = if last_is_float {
        MapperSignalValue::F(result_val.f)
    } else {
        MapperSignalValue::I32(result_val.i32)
    };

    // Save to output history
    if pos < output_history.len() {
        output_history[pos] = result_val;
    }
    mapper.output_history = output_history
        .iter()
        .map(|v| {
            if last_is_float {
                MapperSignalValue::F(v.f)
            } else {
                MapperSignalValue::I32(v.i32)
            }
        })
        .collect();

    output
}
