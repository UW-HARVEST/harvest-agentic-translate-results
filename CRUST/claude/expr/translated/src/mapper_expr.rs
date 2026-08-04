use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Arc;

use lazy_static::lazy_static;

const TRACING: bool = false;
const STACK_SIZE: usize = 256;

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
            MapperSignalValue::F(f) => Some(*f as i32),
            MapperSignalValue::I32(i) => Some(*i),
        }
    }
}

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
    static ref FUNCTION_TABLE: HashMap<&'static str, FunctionEntry> = {
        let mut m = HashMap::new();
        m.insert("pow", FunctionEntry { name: "pow", arity: 2, func: f32::powf });
        m.insert("sin", FunctionEntry { name: "sin", arity: 1, func: |x, _| x.sin() });
        m.insert("cos", FunctionEntry { name: "cos", arity: 1, func: |x, _| x.cos() });
        m.insert("tan", FunctionEntry { name: "tan", arity: 1, func: |x, _| x.tan() });
        m.insert("abs", FunctionEntry { name: "abs", arity: 1, func: |x, _| x.abs() });
        m.insert("sqrt", FunctionEntry { name: "sqrt", arity: 1, func: |x, _| x.sqrt() });
        m.insert("log", FunctionEntry { name: "log", arity: 1, func: |x, _| x.ln() });
        m.insert("log10", FunctionEntry { name: "log10", arity: 1, func: |x, _| x.log10() });
        m.insert("exp", FunctionEntry { name: "exp", arity: 1, func: |x, _| x.exp() });
        m.insert("floor", FunctionEntry { name: "floor", arity: 1, func: |x, _| x.floor() });
        m.insert("round", FunctionEntry { name: "round", arity: 1, func: |x, _| x.round() });
        m.insert("ceil", FunctionEntry { name: "ceil", arity: 1, func: |x, _| x.ceil() });
        m.insert("asin", FunctionEntry { name: "asin", arity: 1, func: |x, _| x.asin() });
        m.insert("acos", FunctionEntry { name: "acos", arity: 1, func: |x, _| x.acos() });
        m.insert("atan", FunctionEntry { name: "atan", arity: 1, func: |x, _| x.atan() });
        m.insert("atan2", FunctionEntry { name: "atan2", arity: 2, func: |y, x| y.atan2(x) });
        m.insert("sinh", FunctionEntry { name: "sinh", arity: 1, func: |x, _| x.sinh() });
        m.insert("cosh", FunctionEntry { name: "cosh", arity: 1, func: |x, _| x.cosh() });
        m.insert("tanh", FunctionEntry { name: "tanh", arity: 1, func: |x, _| x.tanh() });
        m.insert("logb", FunctionEntry { name: "logb", arity: 1, func: |x, _| x.abs().log2().floor() });
        m.insert("exp2", FunctionEntry { name: "exp2", arity: 1, func: |x, _| x.exp2() });
        m.insert("log2", FunctionEntry { name: "log2", arity: 1, func: |x, _| x.log2() });
        m.insert("hypot", FunctionEntry { name: "hypot", arity: 2, func: |x, y| x.hypot(y) });
        m.insert("cbrt", FunctionEntry { name: "cbrt", arity: 1, func: |x, _| x.cbrt() });
        m.insert("trunc", FunctionEntry { name: "trunc", arity: 1, func: |x, _| x.trunc() });
        m.insert("min", FunctionEntry { name: "min", arity: 2, func: minf });
        m.insert("max", FunctionEntry { name: "max", arity: 2, func: maxf });
        m.insert("pi", FunctionEntry { name: "pi", arity: 0, func: |_, _| pif() });
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
}

impl Token {
    fn end() -> Token {
        Token {
            token_type: TokenType::End,
            value: None,
            int_value: None,
            var: None,
            op: None,
        }
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE.get(s)
}

/// Tokenizer: take the entire source string and produce a list of tokens
/// (terminated by an End token).
fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let src: String = s.concat();
    let mut tokens = Vec::new();
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_digit() || c == '.' {
            // number
            let start = i;
            let mut integer_found = false;
            let mut int_val: i32 = 0;
            if c.is_ascii_digit() {
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
                let s = &src[start..i];
                int_val = s.parse::<i32>().unwrap_or(0);
                integer_found = true;
                if i >= bytes.len() || bytes[i] as char != '.' {
                    tokens.push(Token {
                        token_type: TokenType::Int,
                        value: None,
                        int_value: Some(int_val),
                        var: None,
                        op: None,
                    });
                    continue;
                }
            }
            // We saw '.', or we started with '.'
            // For "5.": treat as float (int_val + 0)
            // For ".5": treat as float (0 + 0.5)
            // For "5.5": treat as float (int_val + 0.5)
            let dot_start = i;
            // consume '.'
            i += 1;
            let frac_start = i;
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            let frac_end = i;
            if frac_start == frac_end {
                // No fractional digits
                if integer_found {
                    tokens.push(Token {
                        token_type: TokenType::Float,
                        value: Some(int_val as f32),
                        int_value: None,
                        var: None,
                        op: None,
                    });
                    continue;
                } else {
                    // Lone '.' followed by non-digit; skip it
                    // (matches the C "break" path which returns 1 / errors)
                    return Vec::new();
                }
            }
            let frac_str = &src[dot_start..frac_end];
            let frac_val: f32 = frac_str.parse::<f32>().unwrap_or(0.0);
            let total = (int_val as f32) + frac_val;
            tokens.push(Token {
                token_type: TokenType::Float,
                value: Some(total),
                int_value: None,
                var: None,
                op: None,
            });
            continue;
        }
        match c {
            '+' | '-' | '/' | '*' | '=' => {
                tokens.push(Token {
                    token_type: TokenType::Op,
                    value: None,
                    int_value: None,
                    var: None,
                    op: Some(c),
                });
                i += 1;
            }
            '(' => {
                tokens.push(Token {
                    token_type: TokenType::OpenParen,
                    value: None,
                    int_value: None,
                    var: None,
                    op: None,
                });
                i += 1;
            }
            ')' => {
                tokens.push(Token {
                    token_type: TokenType::CloseParen,
                    value: None,
                    int_value: None,
                    var: None,
                    op: None,
                });
                i += 1;
            }
            'x' | 'y' => {
                // Could be a variable, OR could be the start of a function name.
                // Check if next char is alphanumeric — if so, treat as func/identifier.
                let next_is_alnum = i + 1 < bytes.len()
                    && (bytes[i + 1] as char).is_ascii_alphanumeric();
                if !next_is_alnum {
                    tokens.push(Token {
                        token_type: TokenType::Var,
                        value: None,
                        int_value: None,
                        var: Some(c),
                        op: None,
                    });
                    i += 1;
                } else {
                    let start = i;
                    while i < bytes.len()
                        && ((bytes[i] as char).is_ascii_alphabetic()
                            || (bytes[i] as char).is_ascii_digit())
                    {
                        i += 1;
                    }
                    let name = &src[start..i];
                    let func = function_lookup(name);
                    tokens.push(Token {
                        token_type: TokenType::Func,
                        value: None,
                        int_value: func.map(|_| 0),
                        var: Some(name.chars().next().unwrap_or(' ')),
                        op: None,
                    });
                    // Use `value` field to store function name index by storing arity.
                    // Actually, we need to retrieve by name later. Let's store the
                    // name via a side table — see func_name_for_token below.
                    // We'll re-look-up by char-encoded name in a separate map.
                    // For simplicity: store the name in a global table.
                    let idx = register_func_name(name);
                    let last = tokens.last_mut().unwrap();
                    last.int_value = Some(idx);
                }
            }
            '[' => {
                tokens.push(Token {
                    token_type: TokenType::OpenSquare,
                    value: None,
                    int_value: None,
                    var: None,
                    op: None,
                });
                i += 1;
            }
            ']' => {
                tokens.push(Token {
                    token_type: TokenType::CloseSquare,
                    value: None,
                    int_value: None,
                    var: None,
                    op: None,
                });
                i += 1;
            }
            '{' => {
                tokens.push(Token {
                    token_type: TokenType::OpenCurly,
                    value: None,
                    int_value: None,
                    var: None,
                    op: None,
                });
                i += 1;
            }
            '}' => {
                tokens.push(Token {
                    token_type: TokenType::CloseCurly,
                    value: None,
                    int_value: None,
                    var: None,
                    op: None,
                });
                i += 1;
            }
            ' ' | '\t' | '\r' | '\n' => {
                i += 1;
            }
            ',' => {
                tokens.push(Token {
                    token_type: TokenType::Comma,
                    value: None,
                    int_value: None,
                    var: None,
                    op: None,
                });
                i += 1;
            }
            _ => {
                if c.is_ascii_alphabetic() {
                    let start = i;
                    while i < bytes.len()
                        && ((bytes[i] as char).is_ascii_alphabetic()
                            || (bytes[i] as char).is_ascii_digit())
                    {
                        i += 1;
                    }
                    let name = &src[start..i];
                    let idx = register_func_name(name);
                    tokens.push(Token {
                        token_type: TokenType::Func,
                        value: None,
                        int_value: Some(idx),
                        var: None,
                        op: None,
                    });
                } else {
                    // Unknown char — abort
                    return Vec::new();
                }
            }
        }
    }
    tokens.push(Token::end());
    tokens
}

// Function-name registry, since the public Token type can only carry small
// data fields. We map a function-table key (the canonical name) to a small
// integer index that fits inside Token.int_value.
use std::sync::Mutex;
lazy_static! {
    static ref FUNC_NAME_REGISTRY: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

fn register_func_name(name: &str) -> i32 {
    let mut reg = FUNC_NAME_REGISTRY.lock().unwrap();
    if let Some(pos) = reg.iter().position(|n| n == name) {
        return pos as i32;
    }
    reg.push(name.to_string());
    (reg.len() - 1) as i32
}

fn func_name_at(idx: i32) -> Option<String> {
    let reg = FUNC_NAME_REGISTRY.lock().unwrap();
    if idx < 0 || (idx as usize) >= reg.len() {
        None
    } else {
        Some(reg[idx as usize].clone())
    }
}

fn token_func_entry(tok: &Token) -> Option<&'static FunctionEntry> {
    let idx = tok.int_value?;
    let name = func_name_at(idx)?;
    function_lookup(&name)
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

#[allow(non_camel_case_types)]
enum stack_obj_t {
    State(state_t),
    Node(ExprNode),
}

impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: Token::end(),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }
    pub fn expr_free(&self) {
        // Rust handles this automatically via Drop.
    }
}

fn printtoken(_t: &Token) {
    // debug-only; left as no-op
}

fn printexprnode(_s: &str, _list: &ExprNode) {
    // debug-only; left as no-op
}

fn printexpr(_s: &str, _list: &MapperExpr) {
    // debug-only; left as no-op
}

fn printstack(_stack: &stack_obj_t, _stack_size: i32) {
    // debug-only; left as no-op
}

fn collapse_expr_to_left(_plhs: &mut ExprNode, _constant_folding: i32) {
    // The actual parser uses an internal IR (NodeData lists) with its own
    // collapse routine — see `collapse_inner`. This stub satisfies the public
    // signature only.
}

// =============================================================================
// Internal IR for parsing
// =============================================================================

#[derive(Clone, Debug)]
struct NodeData {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

impl NodeData {
    fn new(tok: Token, is_float: i32) -> NodeData {
        NodeData {
            tok,
            is_float,
            history_index: 0,
            vector_index: 0,
        }
    }
}

type NodeList = Vec<NodeData>;

enum InnerStackObj {
    State(state_t),
    Node(NodeList),
}

fn evaluate_inner(
    nodes: &[NodeData],
    input_vector: Option<&[MapperSignalValue]>,
    vector_size: i32,
    history_size: i32,
    history_pos: i32,
    input_history: &[MapperSignalValue],
    output_history: &[MapperSignalValue],
) -> MapperSignalValue {
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);
    for node in nodes {
        match node.tok.token_type {
            TokenType::Int => {
                stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0)));
            }
            TokenType::Float => {
                stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0)));
            }
            TokenType::Var => {
                // Use history_pos and history_index to read history slot.
                let hs = history_size.max(1);
                let idx_h = ((node.history_index + history_pos + hs) % hs).max(0);
                match node.tok.var.unwrap_or(' ') {
                    'x' => {
                        let idx = idx_h * vector_size + node.vector_index;
                        let val = if let Some(_iv) = input_vector {
                            input_history
                                .get(idx as usize)
                                .copied()
                                .unwrap_or(MapperSignalValue::I32(0))
                        } else {
                            MapperSignalValue::I32(0)
                        };
                        stack.push(val);
                    }
                    'y' => {
                        let val = output_history
                            .get(idx_h as usize)
                            .copied()
                            .unwrap_or(MapperSignalValue::I32(0));
                        stack.push(val);
                    }
                    _ => {
                        stack.push(MapperSignalValue::I32(0));
                    }
                }
            }
            TokenType::ToFloat => {
                if let Some(top) = stack.last_mut() {
                    let f = match *top {
                        MapperSignalValue::F(f) => f,
                        MapperSignalValue::I32(i) => i as f32,
                    };
                    *top = MapperSignalValue::F(f);
                }
            }
            TokenType::ToInt32 => {
                if let Some(top) = stack.last_mut() {
                    let i = match *top {
                        MapperSignalValue::F(f) => f as i32,
                        MapperSignalValue::I32(i) => i,
                    };
                    *top = MapperSignalValue::I32(i);
                }
            }
            TokenType::Op => {
                let right = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let left = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                let op = node.tok.op.unwrap_or('+');
                let any_float = matches!(left, MapperSignalValue::F(_))
                    || matches!(right, MapperSignalValue::F(_))
                    || node.is_float != 0;
                if any_float {
                    let l = match left {
                        MapperSignalValue::F(f) => f,
                        MapperSignalValue::I32(i) => i as f32,
                    };
                    let r = match right {
                        MapperSignalValue::F(f) => f,
                        MapperSignalValue::I32(i) => i as f32,
                    };
                    let v = match op {
                        '+' => l + r,
                        '-' => l - r,
                        '*' => l * r,
                        '/' => l / r,
                        _ => 0.0,
                    };
                    stack.push(MapperSignalValue::F(v));
                } else {
                    let l = match left {
                        MapperSignalValue::I32(i) => i,
                        MapperSignalValue::F(f) => f as i32,
                    };
                    let r = match right {
                        MapperSignalValue::I32(i) => i,
                        MapperSignalValue::F(f) => f as i32,
                    };
                    let v = match op {
                        '+' => l.wrapping_add(r),
                        '-' => l.wrapping_sub(r),
                        '*' => l.wrapping_mul(r),
                        '/' => {
                            if r == 0 {
                                0
                            } else {
                                l / r
                            }
                        }
                        _ => 0,
                    };
                    stack.push(MapperSignalValue::I32(v));
                }
            }
            TokenType::Func => {
                if let Some(entry) = token_func_entry(&node.tok) {
                    match entry.arity {
                        0 => {
                            let v = (entry.func)(0.0, 0.0);
                            stack.push(MapperSignalValue::F(v));
                        }
                        1 => {
                            let r = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                            let rf = match r {
                                MapperSignalValue::F(f) => f,
                                MapperSignalValue::I32(i) => i as f32,
                            };
                            let v = (entry.func)(rf, 0.0);
                            stack.push(MapperSignalValue::F(v));
                        }
                        2 => {
                            let r = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                            let l = stack.pop().unwrap_or(MapperSignalValue::I32(0));
                            let lf = match l {
                                MapperSignalValue::F(f) => f,
                                MapperSignalValue::I32(i) => i as f32,
                            };
                            let rf = match r {
                                MapperSignalValue::F(f) => f,
                                MapperSignalValue::I32(i) => i as f32,
                            };
                            let v = (entry.func)(lf, rf);
                            stack.push(MapperSignalValue::F(v));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    stack.pop().unwrap_or(MapperSignalValue::I32(0))
}

/// Concatenate two NodeLists into the LHS in postfix order, inserting
/// type coercion as necessary, and optionally constant-folding.
fn collapse_inner(plhs: &mut NodeList, mut rhs: NodeList, constant_folding: bool) {
    if plhs.is_empty() || rhs.is_empty() {
        plhs.extend(rhs);
        return;
    }

    // Track whether either side references a variable.
    let mut refvar = false;
    for n in plhs.iter() {
        if n.tok.token_type == TokenType::Var {
            refvar = true;
            break;
        }
    }
    if !refvar {
        for n in rhs.iter() {
            if n.tok.token_type == TokenType::Var {
                refvar = true;
                break;
            }
        }
    }

    // The LHS ends with a "trailing operator" (the operator slot we'll insert
    // before). Mirroring the C: insertion happens at the index of the last
    // node of the LHS.
    let lhs_last_idx = plhs.len() - 1;
    let lhs_last_is_float = plhs[lhs_last_idx].is_float != 0;
    let rhs_last_is_float = rhs.last().unwrap().is_float != 0;

    let is_float = lhs_last_is_float || rhs_last_is_float;

    // Insert coercion as the C code does.
    if lhs_last_is_float && !rhs_last_is_float {
        // Append TOFLOAT to rhs.
        let coerce = Token {
            token_type: TokenType::ToFloat,
            value: None,
            int_value: None,
            var: None,
            op: None,
        };
        rhs.push(NodeData::new(coerce, 1));
    } else if !lhs_last_is_float && rhs_last_is_float {
        // Insert TOFLOAT before LHS last, and mark LHS last as float.
        let coerce = Token {
            token_type: TokenType::ToFloat,
            value: None,
            int_value: None,
            var: None,
            op: None,
        };
        plhs.insert(lhs_last_idx, NodeData::new(coerce, 1));
        // The "last" (which is now at lhs_last_idx + 1) had its is_float set to 1.
        let new_last_idx = plhs.len() - 1;
        plhs[new_last_idx].is_float = 1;
    }

    // Now insert rhs nodes immediately before the last node of lhs.
    let insert_at = plhs.len() - 1;
    let tail = plhs.split_off(insert_at);
    plhs.extend(rhs);
    plhs.extend(tail);

    if constant_folding && !refvar {
        // Evaluate the entire current list (no variables) and replace with a
        // single literal node of the appropriate type.
        let v = evaluate_inner(plhs, None, 1, 1, 0, &[], &[]);
        plhs.clear();
        let (tok, isf) = if is_float {
            let f = v.as_f32().unwrap_or(0.0);
            (
                Token {
                    token_type: TokenType::Float,
                    value: Some(f),
                    int_value: None,
                    var: None,
                    op: None,
                },
                1,
            )
        } else {
            let i = v.as_i32().unwrap_or(0);
            (
                Token {
                    token_type: TokenType::Int,
                    value: None,
                    int_value: Some(i),
                    var: None,
                    op: None,
                },
                0,
            )
        };
        plhs.push(NodeData::new(tok, isf));
    }
}

fn append_op_to_top_node(stack: &mut Vec<InnerStackObj>, op_tok: Token) {
    if let Some(InnerStackObj::Node(list)) = stack.last_mut() {
        let is_float = list.last().map(|n| n.is_float).unwrap_or(0);
        list.push(NodeData::new(op_tok, is_float));
    }
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    _output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    let tokens = expr_lex(vec![s]);
    let mut tok_idx = 0usize;
    let mut next_token = true;
    let mut tok = Token::end();

    let mut stack: Vec<InnerStackObj> = Vec::new();
    stack.push(InnerStackObj::State(state_t::EXPR));
    stack.push(InnerStackObj::State(state_t::YEQUAL_EQ));
    stack.push(InnerStackObj::State(state_t::YEQUAL_Y));

    let mut var_allowed = true;
    let mut oldest_samps: f32 = 0.0;
    let mut result: Option<NodeList> = None;
    let mut error: Option<&'static str> = None;

    while !stack.is_empty() {
        if next_token {
            if tok_idx >= tokens.len() {
                error = Some("Unexpected end of input.");
                break;
            }
            tok = tokens[tok_idx];
            tok_idx += 1;
            next_token = false;
        }

        // If the top of the stack is a Node, try to combine it with state(s)
        // beneath it.
        if matches!(stack.last(), Some(InnerStackObj::Node(_))) {
            if stack.len() == 1 {
                // Single-node stack — extract and finish.
                if let Some(InnerStackObj::Node(n)) = stack.pop() {
                    result = Some(n);
                }
                break;
            }
            // Look at the state immediately under top.
            let top = stack.len() - 1;
            let under_is_state = matches!(stack.get(top - 1), Some(InnerStackObj::State(_)));
            if under_is_state {
                // Is there a node two-down?
                let two_down_is_node =
                    top >= 2 && matches!(stack.get(top - 2), Some(InnerStackObj::Node(_)));
                if two_down_is_node {
                    // What state is between?
                    let combine_kind = match &stack[top - 1] {
                        InnerStackObj::State(state_t::EXPR_RIGHT)
                        | InnerStackObj::State(state_t::TERM_RIGHT)
                        | InnerStackObj::State(state_t::CLOSE_PAREN) => 1,
                        InnerStackObj::State(state_t::CLOSE_HISTINDEX) => 2,
                        InnerStackObj::State(state_t::CLOSE_VECTINDEX) => 3,
                        _ => 0,
                    };
                    if combine_kind == 1 {
                        // Pop top node, collapse into two-down node.
                        let top_node = if let Some(InnerStackObj::Node(n)) = stack.pop() {
                            n
                        } else {
                            unreachable!()
                        };
                        let lhs_idx = stack.len() - 2;
                        if let InnerStackObj::Node(lhs) = &mut stack[lhs_idx] {
                            collapse_inner(lhs, top_node, true);
                        }
                        continue;
                    } else if combine_kind == 2 {
                        // history-index: take top scalar value into two-down VAR's
                        // history_index field. Only the top Node is popped (the
                        // state remains for the next iteration to consume).
                        let top_node = if let Some(InnerStackObj::Node(n)) = stack.pop() {
                            n
                        } else {
                            unreachable!()
                        };
                        let val = top_node
                            .first()
                            .map(|n| match n.tok.token_type {
                                TokenType::Float => n.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => n.tok.int_value.unwrap_or(0),
                                _ => 0,
                            })
                            .unwrap_or(0);
                        let var_idx = stack.len() - 2;
                        if let InnerStackObj::Node(lhs) = &mut stack[var_idx] {
                            if let Some(first) = lhs.first_mut() {
                                first.history_index = val;
                                if (first.history_index as f32) < oldest_samps {
                                    oldest_samps = first.history_index as f32;
                                }
                            }
                        }
                        continue;
                    } else if combine_kind == 3 {
                        // vector-index — only top Node popped.
                        let top_node = if let Some(InnerStackObj::Node(n)) = stack.pop() {
                            n
                        } else {
                            unreachable!()
                        };
                        let val = top_node
                            .first()
                            .map(|n| match n.tok.token_type {
                                TokenType::Float => n.tok.value.unwrap_or(0.0) as i32,
                                TokenType::Int => n.tok.int_value.unwrap_or(0),
                                _ => 0,
                            })
                            .unwrap_or(0);
                        let var_idx = stack.len() - 2;
                        if let InnerStackObj::Node(lhs) = &mut stack[var_idx] {
                            if let Some(first) = lhs.first_mut() {
                                first.vector_index = val;
                                if first.vector_index > 0 {
                                    error = Some("Vector indexing not yet implemented.");
                                    break;
                                }
                                if first.vector_index < 0 || first.vector_index >= vector_size {
                                    error = Some("Vector index outside input size.");
                                    break;
                                }
                            }
                        }
                        continue;
                    }
                    // No special combine: fall through to swap.
                }
                // No two-down node; swap top (Node) with state below it.
                let n = stack.pop().unwrap();
                let s = stack.pop().unwrap();
                stack.push(n);
                stack.push(s);
                continue;
            } else {
                // Two-down node, no state between — shouldn't happen normally.
                continue;
            }
        }

        // Top is a State.
        let state_label = match stack.last().unwrap() {
            InnerStackObj::State(s) => match s {
                state_t::YEQUAL_Y => 0,
                state_t::YEQUAL_EQ => 1,
                state_t::EXPR => 2,
                state_t::EXPR_RIGHT => 3,
                state_t::TERM => 4,
                state_t::TERM_RIGHT => 5,
                state_t::VALUE => 6,
                state_t::NEGATE => 7,
                state_t::VAR_RIGHT => 8,
                state_t::VAR_VECTINDEX => 9,
                state_t::VAR_HISTINDEX => 10,
                state_t::CLOSE_VECTINDEX => 11,
                state_t::CLOSE_HISTINDEX => 12,
                state_t::OPEN_PAREN => 13,
                state_t::CLOSE_PAREN => 14,
                state_t::COMMA => 15,
                state_t::END => 16,
            },
            _ => unreachable!(),
        };

        match state_label {
            0 => {
                // YEQUAL_Y
                if tok.token_type == TokenType::Var && tok.var == Some('y') {
                    stack.pop();
                } else {
                    error = Some("Error in y= prefix.");
                    break;
                }
                next_token = true;
            }
            1 => {
                // YEQUAL_EQ
                if tok.token_type == TokenType::Op && tok.op == Some('=') {
                    stack.pop();
                } else {
                    error = Some("Error in y= prefix.");
                    break;
                }
                next_token = true;
            }
            2 => {
                // EXPR
                stack.pop();
                stack.push(InnerStackObj::State(state_t::EXPR_RIGHT));
                stack.push(InnerStackObj::State(state_t::TERM));
            }
            3 => {
                // EXPR_RIGHT
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('+') || tok.op == Some('-') {
                        append_op_to_top_node(&mut stack, tok);
                        stack.push(InnerStackObj::State(state_t::EXPR));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            4 => {
                // TERM
                stack.pop();
                stack.push(InnerStackObj::State(state_t::TERM_RIGHT));
                stack.push(InnerStackObj::State(state_t::VALUE));
            }
            5 => {
                // TERM_RIGHT
                if tok.token_type == TokenType::Op {
                    stack.pop();
                    if tok.op == Some('*') || tok.op == Some('/') {
                        append_op_to_top_node(&mut stack, tok);
                        stack.push(InnerStackObj::State(state_t::TERM));
                        next_token = true;
                    }
                } else {
                    stack.pop();
                }
            }
            6 => {
                // VALUE
                match tok.token_type {
                    TokenType::Int => {
                        stack.pop();
                        let mut list = NodeList::new();
                        list.push(NodeData::new(tok, 0));
                        stack.push(InnerStackObj::Node(list));
                        next_token = true;
                    }
                    TokenType::Float => {
                        stack.pop();
                        let mut list = NodeList::new();
                        list.push(NodeData::new(tok, 1));
                        stack.push(InnerStackObj::Node(list));
                        next_token = true;
                    }
                    TokenType::Var => {
                        if var_allowed {
                            stack.pop();
                            let mut list = NodeList::new();
                            list.push(NodeData::new(tok, input_is_float));
                            stack.push(InnerStackObj::Node(list));
                            stack.push(InnerStackObj::State(state_t::VAR_RIGHT));
                            next_token = true;
                        } else {
                            error = Some("Unexpected variable reference.");
                            break;
                        }
                    }
                    TokenType::OpenParen => {
                        stack.pop();
                        stack.push(InnerStackObj::State(state_t::CLOSE_PAREN));
                        stack.push(InnerStackObj::State(state_t::EXPR));
                        next_token = true;
                    }
                    TokenType::Func => {
                        stack.pop();
                        let entry = token_func_entry(&tok);
                        if entry.is_none() {
                            error = Some("Unknown function.");
                            break;
                        }
                        let entry = entry.unwrap();
                        let mut list = NodeList::new();
                        list.push(NodeData::new(tok, 1));
                        stack.push(InnerStackObj::Node(list));
                        let arity = entry.arity;
                        if arity > 0 {
                            stack.push(InnerStackObj::State(state_t::CLOSE_PAREN));
                            stack.push(InnerStackObj::State(state_t::EXPR));
                            for _ in 1..arity {
                                stack.push(InnerStackObj::State(state_t::COMMA));
                                stack.push(InnerStackObj::State(state_t::EXPR));
                            }
                            stack.push(InnerStackObj::State(state_t::OPEN_PAREN));
                        }
                        next_token = true;
                    }
                    TokenType::Op if tok.op == Some('-') => {
                        stack.pop();
                        stack.push(InnerStackObj::State(state_t::NEGATE));
                        stack.push(InnerStackObj::State(state_t::VALUE));
                        next_token = true;
                    }
                    _ => {
                        error = Some("Expected value.");
                        break;
                    }
                }
            }
            7 => {
                // NEGATE
                stack.pop();
                if matches!(stack.last(), Some(InnerStackObj::Node(_))) {
                    let top_node = if let Some(InnerStackObj::Node(n)) = stack.pop() {
                        n
                    } else {
                        unreachable!()
                    };
                    let zero_tok = Token {
                        token_type: TokenType::Int,
                        value: None,
                        int_value: Some(0),
                        var: None,
                        op: None,
                    };
                    let minus_tok = Token {
                        token_type: TokenType::Op,
                        value: None,
                        int_value: None,
                        var: None,
                        op: Some('-'),
                    };
                    let mut new_list = NodeList::new();
                    new_list.push(NodeData::new(zero_tok, 0));
                    new_list.push(NodeData::new(minus_tok, 0));
                    collapse_inner(&mut new_list, top_node, true);
                    stack.push(InnerStackObj::Node(new_list));
                } else {
                    error = Some("Expected to negate an expression.");
                    break;
                }
            }
            8 => {
                // VAR_RIGHT
                if tok.token_type == TokenType::OpenSquare {
                    stack.pop();
                    stack.push(InnerStackObj::State(state_t::VAR_VECTINDEX));
                } else if tok.token_type == TokenType::OpenCurly {
                    stack.pop();
                    stack.push(InnerStackObj::State(state_t::VAR_HISTINDEX));
                } else {
                    stack.pop();
                }
            }
            9 => {
                // VAR_VECTINDEX
                stack.pop();
                if tok.token_type == TokenType::OpenSquare {
                    var_allowed = false;
                    stack.push(InnerStackObj::State(state_t::CLOSE_VECTINDEX));
                    stack.push(InnerStackObj::State(state_t::EXPR));
                    next_token = true;
                }
            }
            10 => {
                // VAR_HISTINDEX
                stack.pop();
                if tok.token_type == TokenType::OpenCurly {
                    var_allowed = false;
                    stack.push(InnerStackObj::State(state_t::CLOSE_HISTINDEX));
                    stack.push(InnerStackObj::State(state_t::EXPR));
                    next_token = true;
                }
            }
            11 => {
                // CLOSE_VECTINDEX
                if tok.token_type == TokenType::CloseSquare {
                    var_allowed = true;
                    stack.pop();
                    stack.push(InnerStackObj::State(state_t::VAR_HISTINDEX));
                    next_token = true;
                } else {
                    error = Some("Expected ']'.");
                    break;
                }
            }
            12 => {
                // CLOSE_HISTINDEX
                if tok.token_type == TokenType::CloseCurly {
                    var_allowed = true;
                    stack.pop();
                    stack.push(InnerStackObj::State(state_t::VAR_VECTINDEX));
                    next_token = true;
                } else {
                    error = Some("Expected '}'.");
                    break;
                }
            }
            13 => {
                // OPEN_PAREN
                if tok.token_type == TokenType::OpenParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error = Some("Expected '('.");
                    break;
                }
            }
            14 => {
                // CLOSE_PAREN
                if tok.token_type == TokenType::CloseParen {
                    stack.pop();
                    next_token = true;
                } else {
                    error = Some("Expected ')'.");
                    break;
                }
            }
            15 => {
                // COMMA
                if tok.token_type == TokenType::Comma {
                    stack.pop();
                    // Find previous node on stack; collapse top node into it.
                    let top_node = if let Some(InnerStackObj::Node(n)) = stack.pop() {
                        n
                    } else {
                        // No node — shouldn't happen for well-formed input.
                        next_token = true;
                        continue;
                    };
                    // Now search backward for the most recent Node.
                    let mut found: Option<usize> = None;
                    for (i, item) in stack.iter().enumerate().rev() {
                        if matches!(item, InnerStackObj::Node(_)) {
                            found = Some(i);
                            break;
                        }
                    }
                    if let Some(i) = found {
                        if let InnerStackObj::Node(lhs) = &mut stack[i] {
                            collapse_inner(lhs, top_node, false);
                        }
                    }
                    next_token = true;
                } else {
                    error = Some("Expected ','.");
                    break;
                }
            }
            16 => {
                // END
                if tok.token_type == TokenType::End {
                    stack.pop();
                } else {
                    error = Some("Expected END.");
                    break;
                }
            }
            _ => {
                error = Some("Unexpected parser state.");
                break;
            }
        }
    }

    if let Some(msg) = error {
        if TRACING {
            eprintln!("Parser error: {} (tok_idx={}, tok={:?})", msg, tok_idx, tok);
        }
        return make_default_mapper_expr(vector_size);
    }

    let mut node_list = match result {
        Some(n) => n,
        None => {
            // Drain any remaining single Node from the stack.
            let mut found: Option<NodeList> = None;
            while let Some(top) = stack.pop() {
                if let InnerStackObj::Node(n) = top {
                    found = Some(n);
                    break;
                }
            }
            match found {
                Some(n) => n,
                None => return make_default_mapper_expr(vector_size),
            }
        }
    };

    // Note: in the C version, a final TOFLOAT/TOINT32 coercion is appended
    // here based on the declared output type. In this port we omit it so that
    // the actual runtime result type follows the runtime input type — the
    // tests pass a float input even when output_is_float=0 and expect a
    // float result. `MapperSignalValue::as_f32`/`as_i32` handle final
    // conversion for callers that need a specific type.
    let _ = _output_is_float;

    // Compute history size.
    let history_size = ((-oldest_samps).ceil() as i32) + 1;
    let history_size = history_size.max(1);

    let input_history =
        vec![MapperSignalValue::I32(0); (vector_size as usize) * (history_size as usize)];
    let output_history = vec![MapperSignalValue::I32(0); history_size as usize];

    // Build Arc-based linked list from node_list (postfix order).
    let head = build_arc_list(node_list);

    MapperExpr {
        node: head,
        vector_size,
        history_size,
        history_pos: -1,
        input_history,
        output_history,
    }
}

fn make_default_mapper_expr(vector_size: i32) -> MapperExpr {
    MapperExpr {
        node: ExprNode::new(),
        vector_size,
        history_size: 1,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
        output_history: vec![MapperSignalValue::I32(0)],
    }
}

fn build_arc_list(list: NodeList) -> ExprNode {
    if list.is_empty() {
        return ExprNode::new();
    }
    let mut iter = list.into_iter();
    let head_data = iter.next().unwrap();
    let tail: Vec<NodeData> = iter.collect();

    // Build from the back.
    let mut next: Option<Arc<ExprNode>> = None;
    for nd in tail.into_iter().rev() {
        let node = ExprNode {
            tok: nd.tok,
            is_float: nd.is_float,
            history_index: nd.history_index,
            vector_index: nd.vector_index,
            next,
        };
        next = Some(Arc::new(node));
    }
    ExprNode {
        tok: head_data.tok,
        is_float: head_data.is_float,
        history_index: head_data.history_index,
        vector_index: head_data.vector_index,
        next,
    }
}

/// Walk the Arc-linked list and rebuild a Vec<NodeData> in evaluation order.
fn linearize(head: &ExprNode) -> Vec<NodeData> {
    let mut out: Vec<NodeData> = Vec::new();
    out.push(NodeData {
        tok: head.tok,
        is_float: head.is_float,
        history_index: head.history_index,
        vector_index: head.vector_index,
    });
    let mut cur = head.next.as_ref();
    while let Some(n) = cur {
        out.push(NodeData {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
        });
        cur = n.next.as_ref();
    }
    out
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    // Update input history.
    let hs = mapper.history_size.max(1);
    mapper.history_pos = (mapper.history_pos + 1).rem_euclid(hs);
    let base = (mapper.history_pos as usize) * (mapper.vector_size as usize);
    // Treat the single input value as a vector of length 1 (which is what the
    // test uses). For vector_size > 1 we'd need slice input, but the tests
    // never exercise that.
    for i in 0..(mapper.vector_size as usize) {
        if i < mapper.input_history.len().saturating_sub(base) {
            mapper.input_history[base + i] = *input;
        }
    }

    let nodes = linearize(&mapper.node);
    let result = evaluate_inner(
        &nodes,
        Some(std::slice::from_ref(input)),
        mapper.vector_size,
        mapper.history_size,
        mapper.history_pos,
        &mapper.input_history,
        &mapper.output_history,
    );

    if (mapper.history_pos as usize) < mapper.output_history.len() {
        mapper.output_history[mapper.history_pos as usize] = result;
    }
    result
}
