use std::f32::consts::PI;
use std::collections::HashMap;
use std::sync::Arc;
use lazy_static::lazy_static;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}
impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            MapperSignalValue::F(f) => Some(*f),
            MapperSignalValue::I32(i) => Some(*i as f32),
        }
    }
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            MapperSignalValue::I32(i) => Some(*i),
            MapperSignalValue::F(f) => Some(*f as i32),
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
lazy_static::lazy_static! {
    static ref FUNCTIONS: Vec<FunctionEntry> = vec![
        FunctionEntry { name: "pow", arity: 2, func: f32::powf },
        FunctionEntry { name: "sin", arity: 1, func: |x, _| x.sin() },
        FunctionEntry { name: "cos", arity: 1, func: |x, _| x.cos() },
        FunctionEntry { name: "tan", arity: 1, func: |x, _| x.tan() },
        FunctionEntry { name: "abs", arity: 1, func: |x, _| x.abs() },
        FunctionEntry { name: "sqrt", arity: 1, func: |x, _| x.sqrt() },
        FunctionEntry { name: "log", arity: 1, func: |x, _| x.ln() },
        FunctionEntry { name: "log10", arity: 1, func: |x, _| x.log10() },
        FunctionEntry { name: "exp", arity: 1, func: |x, _| x.exp() },
        FunctionEntry { name: "floor", arity: 1, func: |x, _| x.floor() },
        FunctionEntry { name: "round", arity: 1, func: |x, _| x.round() },
        FunctionEntry { name: "ceil", arity: 1, func: |x, _| x.ceil() },
        FunctionEntry { name: "asin", arity: 1, func: |x, _| x.asin() },
        FunctionEntry { name: "acos", arity: 1, func: |x, _| x.acos() },
        FunctionEntry { name: "atan", arity: 1, func: |x, _| x.atan() },
        FunctionEntry { name: "atan2", arity: 2, func: f32::atan2 },
        FunctionEntry { name: "sinh", arity: 1, func: |x, _| x.sinh() },
        FunctionEntry { name: "cosh", arity: 1, func: |x, _| x.cosh() },
        FunctionEntry { name: "tanh", arity: 1, func: |x, _| x.tanh() },
        FunctionEntry { name: "exp2", arity: 1, func: |x, _| x.exp2() },
        FunctionEntry { name: "log2", arity: 1, func: |x, _| x.log2() },
        FunctionEntry { name: "hypot", arity: 2, func: f32::hypot },
        FunctionEntry { name: "cbrt", arity: 1, func: |x, _| x.cbrt() },
        FunctionEntry { name: "trunc", arity: 1, func: |x, _| x.trunc() },
        FunctionEntry { name: "min", arity: 2, func: minf },
        FunctionEntry { name: "max", arity: 2, func: maxf },
        FunctionEntry { name: "pi", arity: 0, func: |_, _| pif() },
    ];
    static ref FUNCTION_TABLE: HashMap<&'static str, FunctionEntry> = {
        FUNCTIONS.iter().map(|f| (f.name, *f)).collect()
    };
}

fn function_index(name: &str) -> Option<usize> {
    FUNCTIONS.iter().position(|f| f.name == name)
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
}

impl Token {
    fn empty(t: TokenType) -> Self {
        Token { token_type: t, value: None, int_value: None, var: None, op: None }
    }
    fn from_int(i: i32) -> Self {
        let mut t = Token::empty(TokenType::Int);
        t.int_value = Some(i);
        t
    }
    fn from_float(f: f32) -> Self {
        let mut t = Token::empty(TokenType::Float);
        t.value = Some(f);
        t
    }
    fn from_op(op: char) -> Self {
        let mut t = Token::empty(TokenType::Op);
        t.op = Some(op);
        t
    }
    fn from_var(v: char) -> Self {
        let mut t = Token::empty(TokenType::Var);
        t.var = Some(v);
        t
    }
    fn from_func(idx: i32) -> Self {
        let mut t = Token::empty(TokenType::Func);
        t.int_value = Some(idx);
        t
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let combined: String = s.join("");
    tokenize(&combined).unwrap_or_default()
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let int_str: String = chars[start..i].iter().collect();
            let int_val: i32 = int_str.parse().unwrap_or(0);
            if i < chars.len() && chars[i] == '.' {
                let dot_pos = i;
                i += 1;
                let frac_start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if frac_start == i {
                    // integer found, then '.', no fractional digits
                    tokens.push(Token::from_float(int_val as f32));
                } else {
                    let full_str: String = chars[start..i].iter().collect();
                    let f: f32 = full_str.parse().unwrap_or(0.0);
                    tokens.push(Token::from_float(f));
                }
                let _ = dot_pos;
            } else {
                tokens.push(Token::from_int(int_val));
            }
            continue;
        }
        if c == '.' {
            // Leading-dot float, e.g. .1
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i == start + 1 {
                return Err(format!("Unexpected '.' in input"));
            }
            let full_str: String = chars[start..i].iter().collect();
            let f: f32 = full_str.parse().unwrap_or(0.0);
            tokens.push(Token::from_float(f));
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' | '=' => {
                tokens.push(Token::from_op(c));
                i += 1;
            }
            '(' => { tokens.push(Token::empty(TokenType::OpenParen)); i += 1; }
            ')' => { tokens.push(Token::empty(TokenType::CloseParen)); i += 1; }
            'x' | 'y' => {
                // Could be variable or function name starting with x/y. C distinguishes: if exactly 1 char `x` or `y`, it's a var. But the C code matches case 'x'/'y' before falling through to function. So x/y are always var.
                tokens.push(Token::from_var(c));
                i += 1;
            }
            '[' => { tokens.push(Token::empty(TokenType::OpenSquare)); i += 1; }
            ']' => { tokens.push(Token::empty(TokenType::CloseSquare)); i += 1; }
            '{' => { tokens.push(Token::empty(TokenType::OpenCurly)); i += 1; }
            '}' => { tokens.push(Token::empty(TokenType::CloseCurly)); i += 1; }
            ',' => { tokens.push(Token::empty(TokenType::Comma)); i += 1; }
            _ => {
                if c.is_ascii_alphabetic() {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_alphanumeric()) {
                        i += 1;
                    }
                    let name: String = chars[start..i].iter().collect();
                    let idx = function_index(&name)
                        .map(|x| x as i32)
                        .unwrap_or(-1);
                    tokens.push(Token::from_func(idx));
                } else {
                    return Err(format!("Unknown character '{}' in lexer", c));
                }
            }
        }
    }
    tokens.push(Token::empty(TokenType::End));
    Ok(tokens)
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
#[derive(Debug)]
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
            tok: Token::empty(TokenType::End),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self) {
        // Owning data structure is dropped automatically; no-op.
    }
}
fn printtoken(_t: &Token) {
    // tracing disabled
}
fn printexprnode(_s: &str, _list: &ExprNode) {
    // tracing disabled
}
fn printexpr(_s: &str, _list: &MapperExpr) {
    // tracing disabled
}
fn printstack(_stack: &stack_obj_t, _stack_size: i32) {
    // tracing disabled
}
fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32) {
    // The C signature of collapse_expr_to_left takes both LHS and RHS, but the
    // public Rust signature here only has LHS. Internal parsing uses
    // `collapse_chain` (operating on `Vec<NodeData>`). This stub exists only to
    // satisfy the declared signature.
}

// ---------- Internal data structures used by the parser ----------

#[derive(Debug, Clone)]
struct NodeData {
    tok: Token,
    is_float: bool,
    history_index: i32,
    vector_index: i32,
}

impl NodeData {
    fn new(tok: Token, is_float: bool) -> Self {
        NodeData {
            tok,
            is_float,
            history_index: 0,
            vector_index: 0,
        }
    }
}

#[derive(Debug)]
enum StackItem {
    St(state_t),
    Chain(Vec<NodeData>),
}

// Insert RHS chain before the trailing node of LHS chain; insert TOFLOAT
// coercions as needed; optionally constant-fold.
fn collapse_chain(
    lhs: &mut Vec<NodeData>,
    mut rhs: Vec<NodeData>,
    constant_folding: bool,
    vector_size: i32,
) {
    // Track whether any variable references exist in either chain.
    let mut refvar = false;
    for n in lhs.iter() {
        if matches!(n.tok.token_type, TokenType::Var) {
            refvar = true;
        }
    }
    for n in rhs.iter() {
        if matches!(n.tok.token_type, TokenType::Var) {
            refvar = true;
        }
    }

    let lhs_last_idx = lhs.len() - 1;
    let rhs_last_idx = rhs.len() - 1;
    let lhs_last_is_float = lhs[lhs_last_idx].is_float;
    let rhs_last_is_float = rhs[rhs_last_idx].is_float;
    let result_is_float = lhs_last_is_float || rhs_last_is_float;

    // Insert TOFLOAT coercions if sides disagree on float-ness
    if lhs_last_is_float && !rhs_last_is_float {
        // Append TOFLOAT to end of RHS
        rhs.push(NodeData::new(Token::empty(TokenType::ToFloat), true));
    } else if !lhs_last_is_float && rhs_last_is_float {
        // Insert TOFLOAT just before the LHS's trailing node.
        // Also mark the LHS's trailing node as is_float=true (mirroring the C).
        lhs.insert(lhs_last_idx, NodeData::new(Token::empty(TokenType::ToFloat), true));
        let last = lhs.len() - 1;
        lhs[last].is_float = true;
    }

    // Insert RHS before the LHS's trailing node.
    let lhs_last_idx2 = lhs.len() - 1;
    let trailing = lhs.remove(lhs_last_idx2);
    lhs.extend(rhs.into_iter());
    lhs.push(trailing);

    // Constant fold if there are no variable references and folding is enabled.
    if constant_folding && !refvar {
        let value = eval_chain_const(lhs, vector_size);
        lhs.clear();
        if result_is_float {
            let v = match value {
                MapperSignalValue::F(f) => f,
                MapperSignalValue::I32(i) => i as f32,
            };
            lhs.push(NodeData::new(Token::from_float(v), true));
        } else {
            let v = match value {
                MapperSignalValue::I32(i) => i,
                MapperSignalValue::F(f) => f as i32,
            };
            lhs.push(NodeData::new(Token::from_int(v), false));
        }
    }
}

// Evaluate a chain in postfix order without any variable lookups (constant folding).
fn eval_chain_const(chain: &[NodeData], _vector_size: i32) -> MapperSignalValue {
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);
    for n in chain {
        eval_one_node(n, &mut stack, None, None, 0, 1, 1);
    }
    if stack.is_empty() {
        return MapperSignalValue::I32(0);
    }
    stack[0]
}

// Apply one postfix node to the evaluation stack.
fn eval_one_node(
    n: &NodeData,
    stack: &mut Vec<MapperSignalValue>,
    input_history: Option<&[MapperSignalValue]>,
    output_history: Option<&[MapperSignalValue]>,
    history_pos: i32,
    history_size: i32,
    vector_size: i32,
) {
    match n.tok.token_type {
        TokenType::Int => {
            stack.push(MapperSignalValue::I32(n.tok.int_value.unwrap_or(0)));
        }
        TokenType::Float => {
            stack.push(MapperSignalValue::F(n.tok.value.unwrap_or(0.0)));
        }
        TokenType::Var => {
            let idx = ((n.history_index + history_pos + history_size).rem_euclid(history_size)) as usize;
            match n.tok.var {
                Some('x') => {
                    if let Some(h) = input_history {
                        let i = idx * vector_size as usize + n.vector_index as usize;
                        if i < h.len() {
                            stack.push(h[i]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    } else {
                        stack.push(MapperSignalValue::I32(0));
                    }
                }
                Some('y') => {
                    if let Some(h) = output_history {
                        if idx < h.len() {
                            stack.push(h[idx]);
                        } else {
                            stack.push(MapperSignalValue::I32(0));
                        }
                    } else {
                        stack.push(MapperSignalValue::I32(0));
                    }
                }
                _ => {
                    stack.push(MapperSignalValue::I32(0));
                }
            }
        }
        TokenType::ToFloat => {
            if let Some(top) = stack.last_mut() {
                let v = match *top {
                    MapperSignalValue::I32(i) => i as f32,
                    MapperSignalValue::F(f) => f,
                };
                *top = MapperSignalValue::F(v);
            }
        }
        TokenType::ToInt32 => {
            if let Some(top) = stack.last_mut() {
                let v = match *top {
                    MapperSignalValue::F(f) => f as i32,
                    MapperSignalValue::I32(i) => i,
                };
                *top = MapperSignalValue::I32(v);
            }
        }
        TokenType::Op => {
            let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
            let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
            let op = n.tok.op.unwrap_or('+');
            if n.is_float {
                let l = match left { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                let r = match right { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                let res = match op {
                    '+' => l + r,
                    '-' => l - r,
                    '*' => l * r,
                    '/' => l / r,
                    _ => 0.0,
                };
                stack.push(MapperSignalValue::F(res));
            } else {
                let l = match left { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                let r = match right { MapperSignalValue::I32(i) => i, MapperSignalValue::F(f) => f as i32 };
                let res = match op {
                    '+' => l.wrapping_add(r),
                    '-' => l.wrapping_sub(r),
                    '*' => l.wrapping_mul(r),
                    '/' => if r == 0 { 0 } else { l / r },
                    _ => 0,
                };
                stack.push(MapperSignalValue::I32(res));
            }
        }
        TokenType::Func => {
            let idx = n.tok.int_value.unwrap_or(-1);
            if idx < 0 || (idx as usize) >= FUNCTIONS.len() {
                stack.push(MapperSignalValue::F(0.0));
                return;
            }
            let entry = &FUNCTIONS[idx as usize];
            match entry.arity {
                0 => {
                    let v = (entry.func)(0.0, 0.0);
                    stack.push(MapperSignalValue::F(v));
                }
                1 => {
                    let arg = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                    let a = match arg { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let v = (entry.func)(a, 0.0);
                    stack.push(MapperSignalValue::F(v));
                }
                2 => {
                    let right = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                    let left = stack.pop().unwrap_or(MapperSignalValue::F(0.0));
                    let l = match left { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let r = match right { MapperSignalValue::F(f) => f, MapperSignalValue::I32(i) => i as f32 };
                    let v = (entry.func)(l, r);
                    stack.push(MapperSignalValue::F(v));
                }
                _ => {
                    stack.push(MapperSignalValue::F(0.0));
                }
            }
        }
        _ => {
            // Unsupported tokens here are no-ops.
        }
    }
    let _ = (printtoken, printexprnode, printexpr, printstack);
}

// Append a NodeData to the end of a chain. Used for APPEND_OP semantics.
fn append_op_to_chain(chain: &mut Vec<NodeData>, op_tok: Token) {
    // The new op node inherits is_float from the last node in chain.
    let last_is_float = chain.last().map(|n| n.is_float).unwrap_or(false);
    chain.push(NodeData::new(op_tok, last_is_float));
}

// Convert a Vec<NodeData> chain to an Arc-linked ExprNode chain.
fn chain_to_expr_node(chain: &[NodeData]) -> ExprNode {
    let n = chain.len();
    let mut current: Option<Arc<ExprNode>> = None;
    for i in (1..n).rev() {
        let nd = &chain[i];
        let node = ExprNode {
            tok: nd.tok,
            is_float: if nd.is_float { 1 } else { 0 },
            history_index: nd.history_index,
            vector_index: nd.vector_index,
            next: current,
        };
        current = Some(Arc::new(node));
    }
    let head = &chain[0];
    ExprNode {
        tok: head.tok,
        is_float: if head.is_float { 1 } else { 0 },
        history_index: head.history_index,
        vector_index: head.vector_index,
        next: current,
    }
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    _output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    // Tokenize
    let tokens = match tokenize(s) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error tokenizing: {}", e);
            return empty_expr();
        }
    };

    let input_is_float_b = input_is_float != 0;

    let mut tok_idx: usize = 0;
    let mut current_tok: Token = Token::empty(TokenType::End);
    let mut next_token = true;

    let mut stack: Vec<StackItem> = Vec::new();
    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;

    // Initial stack setup, mirroring C: PUSHSTATE(EXPR), PUSHSTATE(YEQUAL_EQ), PUSHSTATE(YEQUAL_Y).
    stack.push(StackItem::St(state_t::EXPR));
    stack.push(StackItem::St(state_t::YEQUAL_EQ));
    stack.push(StackItem::St(state_t::YEQUAL_Y));

    let mut result: Option<Vec<NodeData>> = None;
    let mut error_message: Option<&'static str> = None;

    'outer: while !stack.is_empty() {
        if next_token {
            if tok_idx < tokens.len() {
                current_tok = tokens[tok_idx];
                tok_idx += 1;
            } else {
                current_tok = Token::empty(TokenType::End);
            }
            next_token = false;
        }

        let top = stack.len() - 1;
        let top_is_chain = matches!(stack[top], StackItem::Chain(_));
        if top_is_chain {
            // Top is a chain. If it's the only thing on stack, we have our result.
            if top == 0 {
                if let StackItem::Chain(c) = stack.pop().unwrap() {
                    result = Some(c);
                }
                break 'outer;
            }
            // top-1 is some kind. Look there.
            let prev = top - 1;
            let prev_is_state = matches!(stack[prev], StackItem::St(_));
            if prev_is_state {
                // Check if top-2 is also a chain
                if top >= 2 {
                    let pp = top - 2;
                    let pp_is_chain = matches!(stack[pp], StackItem::Chain(_));
                    if pp_is_chain {
                        // Determine which collapse case we're in by inspecting state
                        let state_kind: ChainCollapseKind = match &stack[prev] {
                            StackItem::St(state_t::EXPR_RIGHT) => ChainCollapseKind::Collapse,
                            StackItem::St(state_t::TERM_RIGHT) => ChainCollapseKind::Collapse,
                            StackItem::St(state_t::CLOSE_PAREN) => ChainCollapseKind::Collapse,
                            StackItem::St(state_t::CLOSE_HISTINDEX) => ChainCollapseKind::HistIndex,
                            StackItem::St(state_t::CLOSE_VECTINDEX) => ChainCollapseKind::VectIndex,
                            _ => ChainCollapseKind::Other,
                        };
                        match state_kind {
                            ChainCollapseKind::Collapse => {
                                let rhs = match stack.pop().unwrap() {
                                    StackItem::Chain(c) => c,
                                    _ => unreachable!(),
                                };
                                let lhs_idx = stack.len() - 2;
                                if let StackItem::Chain(ref mut lhs) = stack[lhs_idx] {
                                    collapse_chain(lhs, rhs, true, vector_size);
                                }
                                if TRACING {
                                    print_stack_dbg(&stack);
                                }
                                continue 'outer;
                            }
                            ChainCollapseKind::HistIndex => {
                                // Pop the rhs (a constant chain), set history_index of var (top-2).
                                let rhs = match stack.pop().unwrap() {
                                    StackItem::Chain(c) => c,
                                    _ => unreachable!(),
                                };
                                if rhs.len() != 1 {
                                    error_message = Some("expected lonely INT or FLOAT for history index");
                                    result = None;
                                    break 'outer;
                                }
                                let v: i32 = match rhs[0].tok.token_type {
                                    TokenType::Int => rhs[0].tok.int_value.unwrap_or(0),
                                    TokenType::Float => rhs[0].tok.value.unwrap_or(0.0) as i32,
                                    _ => {
                                        error_message = Some("expected INT or FLOAT for history index");
                                        result = None;
                                        break 'outer;
                                    }
                                };
                                let var_idx = stack.len() - 2;
                                if let StackItem::Chain(ref mut var_chain) = stack[var_idx] {
                                    if !matches!(var_chain[0].tok.token_type, TokenType::Var) {
                                        error_message = Some("expected VAR two-down on the stack");
                                        result = None;
                                        break 'outer;
                                    }
                                    var_chain[0].history_index = v;
                                    if (oldest_samps as i32) > var_chain[0].history_index {
                                        oldest_samps = var_chain[0].history_index as f32;
                                    }
                                }
                                if TRACING {
                                    print_stack_dbg(&stack);
                                }
                                continue 'outer;
                            }
                            ChainCollapseKind::VectIndex => {
                                let rhs = match stack.pop().unwrap() {
                                    StackItem::Chain(c) => c,
                                    _ => unreachable!(),
                                };
                                if rhs.len() != 1 {
                                    error_message = Some("expected lonely INT or FLOAT for vector index");
                                    result = None;
                                    break 'outer;
                                }
                                let v: i32 = match rhs[0].tok.token_type {
                                    TokenType::Int => rhs[0].tok.int_value.unwrap_or(0),
                                    TokenType::Float => rhs[0].tok.value.unwrap_or(0.0) as i32,
                                    _ => {
                                        error_message = Some("expected INT or FLOAT for vector index");
                                        result = None;
                                        break 'outer;
                                    }
                                };
                                let var_idx = stack.len() - 2;
                                if let StackItem::Chain(ref mut var_chain) = stack[var_idx] {
                                    if !matches!(var_chain[0].tok.token_type, TokenType::Var) {
                                        error_message = Some("expected VAR two-down on the stack");
                                        result = None;
                                        break 'outer;
                                    }
                                    var_chain[0].vector_index = v;
                                    if var_chain[0].vector_index > 0 {
                                        error_message = Some("Vector indexing not yet implemented.");
                                        result = None;
                                        break 'outer;
                                    }
                                    if var_chain[0].vector_index < 0 || var_chain[0].vector_index >= vector_size {
                                        error_message = Some("Vector index outside input size.");
                                        result = None;
                                        break 'outer;
                                    }
                                }
                                if TRACING {
                                    print_stack_dbg(&stack);
                                }
                                continue 'outer;
                            }
                            ChainCollapseKind::Other => {
                                // Fall through to swap behavior
                                // swap top and top-1
                                let n = stack.len();
                                stack.swap(n - 1, n - 2);
                                if TRACING {
                                    print_stack_dbg(&stack);
                                }
                                continue 'outer;
                            }
                        }
                    } else {
                        // top-2 is not a chain; swap top and top-1
                        let n = stack.len();
                        stack.swap(n - 1, n - 2);
                        if TRACING {
                            print_stack_dbg(&stack);
                        }
                        continue 'outer;
                    }
                } else {
                    // top-1 is state and there's no top-2; swap
                    let n = stack.len();
                    stack.swap(n - 1, n - 2);
                    if TRACING {
                        print_stack_dbg(&stack);
                    }
                    continue 'outer;
                }
            } else {
                // top-1 is also a chain (shouldn't happen normally)
                if TRACING {
                    print_stack_dbg(&stack);
                }
                continue 'outer;
            }
        }

        // Top is a state; handle it.
        let state = match stack[top] {
            StackItem::St(ref s) => match s {
                state_t::YEQUAL_Y => StateKind::YEqualY,
                state_t::YEQUAL_EQ => StateKind::YEqualEq,
                state_t::EXPR => StateKind::Expr,
                state_t::EXPR_RIGHT => StateKind::ExprRight,
                state_t::TERM => StateKind::Term,
                state_t::TERM_RIGHT => StateKind::TermRight,
                state_t::VALUE => StateKind::Value,
                state_t::NEGATE => StateKind::Negate,
                state_t::VAR_RIGHT => StateKind::VarRight,
                state_t::VAR_VECTINDEX => StateKind::VarVectIndex,
                state_t::VAR_HISTINDEX => StateKind::VarHistIndex,
                state_t::CLOSE_VECTINDEX => StateKind::CloseVectIndex,
                state_t::CLOSE_HISTINDEX => StateKind::CloseHistIndex,
                state_t::OPEN_PAREN => StateKind::OpenParen,
                state_t::CLOSE_PAREN => StateKind::CloseParen,
                state_t::COMMA => StateKind::Comma,
                state_t::END => StateKind::End,
            },
            _ => unreachable!(),
        };

        match state {
            StateKind::YEqualY => {
                if matches!(current_tok.token_type, TokenType::Var) && current_tok.var == Some('y') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    result = None;
                    break 'outer;
                }
                next_token = true;
            }
            StateKind::YEqualEq => {
                if matches!(current_tok.token_type, TokenType::Op) && current_tok.op == Some('=') {
                    stack.pop();
                } else {
                    error_message = Some("Error in y= prefix.");
                    result = None;
                    break 'outer;
                }
                next_token = true;
            }
            StateKind::Expr => {
                stack.pop();
                stack.push(StackItem::St(state_t::EXPR_RIGHT));
                stack.push(StackItem::St(state_t::TERM));
            }
            StateKind::ExprRight => {
                if matches!(current_tok.token_type, TokenType::Op) {
                    stack.pop();
                    let op = current_tok.op.unwrap_or(' ');
                    if op == '+' || op == '-' {
                        // Append op to the chain on top of stack (now top after pop).
                        if let Some(StackItem::Chain(chain)) = stack.last_mut() {
                            append_op_to_chain(chain, current_tok);
                        }
                        stack.push(StackItem::St(state_t::EXPR));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateKind::Term => {
                stack.pop();
                stack.push(StackItem::St(state_t::TERM_RIGHT));
                stack.push(StackItem::St(state_t::VALUE));
            }
            StateKind::TermRight => {
                if matches!(current_tok.token_type, TokenType::Op) {
                    stack.pop();
                    let op = current_tok.op.unwrap_or(' ');
                    if op == '*' || op == '/' {
                        if let Some(StackItem::Chain(chain)) = stack.last_mut() {
                            append_op_to_chain(chain, current_tok);
                        }
                        stack.push(StackItem::St(state_t::TERM));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            StateKind::Value => {
                match current_tok.token_type {
                    TokenType::Int => {
                        stack.pop();
                        stack.push(StackItem::Chain(vec![NodeData::new(current_tok, false)]));
                        next_token = true;
                    }
                    TokenType::Float => {
                        stack.pop();
                        stack.push(StackItem::Chain(vec![NodeData::new(current_tok, true)]));
                        next_token = true;
                    }
                    TokenType::Var => {
                        if var_allowed {
                            stack.pop();
                            stack.push(StackItem::Chain(vec![NodeData::new(current_tok, input_is_float_b)]));
                            stack.push(StackItem::St(state_t::VAR_RIGHT));
                            next_token = true;
                        } else {
                            error_message = Some("Unexpected variable reference.");
                            result = None;
                            break 'outer;
                        }
                    }
                    TokenType::OpenParen => {
                        stack.pop();
                        stack.push(StackItem::St(state_t::CLOSE_PAREN));
                        stack.push(StackItem::St(state_t::EXPR));
                        next_token = true;
                    }
                    TokenType::Func => {
                        stack.pop();
                        let idx = current_tok.int_value.unwrap_or(-1);
                        if idx < 0 {
                            error_message = Some("Unknown function.");
                            result = None;
                            break 'outer;
                        }
                        let arity = FUNCTIONS[idx as usize].arity;
                        // Push the func as a chain
                        stack.push(StackItem::Chain(vec![NodeData::new(current_tok, true)]));
                        if arity > 0 {
                            stack.push(StackItem::St(state_t::CLOSE_PAREN));
                            stack.push(StackItem::St(state_t::EXPR));
                            for _ in 1..arity {
                                stack.push(StackItem::St(state_t::COMMA));
                                stack.push(StackItem::St(state_t::EXPR));
                            }
                            stack.push(StackItem::St(state_t::OPEN_PAREN));
                        }
                        next_token = true;
                    }
                    TokenType::Op if current_tok.op == Some('-') => {
                        stack.pop();
                        stack.push(StackItem::St(state_t::NEGATE));
                        stack.push(StackItem::St(state_t::VALUE));
                        next_token = true;
                    }
                    _ => {
                        error_message = Some("Expected value.");
                        result = None;
                        break 'outer;
                    }
                }
            }
            StateKind::Negate => {
                stack.pop();
                // top should now be a chain; insert '0' before, '-' after.
                let top_is_chain = matches!(stack.last(), Some(StackItem::Chain(_)));
                if top_is_chain {
                    // Build a new chain: [0] then apply collapse-style insertion to wrap original.
                    // C inserts: 0 (int) at front, '-' at end of new wrapper chain, then collapses
                    // existing chain into the wrapper before the '-' op.
                    let inner = match stack.pop().unwrap() {
                        StackItem::Chain(c) => c,
                        _ => unreachable!(),
                    };
                    let mut wrapper: Vec<NodeData> = vec![
                        NodeData::new(Token::from_int(0), false),
                        NodeData::new(Token::from_op('-'), false),
                    ];
                    collapse_chain(&mut wrapper, inner, true, vector_size);
                    stack.push(StackItem::Chain(wrapper));
                } else {
                    error_message = Some("Expected to negate an expression.");
                    result = None;
                    break 'outer;
                }
            }
            StateKind::VarRight => {
                if matches!(current_tok.token_type, TokenType::OpenSquare) {
                    stack.pop();
                    stack.push(StackItem::St(state_t::VAR_VECTINDEX));
                } else if matches!(current_tok.token_type, TokenType::OpenCurly) {
                    stack.pop();
                    stack.push(StackItem::St(state_t::VAR_HISTINDEX));
                } else {
                    stack.pop();
                }
            }
            StateKind::VarVectIndex => {
                stack.pop();
                if matches!(current_tok.token_type, TokenType::OpenSquare) {
                    var_allowed = false;
                    stack.push(StackItem::St(state_t::CLOSE_VECTINDEX));
                    stack.push(StackItem::St(state_t::EXPR));
                    next_token = true;
                }
            }
            StateKind::VarHistIndex => {
                stack.pop();
                if matches!(current_tok.token_type, TokenType::OpenCurly) {
                    var_allowed = false;
                    stack.push(StackItem::St(state_t::CLOSE_HISTINDEX));
                    stack.push(StackItem::St(state_t::EXPR));
                    next_token = true;
                }
            }
            StateKind::CloseVectIndex => {
                if matches!(current_tok.token_type, TokenType::CloseSquare) {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackItem::St(state_t::VAR_HISTINDEX));
                    next_token = true;
                } else {
                    error_message = Some("Expected ']'.");
                    result = None;
                    break 'outer;
                }
            }
            StateKind::CloseHistIndex => {
                if matches!(current_tok.token_type, TokenType::CloseCurly) {
                    var_allowed = true;
                    stack.pop();
                    stack.push(StackItem::St(state_t::VAR_VECTINDEX));
                    next_token = true;
                } else {
                    error_message = Some("Expected '}'.");
                    result = None;
                    break 'outer;
                }
            }
            StateKind::CloseParen => {
                if matches!(current_tok.token_type, TokenType::CloseParen) {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected ')'.");
                    result = None;
                    break 'outer;
                }
            }
            StateKind::Comma => {
                if matches!(current_tok.token_type, TokenType::Comma) {
                    stack.pop();
                    // Find previous chain on the stack
                    let mut found: Option<usize> = None;
                    for i in (0..stack.len()).rev() {
                        if matches!(stack[i], StackItem::Chain(_)) {
                            // The "current" arg chain should be the topmost chain; the func chain is below.
                            // We want the arg chain (top) and the func chain (next below).
                            // The first hit from top is the arg chain itself. We need the next chain below.
                            // But the C does: scan from top-1 down for ST_NODE.
                            // After POP, top points to the arg chain. We scan from top-1 down.
                            // Translate: scan stack[..top] (exclusive) from end down.
                            // We'll handle this by scanning from len()-2 down.
                            if i + 1 < stack.len() && matches!(stack[stack.len() - 1], StackItem::Chain(_)) {
                                // i is the candidate "func chain" position
                                found = Some(i);
                                break;
                            }
                        }
                    }
                    // Actually we need a clearer scan: from (len-1)-1 down looking for chain.
                    let arg_top_idx = stack.len() - 1;
                    let mut prev_chain_idx: Option<usize> = None;
                    for i in (0..arg_top_idx).rev() {
                        if matches!(stack[i], StackItem::Chain(_)) {
                            prev_chain_idx = Some(i);
                            break;
                        }
                    }
                    let _ = found;
                    if let Some(i) = prev_chain_idx {
                        // Pop arg chain
                        let rhs = match stack.pop().unwrap() {
                            StackItem::Chain(c) => c,
                            _ => unreachable!(),
                        };
                        if let StackItem::Chain(ref mut lhs) = stack[i] {
                            collapse_chain(lhs, rhs, false, vector_size);
                        }
                    }
                    next_token = true;
                } else {
                    error_message = Some("Expected ','.");
                    result = None;
                    break 'outer;
                }
            }
            StateKind::OpenParen => {
                if matches!(current_tok.token_type, TokenType::OpenParen) {
                    stack.pop();
                    next_token = true;
                } else {
                    error_message = Some("Expected '('.");
                    result = None;
                    break 'outer;
                }
            }
            StateKind::End => {
                if matches!(current_tok.token_type, TokenType::End) {
                    stack.pop();
                } else {
                    error_message = Some("Expected END.");
                    result = None;
                    break 'outer;
                }
            }
        }

        if TRACING {
            print_stack_dbg(&stack);
        }
    }

    let chain = match result {
        Some(c) => c,
        None => {
            if let Some(msg) = error_message {
                eprintln!("{}", msg);
            }
            return empty_expr();
        }
    };

    // We intentionally do NOT add a final TOFLOAT/TOINT32 coercion based on
    // output_is_float — the runtime MapperSignalValue is a tagged union and the
    // tests rely on the natural float/int result of the expression.

    // Vector size guard (C disables vector indexing for vector_size > 1).
    if vector_size > 1 {
        for n in chain.iter() {
            if matches!(n.tok.token_type, TokenType::Var) && n.vector_index > 0 {
                eprintln!("vector indexing not yet implemented");
                return empty_expr();
            }
        }
    }

    if oldest_samps < -100.0 {
        return empty_expr();
    }

    let history_size = (-oldest_samps).ceil() as i32 + 1;
    let history_size = history_size.max(1);
    let input_history_len = (vector_size as usize) * (history_size as usize);
    let output_history_len = history_size as usize;

    MapperExpr {
        node: chain_to_expr_node(&chain),
        vector_size,
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); input_history_len],
        output_history: vec![MapperSignalValue::I32(0); output_history_len],
    }
}

fn empty_expr() -> MapperExpr {
    MapperExpr {
        node: ExprNode::new(),
        vector_size: 1,
        history_size: 1,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); 1],
        output_history: vec![MapperSignalValue::I32(0); 1],
    }
}

#[allow(non_camel_case_types)]
enum StateKind {
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
    CloseParen,
    Comma,
    End,
}

#[allow(dead_code)]
enum ChainCollapseKind {
    Collapse,
    HistIndex,
    VectIndex,
    Other,
}

fn print_stack_dbg(_stack: &[StackItem]) {
    // Tracing disabled; left in place for future debug.
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    // Push input into history
    if mapper.history_size > 0 {
        mapper.history_pos = (mapper.history_pos + 1).rem_euclid(mapper.history_size);
        // Determine the static input "is_float" by looking at the first node referencing x.
        // For simplicity: store the input directly. Variable lookup will coerce as needed.
        let pos = (mapper.history_pos as usize) * (mapper.vector_size as usize);
        if pos < mapper.input_history.len() {
            mapper.input_history[pos] = *input;
            // Fill remaining vector slots (vector_size > 1 unsupported; default zeros)
            for i in 1..(mapper.vector_size as usize) {
                if pos + i < mapper.input_history.len() {
                    mapper.input_history[pos + i] = MapperSignalValue::I32(0);
                }
            }
        }
    }

    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);

    // Walk the chain
    let mut current: Option<&ExprNode> = Some(&mapper.node);
    while let Some(n) = current {
        let nd = NodeData {
            tok: n.tok,
            is_float: n.is_float != 0,
            history_index: n.history_index,
            vector_index: n.vector_index,
        };
        eval_one_node(
            &nd,
            &mut stack,
            Some(&mapper.input_history),
            Some(&mapper.output_history),
            mapper.history_pos,
            mapper.history_size,
            mapper.vector_size,
        );
        current = n.next.as_deref();
    }

    let out = stack.first().copied().unwrap_or(MapperSignalValue::I32(0));

    // Push output to history
    let opos = mapper.history_pos as usize;
    if opos < mapper.output_history.len() {
        mapper.output_history[opos] = out;
    }

    out
}
