use std::collections::HashMap;
use std::f32::consts::PI;
use std::sync::Arc;

use lazy_static::lazy_static;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}

impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        Some(match *self {
            MapperSignalValue::F(v) => v,
            MapperSignalValue::I32(v) => v as f32,
        })
    }

    pub fn as_i32(&self) -> Option<i32> {
        Some(match *self {
            MapperSignalValue::F(v) => v as i32,
            MapperSignalValue::I32(v) => v,
        })
    }
}

const STACK_SIZE: usize = 256;
const TRACING: bool = false;

macro_rules! trace {
    ($($arg:tt)*) => {
        if TRACING {
            println!("-- {}", format!($($arg)*));
        }
    };
}

macro_rules! die_unless {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            println!("-- {}", format!($($arg)*));
            debug_assert!($cond);
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

fn pow_fn(x: f32, y: f32) -> f32 { x.powf(y) }
fn sin_fn(x: f32, _: f32) -> f32 { x.sin() }
fn cos_fn(x: f32, _: f32) -> f32 { x.cos() }
fn tan_fn(x: f32, _: f32) -> f32 { x.tan() }
fn abs_fn(x: f32, _: f32) -> f32 { x.abs() }
fn sqrt_fn(x: f32, _: f32) -> f32 { x.sqrt() }
fn log_fn(x: f32, _: f32) -> f32 { x.ln() }
fn log10_fn(x: f32, _: f32) -> f32 { x.log10() }
fn exp_fn(x: f32, _: f32) -> f32 { x.exp() }
fn floor_fn(x: f32, _: f32) -> f32 { x.floor() }
fn round_fn(x: f32, _: f32) -> f32 { x.round() }
fn ceil_fn(x: f32, _: f32) -> f32 { x.ceil() }
fn asin_fn(x: f32, _: f32) -> f32 { x.asin() }
fn acos_fn(x: f32, _: f32) -> f32 { x.acos() }
fn atan_fn(x: f32, _: f32) -> f32 { x.atan() }
fn atan2_fn(x: f32, y: f32) -> f32 { x.atan2(y) }
fn sinh_fn(x: f32, _: f32) -> f32 { x.sinh() }
fn cosh_fn(x: f32, _: f32) -> f32 { x.cosh() }
fn tanh_fn(x: f32, _: f32) -> f32 { x.tanh() }
fn logb_fn(x: f32, _: f32) -> f32 {
    if x == 0.0 {
        f32::NEG_INFINITY
    } else if x.is_nan() {
        f32::NAN
    } else if x.is_infinite() {
        f32::INFINITY
    } else {
        let (_, exp) = libm::frexpf(x.abs());
        (exp - 1) as f32
    }
}
fn exp2_fn(x: f32, _: f32) -> f32 { x.exp2() }
fn log2_fn(x: f32, _: f32) -> f32 { x.log2() }
fn hypot_fn(x: f32, y: f32) -> f32 { x.hypot(y) }
fn cbrt_fn(x: f32, _: f32) -> f32 { x.cbrt() }
fn trunc_fn(x: f32, _: f32) -> f32 { x.trunc() }
fn min_fn(x: f32, y: f32) -> f32 { minf(x, y) }
fn max_fn(x: f32, y: f32) -> f32 { maxf(x, y) }
fn pi_fn(_: f32, _: f32) -> f32 { pif() }

#[derive(Debug, Clone, Copy)]
struct FunctionSpec {
    id: ExprFunc,
    entry: FunctionEntry,
}

static FUNCTION_SPECS: [FunctionSpec; 29] = [
    FunctionSpec { id: ExprFunc::Pow, entry: FunctionEntry { name: "pow", arity: 2, func: pow_fn } },
    FunctionSpec { id: ExprFunc::Sin, entry: FunctionEntry { name: "sin", arity: 1, func: sin_fn } },
    FunctionSpec { id: ExprFunc::Cos, entry: FunctionEntry { name: "cos", arity: 1, func: cos_fn } },
    FunctionSpec { id: ExprFunc::Tan, entry: FunctionEntry { name: "tan", arity: 1, func: tan_fn } },
    FunctionSpec { id: ExprFunc::Abs, entry: FunctionEntry { name: "abs", arity: 1, func: abs_fn } },
    FunctionSpec { id: ExprFunc::Sqrt, entry: FunctionEntry { name: "sqrt", arity: 1, func: sqrt_fn } },
    FunctionSpec { id: ExprFunc::Log, entry: FunctionEntry { name: "log", arity: 1, func: log_fn } },
    FunctionSpec { id: ExprFunc::Log10, entry: FunctionEntry { name: "log10", arity: 1, func: log10_fn } },
    FunctionSpec { id: ExprFunc::Exp, entry: FunctionEntry { name: "exp", arity: 1, func: exp_fn } },
    FunctionSpec { id: ExprFunc::Floor, entry: FunctionEntry { name: "floor", arity: 1, func: floor_fn } },
    FunctionSpec { id: ExprFunc::Round, entry: FunctionEntry { name: "round", arity: 1, func: round_fn } },
    FunctionSpec { id: ExprFunc::Ceil, entry: FunctionEntry { name: "ceil", arity: 1, func: ceil_fn } },
    FunctionSpec { id: ExprFunc::Asin, entry: FunctionEntry { name: "asin", arity: 1, func: asin_fn } },
    FunctionSpec { id: ExprFunc::Acos, entry: FunctionEntry { name: "acos", arity: 1, func: acos_fn } },
    FunctionSpec { id: ExprFunc::Atan, entry: FunctionEntry { name: "atan", arity: 1, func: atan_fn } },
    FunctionSpec { id: ExprFunc::Atan2, entry: FunctionEntry { name: "atan2", arity: 2, func: atan2_fn } },
    FunctionSpec { id: ExprFunc::Sinh, entry: FunctionEntry { name: "sinh", arity: 1, func: sinh_fn } },
    FunctionSpec { id: ExprFunc::Cosh, entry: FunctionEntry { name: "cosh", arity: 1, func: cosh_fn } },
    FunctionSpec { id: ExprFunc::Tanh, entry: FunctionEntry { name: "tanh", arity: 1, func: tanh_fn } },
    FunctionSpec { id: ExprFunc::Logb, entry: FunctionEntry { name: "logb", arity: 1, func: logb_fn } },
    FunctionSpec { id: ExprFunc::Exp2, entry: FunctionEntry { name: "exp2", arity: 1, func: exp2_fn } },
    FunctionSpec { id: ExprFunc::Log2, entry: FunctionEntry { name: "log2", arity: 1, func: log2_fn } },
    FunctionSpec { id: ExprFunc::Hypot, entry: FunctionEntry { name: "hypot", arity: 2, func: hypot_fn } },
    FunctionSpec { id: ExprFunc::Cbrt, entry: FunctionEntry { name: "cbrt", arity: 1, func: cbrt_fn } },
    FunctionSpec { id: ExprFunc::Trunc, entry: FunctionEntry { name: "trunc", arity: 1, func: trunc_fn } },
    FunctionSpec { id: ExprFunc::Min, entry: FunctionEntry { name: "min", arity: 2, func: min_fn } },
    FunctionSpec { id: ExprFunc::Max, entry: FunctionEntry { name: "max", arity: 2, func: max_fn } },
    FunctionSpec { id: ExprFunc::Pi, entry: FunctionEntry { name: "pi", arity: 0, func: pi_fn } },
    FunctionSpec { id: ExprFunc::NFuncs, entry: FunctionEntry { name: "", arity: 0, func: pi_fn } },
];

lazy_static! {
    static ref FUNCTION_TABLE: HashMap<&'static str, FunctionEntry> = {
        let mut m = HashMap::new();
        m.insert("pow", FunctionEntry { name: "pow", arity: 2, func: pow_fn });
        m.insert("sin", FunctionEntry { name: "sin", arity: 1, func: sin_fn });
        m.insert("cos", FunctionEntry { name: "cos", arity: 1, func: cos_fn });
        m.insert("tan", FunctionEntry { name: "tan", arity: 1, func: tan_fn });
        m.insert("abs", FunctionEntry { name: "abs", arity: 1, func: abs_fn });
        m.insert("sqrt", FunctionEntry { name: "sqrt", arity: 1, func: sqrt_fn });
        m.insert("log", FunctionEntry { name: "log", arity: 1, func: log_fn });
        m.insert("log10", FunctionEntry { name: "log10", arity: 1, func: log10_fn });
        m.insert("exp", FunctionEntry { name: "exp", arity: 1, func: exp_fn });
        m.insert("floor", FunctionEntry { name: "floor", arity: 1, func: floor_fn });
        m.insert("round", FunctionEntry { name: "round", arity: 1, func: round_fn });
        m.insert("ceil", FunctionEntry { name: "ceil", arity: 1, func: ceil_fn });
        m.insert("asin", FunctionEntry { name: "asin", arity: 1, func: asin_fn });
        m.insert("acos", FunctionEntry { name: "acos", arity: 1, func: acos_fn });
        m.insert("atan", FunctionEntry { name: "atan", arity: 1, func: atan_fn });
        m.insert("atan2", FunctionEntry { name: "atan2", arity: 2, func: atan2_fn });
        m.insert("sinh", FunctionEntry { name: "sinh", arity: 1, func: sinh_fn });
        m.insert("cosh", FunctionEntry { name: "cosh", arity: 1, func: cosh_fn });
        m.insert("tanh", FunctionEntry { name: "tanh", arity: 1, func: tanh_fn });
        m.insert("logb", FunctionEntry { name: "logb", arity: 1, func: logb_fn });
        m.insert("exp2", FunctionEntry { name: "exp2", arity: 1, func: exp2_fn });
        m.insert("log2", FunctionEntry { name: "log2", arity: 1, func: log2_fn });
        m.insert("hypot", FunctionEntry { name: "hypot", arity: 2, func: hypot_fn });
        m.insert("cbrt", FunctionEntry { name: "cbrt", arity: 1, func: cbrt_fn });
        m.insert("trunc", FunctionEntry { name: "trunc", arity: 1, func: trunc_fn });
        m.insert("min", FunctionEntry { name: "min", arity: 2, func: min_fn });
        m.insert("max", FunctionEntry { name: "max", arity: 2, func: max_fn });
        m.insert("pi", FunctionEntry { name: "pi", arity: 0, func: pi_fn });
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

fn token_float(v: f32) -> Token {
    Token { token_type: TokenType::Float, value: Some(v), int_value: None, var: None, op: None }
}

fn token_int(v: i32) -> Token {
    Token { token_type: TokenType::Int, value: None, int_value: Some(v), var: None, op: None }
}

fn token_op(op: char) -> Token {
    Token { token_type: TokenType::Op, value: None, int_value: None, var: None, op: Some(op) }
}

fn token_var(var: char) -> Token {
    Token { token_type: TokenType::Var, value: None, int_value: None, var: Some(var), op: None }
}

fn token_func(func: ExprFunc) -> Token {
    Token { token_type: TokenType::Func, value: None, int_value: Some(func as i32), var: None, op: None }
}

fn token_tofloat() -> Token {
    Token { token_type: TokenType::ToFloat, value: None, int_value: None, var: None, op: None }
}

fn token_toint32() -> Token {
    Token { token_type: TokenType::ToInt32, value: None, int_value: None, var: None, op: None }
}

fn expr_func_from_i32(v: i32) -> ExprFunc {
    match v {
        0 => ExprFunc::Pow,
        1 => ExprFunc::Sin,
        2 => ExprFunc::Cos,
        3 => ExprFunc::Tan,
        4 => ExprFunc::Abs,
        5 => ExprFunc::Sqrt,
        6 => ExprFunc::Log,
        7 => ExprFunc::Log10,
        8 => ExprFunc::Exp,
        9 => ExprFunc::Floor,
        10 => ExprFunc::Round,
        11 => ExprFunc::Ceil,
        12 => ExprFunc::Asin,
        13 => ExprFunc::Acos,
        14 => ExprFunc::Atan,
        15 => ExprFunc::Atan2,
        16 => ExprFunc::Sinh,
        17 => ExprFunc::Cosh,
        18 => ExprFunc::Tanh,
        19 => ExprFunc::Logb,
        20 => ExprFunc::Exp2,
        21 => ExprFunc::Log2,
        22 => ExprFunc::Hypot,
        23 => ExprFunc::Cbrt,
        24 => ExprFunc::Trunc,
        25 => ExprFunc::Min,
        26 => ExprFunc::Max,
        27 => ExprFunc::Pi,
        _ => ExprFunc::Unknown,
    }
}

fn function_spec_by_id(id: ExprFunc) -> Option<&'static FunctionSpec> {
    FUNCTION_SPECS
        .iter()
        .find(|spec| spec.id == id && spec.id != ExprFunc::NFuncs)
}

fn function_spec_by_name(name: &str) -> Option<&'static FunctionSpec> {
    FUNCTION_SPECS.iter().find(|spec| {
        spec.id != ExprFunc::NFuncs && spec.entry.name.starts_with(name)
    })
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    function_spec_by_name(s).map(|spec| &spec.entry)
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let bytes = input.as_bytes();
    let mut pos = 0usize;
    let mut tokens = Vec::new();

    while pos < bytes.len() {
        let c = bytes[pos] as char;
        if c.is_ascii_whitespace() {
            pos += 1;
            continue;
        }

        if c.is_ascii_digit() {
            let start = pos;
            pos += 1;
            while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                pos += 1;
            }

            let integer: i32 = input[start..pos].parse().unwrap_or(0);
            if pos >= bytes.len() || (bytes[pos] as char) != '.' {
                tokens.push(token_int(integer));
                continue;
            }

            let dot_pos = pos;
            pos += 1;
            while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                pos += 1;
            }

            let frac = input[dot_pos..pos].parse::<f32>().unwrap_or(0.0);
            tokens.push(token_float(integer as f32 + frac));
            continue;
        }

        match c {
            '.' => {
                let start = pos;
                pos += 1;
                if pos >= bytes.len() || !(bytes[pos] as char).is_ascii_digit() {
                    return Err(format!("unknown character '{}' in lexer", c));
                }
                while pos < bytes.len() && (bytes[pos] as char).is_ascii_digit() {
                    pos += 1;
                }
                tokens.push(token_float(input[start..pos].parse::<f32>().unwrap_or(0.0)));
            }
            '+' | '-' | '/' | '*' | '=' => {
                tokens.push(token_op(c));
                pos += 1;
            }
            '(' => {
                tokens.push(Token { token_type: TokenType::OpenParen, value: None, int_value: None, var: None, op: None });
                pos += 1;
            }
            ')' => {
                tokens.push(Token { token_type: TokenType::CloseParen, value: None, int_value: None, var: None, op: None });
                pos += 1;
            }
            '[' => {
                tokens.push(Token { token_type: TokenType::OpenSquare, value: None, int_value: None, var: None, op: None });
                pos += 1;
            }
            ']' => {
                tokens.push(Token { token_type: TokenType::CloseSquare, value: None, int_value: None, var: None, op: None });
                pos += 1;
            }
            '{' => {
                tokens.push(Token { token_type: TokenType::OpenCurly, value: None, int_value: None, var: None, op: None });
                pos += 1;
            }
            '}' => {
                tokens.push(Token { token_type: TokenType::CloseCurly, value: None, int_value: None, var: None, op: None });
                pos += 1;
            }
            ',' => {
                tokens.push(Token { token_type: TokenType::Comma, value: None, int_value: None, var: None, op: None });
                pos += 1;
            }
            'x' | 'y' => {
                if pos + 1 < bytes.len() && (bytes[pos + 1] as char).is_ascii_alphanumeric() {
                    let start = pos;
                    pos += 1;
                    while pos < bytes.len() && (bytes[pos] as char).is_ascii_alphanumeric() {
                        pos += 1;
                    }
                    let name = &input[start..pos];
                    let spec = function_spec_by_name(name)
                        .ok_or_else(|| "Unknown function.".to_string())?;
                    tokens.push(token_func(spec.id));
                } else {
                    tokens.push(token_var(c));
                    pos += 1;
                }
            }
            _ if c.is_ascii_alphabetic() => {
                let start = pos;
                pos += 1;
                while pos < bytes.len() && (bytes[pos] as char).is_ascii_alphanumeric() {
                    pos += 1;
                }
                let name = &input[start..pos];
                let spec = function_spec_by_name(name)
                    .ok_or_else(|| "Unknown function.".to_string())?;
                tokens.push(token_func(spec.id));
            }
            _ => {
                return Err(format!("unknown character '{}' in lexer", c));
            }
        }
    }

    tokens.push(Token { token_type: TokenType::End, value: None, int_value: None, var: None, op: None });
    Ok(tokens)
}

fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let joined = s.concat();
    tokenize(&joined).unwrap_or_else(|_| vec![Token {
        token_type: TokenType::End,
        value: None,
        int_value: None,
        var: None,
        op: None,
    }])
}

#[derive(Clone, Copy, Debug)]
struct CompiledNode {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

#[derive(Clone, Debug)]
struct ExprFragment {
    nodes: Vec<CompiledNode>,
    is_float: bool,
    refvar: bool,
}

impl ExprFragment {
    fn from_node(node: CompiledNode) -> Self {
        Self { nodes: vec![node], is_float: node.is_float != 0, refvar: node.tok.token_type == TokenType::Var }
    }
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

enum stack_obj_t {
    State(state_t),
    Node(ExprNode),
}

impl ExprNode {
    pub fn new() -> ExprNode {
        ExprNode {
            tok: token_int(0),
            is_float: 0,
            history_index: 0,
            vector_index: 0,
            next: None,
        }
    }

    pub fn expr_free(&self) {}
}

fn printtoken(t: &Token) {
    match t.token_type {
        TokenType::Float => print!("{}", t.value.unwrap_or(0.0)),
        TokenType::Int => print!("{}", t.int_value.unwrap_or(0)),
        TokenType::Op => print!("{}", t.op.unwrap_or('?')),
        TokenType::OpenParen => print!("("),
        TokenType::CloseParen => print!(")"),
        TokenType::Var => print!("VAR({})", t.var.unwrap_or('?')),
        TokenType::OpenSquare => print!("["),
        TokenType::CloseSquare => print!("]"),
        TokenType::OpenCurly => print!("{{"),
        TokenType::CloseCurly => print!("}}"),
        TokenType::Func => {
            let func = t.int_value.map(expr_func_from_i32).unwrap_or(ExprFunc::Unknown);
            if let Some(spec) = function_spec_by_id(func) {
                print!("FUNC({})", spec.entry.name);
            } else {
                print!("FUNC(?)");
            }
        }
        TokenType::Comma => print!(","),
        TokenType::End => print!("END"),
        TokenType::ToFloat => print!("(float)"),
        TokenType::ToInt32 => print!("(int32)"),
    }
}

fn printexprnode(s: &str, list: &ExprNode) {
    print!("{s}");
    let mut current = Some(list);
    let mut first = true;
    while let Some(node) = current {
        if !first {
            print!(" ");
        }
        if node.is_float != 0
            && node.tok.token_type != TokenType::Float
            && node.tok.token_type != TokenType::ToFloat
        {
            print!(".");
        }
        printtoken(&node.tok);
        if node.tok.token_type == TokenType::Var {
            if node.history_index < 0 {
                print!("{{{}}}", node.history_index);
            }
            if node.vector_index > -1 {
                print!("[{}]", node.vector_index);
            }
        }
        current = node.next.as_deref();
        first = false;
    }
}

fn printexpr(s: &str, list: &MapperExpr) {
    printexprnode(s, &list.node);
}

fn printstack(stack: &stack_obj_t, stack_size: i32) {
    match stack {
        stack_obj_t::State(_) => println!("Stack state depth={stack_size}"),
        stack_obj_t::Node(node) => {
            print!("Stack node depth={stack_size}: ");
            printexprnode("", node);
            println!();
        }
    }
}

fn collapse_expr_to_left(plhs: &mut ExprNode, constant_folding: i32) {
    let _ = (plhs, constant_folding);
}

fn clone_expr_node(node: &ExprNode) -> ExprNode {
    ExprNode {
        tok: node.tok,
        is_float: node.is_float,
        history_index: node.history_index,
        vector_index: node.vector_index,
        next: node.next.clone(),
    }
}

fn compiled_to_exprnode(nodes: &[CompiledNode]) -> ExprNode {
    if nodes.is_empty() {
        return ExprNode::new();
    }

    let mut next: Option<Arc<ExprNode>> = None;
    for node in nodes.iter().rev() {
        let expr_node = ExprNode {
            tok: node.tok,
            is_float: node.is_float,
            history_index: node.history_index,
            vector_index: node.vector_index,
            next,
        };
        next = Some(Arc::new(expr_node));
    }

    clone_expr_node(next.as_deref().unwrap_or(&ExprNode::new()))
}

fn exprnode_to_compiled(node: &ExprNode) -> Vec<CompiledNode> {
    let mut result = Vec::new();
    let mut current = Some(node);
    while let Some(n) = current {
        result.push(CompiledNode {
            tok: n.tok,
            is_float: n.is_float,
            history_index: n.history_index,
            vector_index: n.vector_index,
        });
        current = n.next.as_deref();
    }
    result
}

fn mod_i32(value: i32, modulus: i32) -> i32 {
    ((value % modulus) + modulus) % modulus
}

fn coerce_fragment_to_float(mut fragment: ExprFragment) -> ExprFragment {
    if !fragment.is_float {
        fragment.nodes.push(CompiledNode {
            tok: token_tofloat(),
            is_float: 1,
            history_index: 0,
            vector_index: 0,
        });
        fragment.is_float = true;
    }
    fragment
}

fn binary_fragment(mut lhs: ExprFragment, mut rhs: ExprFragment, op: char) -> ExprFragment {
    if lhs.is_float && !rhs.is_float {
        rhs = coerce_fragment_to_float(rhs);
    } else if !lhs.is_float && rhs.is_float {
        lhs = coerce_fragment_to_float(lhs);
    }

    let is_float = lhs.is_float || rhs.is_float;
    let refvar = lhs.refvar || rhs.refvar;
    let mut nodes = lhs.nodes;
    nodes.extend(rhs.nodes);
    nodes.push(CompiledNode {
        tok: token_op(op),
        is_float: if is_float { 1 } else { 0 },
        history_index: 0,
        vector_index: 0,
    });
    ExprFragment { nodes, is_float, refvar }
}

fn func_fragment(func: ExprFunc, args: Vec<ExprFragment>) -> ExprFragment {
    let mut nodes = Vec::new();
    let mut refvar = false;
    for arg in args {
        let arg = coerce_fragment_to_float(arg);
        refvar |= arg.refvar;
        nodes.extend(arg.nodes);
    }
    nodes.push(CompiledNode {
        tok: token_func(func),
        is_float: 1,
        history_index: 0,
        vector_index: 0,
    });
    ExprFragment { nodes, is_float: true, refvar }
}

fn eval_nodes(nodes: &[CompiledNode], expr: Option<&MapperExpr>) -> Result<MapperSignalValue, ()> {
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
                let Some(expr) = expr else {
                    return Err(());
                };
                let idx = mod_i32(node.history_index + expr.history_pos, expr.history_size);
                let value = match node.tok.var.unwrap_or('?') {
                    'x' => {
                        let offset = idx as usize * expr.vector_size as usize + node.vector_index as usize;
                        *expr.input_history.get(offset).unwrap_or(&MapperSignalValue::I32(0))
                    }
                    'y' => *expr.output_history.get(idx as usize).unwrap_or(&MapperSignalValue::I32(0)),
                    _ => return Err(()),
                };
                stack.push(value);
            }
            TokenType::ToFloat => {
                let value = stack.pop().ok_or(())?.as_f32().unwrap_or(0.0);
                stack.push(MapperSignalValue::F(value));
            }
            TokenType::ToInt32 => {
                let value = stack.pop().ok_or(())?.as_i32().unwrap_or(0);
                stack.push(MapperSignalValue::I32(value));
            }
            TokenType::Op => {
                let right = stack.pop().ok_or(())?;
                let left = stack.pop().ok_or(())?;
                let op = node.tok.op.unwrap_or('?');
                if node.is_float != 0 {
                    let lhs = left.as_f32().unwrap_or(0.0);
                    let rhs = right.as_f32().unwrap_or(0.0);
                    let value = match op {
                        '+' => lhs + rhs,
                        '-' => lhs - rhs,
                        '*' => lhs * rhs,
                        '/' => lhs / rhs,
                        _ => return Err(()),
                    };
                    stack.push(MapperSignalValue::F(value));
                } else {
                    let lhs = left.as_i32().unwrap_or(0);
                    let rhs = right.as_i32().unwrap_or(0);
                    let value = match op {
                        '+' => lhs + rhs,
                        '-' => lhs - rhs,
                        '*' => lhs * rhs,
                        '/' => lhs / rhs,
                        _ => return Err(()),
                    };
                    stack.push(MapperSignalValue::I32(value));
                }
            }
            TokenType::Func => {
                let func = expr_func_from_i32(node.tok.int_value.unwrap_or(-1));
                let spec = function_spec_by_id(func).ok_or(())?;
                match spec.entry.arity {
                    0 => stack.push(MapperSignalValue::F((spec.entry.func)(0.0, 0.0))),
                    1 => {
                        let arg = stack.pop().ok_or(())?.as_f32().unwrap_or(0.0);
                        stack.push(MapperSignalValue::F((spec.entry.func)(arg, 0.0)));
                    }
                    2 => {
                        let right = stack.pop().ok_or(())?.as_f32().unwrap_or(0.0);
                        let left = stack.pop().ok_or(())?.as_f32().unwrap_or(0.0);
                        stack.push(MapperSignalValue::F((spec.entry.func)(left, right)));
                    }
                    _ => return Err(()),
                }
            }
            _ => return Err(()),
        }
    }

    stack.pop().ok_or(())
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    input_is_float: bool,
    output_is_float: bool,
    vector_size: i32,
    oldest_samps: i32,
}

impl Parser {
    fn new(tokens: Vec<Token>, input_is_float: i32, output_is_float: i32, vector_size: i32) -> Self {
        Self {
            tokens,
            pos: 0,
            input_is_float: input_is_float != 0,
            output_is_float: output_is_float != 0,
            vector_size,
            oldest_samps: 0,
        }
    }

    fn current(&self) -> Token {
        *self.tokens.get(self.pos).unwrap_or(&Token {
            token_type: TokenType::End,
            value: None,
            int_value: None,
            var: None,
            op: None,
        })
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn expect_simple(&mut self, token_type: TokenType) -> Result<(), String> {
        if self.current().token_type == token_type {
            self.advance();
            Ok(())
        } else {
            Err(match token_type {
                TokenType::CloseParen => "Expected ')'.".to_string(),
                TokenType::OpenParen => "Expected '('.".to_string(),
                TokenType::Comma => "Expected ','.".to_string(),
                TokenType::CloseSquare => "Expected ']'.".to_string(),
                TokenType::CloseCurly => "Expected '}'.".to_string(),
                TokenType::End => "Expected END.".to_string(),
                _ => "Unexpected token.".to_string(),
            })
        }
    }

    fn parse(mut self) -> Result<MapperExpr, String> {
        let tok = self.current();
        if tok.token_type != TokenType::Var || tok.var != Some('y') {
            return Err("Error in y= prefix.".to_string());
        }
        self.advance();

        let tok = self.current();
        if tok.token_type != TokenType::Op || tok.op != Some('=') {
            return Err("Error in y= prefix.".to_string());
        }
        self.advance();

        let mut fragment = self.parse_expr(true)?;
        self.expect_simple(TokenType::End)?;

        if self.oldest_samps < -100 {
            trace!("Expression contains history reference of {}", self.oldest_samps);
            return Err("Expression contains history reference outside supported range.".to_string());
        }

        if fragment.is_float && !self.output_is_float {
            fragment.nodes.push(CompiledNode {
                tok: token_toint32(),
                is_float: 0,
                history_index: 0,
                vector_index: 0,
            });
            fragment.is_float = false;
        } else if !fragment.is_float && self.output_is_float {
            fragment.nodes.push(CompiledNode {
                tok: token_tofloat(),
                is_float: 1,
                history_index: 0,
                vector_index: 0,
            });
            fragment.is_float = true;
        }

        if self.vector_size > 1 {
            for node in &fragment.nodes {
                if node.tok.token_type == TokenType::Var && node.vector_index > 0 {
                    return Err("vector indexing not yet implemented".to_string());
                }
            }
        }

        let history_size = (-self.oldest_samps) + 1;
        let node = compiled_to_exprnode(&fragment.nodes);
        Ok(MapperExpr {
            node,
            vector_size: self.vector_size,
            history_size,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0); (self.vector_size * history_size) as usize],
            output_history: vec![MapperSignalValue::I32(0); history_size as usize],
        })
    }

    fn parse_expr(&mut self, allow_vars: bool) -> Result<ExprFragment, String> {
        let mut lhs = self.parse_term(allow_vars)?;
        loop {
            let tok = self.current();
            if tok.token_type == TokenType::Op && matches!(tok.op, Some('+') | Some('-')) {
                let op = tok.op.unwrap_or('+');
                self.advance();
                let rhs = self.parse_term(allow_vars)?;
                lhs = binary_fragment(lhs, rhs, op);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_term(&mut self, allow_vars: bool) -> Result<ExprFragment, String> {
        let mut lhs = self.parse_value(allow_vars)?;
        loop {
            let tok = self.current();
            if tok.token_type == TokenType::Op && matches!(tok.op, Some('*') | Some('/')) {
                let op = tok.op.unwrap_or('*');
                self.advance();
                let rhs = self.parse_value(allow_vars)?;
                lhs = binary_fragment(lhs, rhs, op);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_value(&mut self, allow_vars: bool) -> Result<ExprFragment, String> {
        let tok = self.current();
        match tok.token_type {
            TokenType::Int => {
                self.advance();
                Ok(ExprFragment::from_node(CompiledNode {
                    tok,
                    is_float: 0,
                    history_index: 0,
                    vector_index: 0,
                }))
            }
            TokenType::Float => {
                self.advance();
                Ok(ExprFragment::from_node(CompiledNode {
                    tok,
                    is_float: 1,
                    history_index: 0,
                    vector_index: 0,
                }))
            }
            TokenType::Var => {
                if !allow_vars {
                    return Err("Unexpected variable reference.".to_string());
                }
                self.parse_variable()
            }
            TokenType::OpenParen => {
                self.advance();
                let expr = self.parse_expr(allow_vars)?;
                self.expect_simple(TokenType::CloseParen)?;
                Ok(expr)
            }
            TokenType::Func => self.parse_function(),
            TokenType::Op if tok.op == Some('-') => {
                self.advance();
                let value = self.parse_value(allow_vars)?;
                Ok(binary_fragment(
                    ExprFragment::from_node(CompiledNode {
                        tok: token_int(0),
                        is_float: 0,
                        history_index: 0,
                        vector_index: 0,
                    }),
                    value,
                    '-',
                ))
            }
            _ => Err("Expected value.".to_string()),
        }
    }

    fn parse_function(&mut self) -> Result<ExprFragment, String> {
        let tok = self.current();
        self.advance();

        let func = expr_func_from_i32(tok.int_value.unwrap_or(-1));
        let spec = function_spec_by_id(func).ok_or_else(|| "Unknown function.".to_string())?;
        if spec.id == ExprFunc::Unknown {
            return Err("Unknown function.".to_string());
        }

        if spec.entry.arity == 0 {
            return Ok(func_fragment(func, Vec::new()));
        }

        self.expect_simple(TokenType::OpenParen)?;
        let mut args = Vec::new();
        for i in 0..spec.entry.arity {
            args.push(self.parse_expr(true)?);
            if i + 1 < spec.entry.arity {
                self.expect_simple(TokenType::Comma)?;
            }
        }
        self.expect_simple(TokenType::CloseParen)?;
        Ok(func_fragment(func, args))
    }

    fn parse_index_expr(&mut self, close: TokenType) -> Result<i32, String> {
        let expr = self.parse_expr(false)?;
        let value = eval_nodes(&expr.nodes, None).map_err(|_| "Expected value.".to_string())?;
        self.expect_simple(close)?;
        Ok(match value {
            MapperSignalValue::F(v) => v as i32,
            MapperSignalValue::I32(v) => v,
        })
    }

    fn parse_variable(&mut self) -> Result<ExprFragment, String> {
        let tok = self.current();
        self.advance();

        let mut history_index = 0;
        let mut vector_index = 0;
        let mut seen_vec = false;
        let mut seen_hist = false;

        loop {
            match self.current().token_type {
                TokenType::OpenSquare if !seen_vec => {
                    self.advance();
                    vector_index = self.parse_index_expr(TokenType::CloseSquare)?;
                    if vector_index > 0 {
                        return Err("Vector indexing not yet implemented.".to_string());
                    }
                    if vector_index < 0 || vector_index >= self.vector_size {
                        return Err("Vector index outside input size.".to_string());
                    }
                    seen_vec = true;
                }
                TokenType::OpenCurly if !seen_hist => {
                    self.advance();
                    history_index = self.parse_index_expr(TokenType::CloseCurly)?;
                    if self.oldest_samps > history_index {
                        self.oldest_samps = history_index;
                    }
                    seen_hist = true;
                }
                _ => break,
            }
        }

        Ok(ExprFragment::from_node(CompiledNode {
            tok,
            is_float: if self.input_is_float { 1 } else { 0 },
            history_index,
            vector_index,
        }))
    }
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    if s.is_empty() {
        return MapperExpr {
            node: ExprNode::new(),
            vector_size,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
            output_history: vec![MapperSignalValue::I32(0)],
        };
    }

    match tokenize(s).and_then(|tokens| Parser::new(tokens, input_is_float, output_is_float, vector_size).parse()) {
        Ok(expr) => expr,
        Err(msg) => {
            println!("{msg}");
            MapperExpr {
                node: ExprNode::new(),
                vector_size,
                history_size: 1,
                history_pos: -1,
                input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize],
                output_history: vec![MapperSignalValue::I32(0)],
            }
        }
    }
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    if mapper.history_size <= 0 || mapper.vector_size <= 0 {
        return MapperSignalValue::I32(0);
    }

    mapper.history_pos = mod_i32(mapper.history_pos + 1, mapper.history_size);
    let input_slot = mapper.history_pos as usize * mapper.vector_size as usize;
    if let Some(slot) = mapper.input_history.get_mut(input_slot) {
        *slot = *input;
    }

    let nodes = exprnode_to_compiled(&mapper.node);
    let result = eval_nodes(&nodes, Some(mapper)).unwrap_or(MapperSignalValue::I32(0));

    if let Some(slot) = mapper.output_history.get_mut(mapper.history_pos as usize) {
        *slot = result;
    }

    result
}
