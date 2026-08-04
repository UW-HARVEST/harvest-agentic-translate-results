use std::f32::consts::PI;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub enum MapperSignalValue {
    F(f32),
    I32(i32),
}

impl MapperSignalValue {
    pub fn as_f32(&self) -> Option<f32> {
        Some(match *self {
            Self::F(value) => value,
            Self::I32(value) => value as f32,
        })
    }

    pub fn as_i32(&self) -> Option<i32> {
        Some(match *self {
            Self::F(value) => value as i32,
            Self::I32(value) => value,
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
    if x.is_nan() {
        f32::NAN
    } else if x == 0.0 {
        f32::NEG_INFINITY
    } else if x.is_infinite() {
        f32::INFINITY
    } else {
        x.abs().log2().floor()
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

const FUNCTION_TABLE: [FunctionEntry; 29] = [
    FunctionEntry { name: "pow", arity: 2, func: pow_fn },
    FunctionEntry { name: "sin", arity: 1, func: sin_fn },
    FunctionEntry { name: "cos", arity: 1, func: cos_fn },
    FunctionEntry { name: "tan", arity: 1, func: tan_fn },
    FunctionEntry { name: "abs", arity: 1, func: abs_fn },
    FunctionEntry { name: "sqrt", arity: 1, func: sqrt_fn },
    FunctionEntry { name: "log", arity: 1, func: log_fn },
    FunctionEntry { name: "log10", arity: 1, func: log10_fn },
    FunctionEntry { name: "exp", arity: 1, func: exp_fn },
    FunctionEntry { name: "floor", arity: 1, func: floor_fn },
    FunctionEntry { name: "round", arity: 1, func: round_fn },
    FunctionEntry { name: "ceil", arity: 1, func: ceil_fn },
    FunctionEntry { name: "asin", arity: 1, func: asin_fn },
    FunctionEntry { name: "acos", arity: 1, func: acos_fn },
    FunctionEntry { name: "atan", arity: 1, func: atan_fn },
    FunctionEntry { name: "atan2", arity: 2, func: atan2_fn },
    FunctionEntry { name: "sinh", arity: 1, func: sinh_fn },
    FunctionEntry { name: "cosh", arity: 1, func: cosh_fn },
    FunctionEntry { name: "tanh", arity: 1, func: tanh_fn },
    FunctionEntry { name: "logb", arity: 1, func: logb_fn },
    FunctionEntry { name: "exp2", arity: 1, func: exp2_fn },
    FunctionEntry { name: "log2", arity: 1, func: log2_fn },
    FunctionEntry { name: "hypot", arity: 2, func: hypot_fn },
    FunctionEntry { name: "cbrt", arity: 1, func: cbrt_fn },
    FunctionEntry { name: "trunc", arity: 1, func: trunc_fn },
    FunctionEntry { name: "min", arity: 2, func: min_fn },
    FunctionEntry { name: "max", arity: 2, func: max_fn },
    FunctionEntry { name: "pi", arity: 0, func: pi_fn },
    FunctionEntry { name: "", arity: 0, func: pi_fn },
];

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

fn default_token() -> Token {
    Token {
        token_type: TokenType::End,
        value: None,
        int_value: None,
        var: None,
        op: None,
    }
}

fn token_with_int(value: i32) -> Token {
    Token {
        token_type: TokenType::Int,
        value: None,
        int_value: Some(value),
        var: None,
        op: None,
    }
}

fn token_with_float(value: f32) -> Token {
    Token {
        token_type: TokenType::Float,
        value: Some(value),
        int_value: None,
        var: None,
        op: None,
    }
}

fn token_with_op(op: char) -> Token {
    Token {
        token_type: TokenType::Op,
        value: None,
        int_value: None,
        var: None,
        op: Some(op),
    }
}

fn token_with_type(token_type: TokenType) -> Token {
    Token { token_type, ..default_token() }
}

fn token_with_func(func: ExprFunc) -> Token {
    Token {
        token_type: TokenType::Func,
        value: None,
        int_value: Some(func as i32),
        var: None,
        op: None,
    }
}

fn expr_func_from_i32(value: i32) -> ExprFunc {
    match value {
        -1 => ExprFunc::Unknown,
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
        28 => ExprFunc::NFuncs,
        _ => ExprFunc::Unknown,
    }
}

fn function_entry(func: ExprFunc) -> Option<&'static FunctionEntry> {
    let index = func as i32;
    if index < 0 {
        None
    } else {
        FUNCTION_TABLE.get(index as usize)
    }
}

fn function_lookup(s: &str) -> Option<&'static FunctionEntry> {
    FUNCTION_TABLE
        .iter()
        .take_while(|entry| !entry.name.is_empty())
        .find(|entry| entry.name.starts_with(s))
}

fn function_lookup_index(s: &str) -> Option<usize> {
    FUNCTION_TABLE
        .iter()
        .take_while(|entry| !entry.name.is_empty())
        .position(|entry| entry.name.starts_with(s))
}

fn expr_lex(s: Vec<&str>) -> Vec<Token> {
    let mut joined = String::new();
    for part in s {
        joined.push_str(part);
    }
    let mut pos = 0;
    let mut tokens = Vec::new();
    while let Ok(token) = lex_one(&joined, &mut pos) {
        let is_end = token.token_type == TokenType::End;
        tokens.push(token);
        if is_end {
            break;
        }
    }
    tokens
}

fn lex_one(input: &str, pos: &mut usize) -> Result<Token, ()> {
    let bytes = input.as_bytes();
    if *pos >= bytes.len() {
        return Ok(token_with_type(TokenType::End));
    }

    loop {
        if *pos >= bytes.len() {
            return Ok(token_with_type(TokenType::End));
        }
        match bytes[*pos] as char {
            ' ' | '\t' | '\r' | '\n' => *pos += 1,
            _ => break,
        }
    }

    if *pos >= bytes.len() {
        return Ok(token_with_type(TokenType::End));
    }

    let start = *pos;
    let mut c = bytes[*pos] as char;
    let mut integer_found = false;
    let mut integer_value = 0_i32;

    if c.is_ascii_digit() {
        while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_digit() {
            *pos += 1;
        }
        integer_value = input[start..*pos].parse::<i32>().map_err(|_| ())?;
        integer_found = true;
        if *pos >= bytes.len() || (bytes[*pos] as char) != '.' {
            return Ok(token_with_int(integer_value));
        }
        c = '.';
    }

    match c {
        '.' => {
            let dot_pos = *pos;
            *pos += 1;
            if *pos >= bytes.len() || !(bytes[*pos] as char).is_ascii_digit() {
                if integer_found {
                    return Ok(token_with_float(integer_value as f32));
                }
                return Err(());
            }
            while *pos < bytes.len() && (bytes[*pos] as char).is_ascii_digit() {
                *pos += 1;
            }
            let fraction = input[dot_pos..*pos].parse::<f32>().map_err(|_| ())?;
            return Ok(token_with_float(integer_value as f32 + fraction));
        }
        '+' | '-' | '/' | '*' | '=' => {
            *pos += 1;
            return Ok(token_with_op(c));
        }
        '(' => {
            *pos += 1;
            return Ok(token_with_type(TokenType::OpenParen));
        }
        ')' => {
            *pos += 1;
            return Ok(token_with_type(TokenType::CloseParen));
        }
        '[' => {
            *pos += 1;
            return Ok(token_with_type(TokenType::OpenSquare));
        }
        ']' => {
            *pos += 1;
            return Ok(token_with_type(TokenType::CloseSquare));
        }
        '{' => {
            *pos += 1;
            return Ok(token_with_type(TokenType::OpenCurly));
        }
        '}' => {
            *pos += 1;
            return Ok(token_with_type(TokenType::CloseCurly));
        }
        ',' => {
            *pos += 1;
            return Ok(token_with_type(TokenType::Comma));
        }
        'x' | 'y' if start + 1 == input.len()
            || !(bytes.get(start + 1).copied().map(char::from).unwrap_or('\0').is_ascii_alphanumeric()) =>
        {
            *pos += 1;
            return Ok(Token {
                token_type: TokenType::Var,
                value: None,
                int_value: None,
                var: Some(c),
                op: None,
            });
        }
        _ => {}
    }

    if !c.is_ascii_alphabetic() {
        println!("unknown character '{}' in lexer", c);
        return Err(());
    }

    while *pos < bytes.len() {
        let ch = bytes[*pos] as char;
        if ch.is_ascii_alphabetic() || ch.is_ascii_digit() {
            *pos += 1;
        } else {
            break;
        }
    }

    let ident = &input[start..*pos];
    let func = function_lookup_index(ident)
        .map(|index| expr_func_from_i32(index as i32))
        .unwrap_or(ExprFunc::Unknown);
    Ok(token_with_func(func))
}

#[derive(Clone, Copy)]
struct InternalNode {
    tok: Token,
    is_float: i32,
    history_index: i32,
    vector_index: i32,
}

type CompiledExpr = Vec<InternalNode>;

fn node_from_token(tok: Token, is_float: i32) -> InternalNode {
    InternalNode {
        tok,
        is_float,
        history_index: 0,
        vector_index: 0,
    }
}

fn append_token(expr: &mut CompiledExpr, tok: Token, is_float: i32) {
    expr.push(node_from_token(tok, is_float));
}

fn build_expr_chain(nodes: &[InternalNode]) -> ExprNode {
    fn build_from(nodes: &[InternalNode], index: usize) -> ExprNode {
        let node = nodes[index];
        ExprNode {
            tok: node.tok,
            is_float: node.is_float,
            history_index: node.history_index,
            vector_index: node.vector_index,
            next: if index + 1 < nodes.len() {
                Some(Arc::new(build_from(nodes, index + 1)))
            } else {
                None
            },
        }
    }

    if nodes.is_empty() {
        ExprNode::new()
    } else {
        build_from(nodes, 0)
    }
}

fn collect_expr_nodes(node: &ExprNode, out: &mut CompiledExpr) {
    out.push(InternalNode {
        tok: node.tok,
        is_float: node.is_float,
        history_index: node.history_index,
        vector_index: node.vector_index,
    });
    if let Some(next) = &node.next {
        collect_expr_nodes(next, out);
    }
}

fn flatten_expr(node: &ExprNode) -> CompiledExpr {
    let mut out = Vec::new();
    collect_expr_nodes(node, &mut out);
    out
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
#[derive(Clone, Copy, PartialEq, Eq)]
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
            tok: default_token(),
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
            let func = expr_func_from_i32(t.int_value.unwrap_or(-1));
            if let Some(entry) = function_entry(func) {
                print!("FUNC({})", entry.name);
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
    let nodes = flatten_expr(list);
    for (index, node) in nodes.iter().enumerate() {
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
        if index + 1 < nodes.len() {
            print!(" ");
        }
    }
}

fn printexpr(s: &str, list: &MapperExpr) {
    printexprnode(s, &list.node);
}

fn printstack(stack: &stack_obj_t, stack_size: i32) {
    let _ = stack_size;
    match stack {
        stack_obj_t::State(_) => print!("STATE"),
        stack_obj_t::Node(node) => printexprnode("", node),
    }
}

fn collapse_expr_to_left(plhs: &mut ExprNode, constant_folding: i32) {
    let mut nodes = flatten_expr(plhs);
    if constant_folding != 0
        && !nodes.iter().any(|node| node.tok.token_type == TokenType::Var)
        && !nodes.is_empty()
    {
        if let Some(value) = eval_compiled_nodes(
            &nodes,
            &MapperExpr {
                node: ExprNode::new(),
                vector_size: 1,
                history_size: 1,
                history_pos: 0,
                input_history: vec![MapperSignalValue::I32(0)],
                output_history: vec![MapperSignalValue::I32(0)],
            },
        ) {
            let is_float = nodes.last().map(|node| node.is_float != 0).unwrap_or(false);
            nodes.clear();
            nodes.push(node_from_token(
                if is_float {
                    token_with_float(value.as_f32().unwrap_or(0.0))
                } else {
                    token_with_int(value.as_i32().unwrap_or(0))
                },
                if is_float { 1 } else { 0 },
            ));
        }
    }
    *plhs = build_expr_chain(&nodes);
}

fn collapse_internal_expr_to_left(plhs: &mut CompiledExpr, mut rhs: CompiledExpr, constant_folding: bool) {
    if plhs.is_empty() || rhs.is_empty() {
        return;
    }

    let mut refvar = plhs.iter().any(|node| node.tok.token_type == TokenType::Var)
        || rhs.iter().any(|node| node.tok.token_type == TokenType::Var);

    let lhs_last_index = plhs.len() - 1;
    let rhs_last_index = rhs.len() - 1;
    let is_float = plhs[lhs_last_index].is_float != 0 || rhs[rhs_last_index].is_float != 0;

    if plhs[lhs_last_index].is_float != 0 && rhs[rhs_last_index].is_float == 0 {
        append_token(&mut rhs, token_with_type(TokenType::ToFloat), 1);
    } else if plhs[lhs_last_index].is_float == 0 && rhs[rhs_last_index].is_float != 0 {
        let lhs_tail = plhs[lhs_last_index];
        plhs[lhs_last_index] = node_from_token(token_with_type(TokenType::ToFloat), 1);
        plhs.push(lhs_tail);
        if let Some(last) = plhs.last_mut() {
            last.is_float = 1;
        }
    }

    let lhs_insert_at = plhs.len().saturating_sub(1);
    plhs.splice(lhs_insert_at..lhs_insert_at, rhs);

    if constant_folding && !refvar {
        if let Some(value) = eval_compiled_nodes(
            plhs,
            &MapperExpr {
                node: ExprNode::new(),
                vector_size: 1,
                history_size: 1,
                history_pos: 0,
                input_history: vec![MapperSignalValue::I32(0)],
                output_history: vec![MapperSignalValue::I32(0)],
            },
        ) {
            plhs.truncate(1);
            plhs[0].is_float = if is_float { 1 } else { 0 };
            plhs[0].tok = if is_float {
                token_with_float(value.as_f32().unwrap_or(0.0))
            } else {
                token_with_int(value.as_i32().unwrap_or(0))
            };
        }
    }

    refvar = refvar && constant_folding;
    let _ = refvar;
}

fn value_for_node_type(value: MapperSignalValue, is_float: i32) -> MapperSignalValue {
    if is_float != 0 {
        MapperSignalValue::F(value.as_f32().unwrap_or(0.0))
    } else {
        MapperSignalValue::I32(value.as_i32().unwrap_or(0))
    }
}

fn eval_compiled_nodes(nodes: &[InternalNode], expr: &MapperExpr) -> Option<MapperSignalValue> {
    let mut stack: Vec<MapperSignalValue> = Vec::with_capacity(STACK_SIZE);

    for node in nodes {
        match node.tok.token_type {
            TokenType::Int => stack.push(MapperSignalValue::I32(node.tok.int_value.unwrap_or(0))),
            TokenType::Float => stack.push(MapperSignalValue::F(node.tok.value.unwrap_or(0.0))),
            TokenType::Var => {
                let history_size = expr.history_size.max(1);
                let index = (node.history_index + expr.history_pos + history_size).rem_euclid(history_size) as usize;
                let value = match node.tok.var.unwrap_or('\0') {
                    'x' => {
                        let vector_size = expr.vector_size.max(1) as usize;
                        let input_index = index * vector_size + node.vector_index.max(0) as usize;
                        expr.input_history
                            .get(input_index)
                            .copied()
                            .unwrap_or(MapperSignalValue::I32(0))
                    }
                    'y' => expr
                        .output_history
                        .get(index)
                        .copied()
                        .unwrap_or(MapperSignalValue::I32(0)),
                    _ => return None,
                };
                stack.push(value_for_node_type(value, node.is_float));
            }
            TokenType::ToFloat => {
                let value = stack.pop()?;
                stack.push(MapperSignalValue::F(value.as_f32()?));
            }
            TokenType::ToInt32 => {
                let value = stack.pop()?;
                stack.push(MapperSignalValue::I32(value.as_i32()?));
            }
            TokenType::Op => {
                let right = stack.pop()?;
                let left = stack.pop()?;
                if node.is_float != 0 {
                    let left = left.as_f32()?;
                    let right = right.as_f32()?;
                    let value = match node.tok.op.unwrap_or('\0') {
                        '+' => left + right,
                        '-' => left - right,
                        '*' => left * right,
                        '/' => left / right,
                        _ => return None,
                    };
                    stack.push(MapperSignalValue::F(value));
                } else {
                    let left = left.as_i32()?;
                    let right = right.as_i32()?;
                    let value = match node.tok.op.unwrap_or('\0') {
                        '+' => left + right,
                        '-' => left - right,
                        '*' => left * right,
                        '/' => left / right,
                        _ => return None,
                    };
                    stack.push(MapperSignalValue::I32(value));
                }
            }
            TokenType::Func => {
                let func = expr_func_from_i32(node.tok.int_value.unwrap_or(-1));
                let entry = function_entry(func)?;
                let result = match entry.arity {
                    0 => (entry.func)(0.0, 0.0),
                    1 => {
                        let value = stack.pop()?.as_f32()?;
                        (entry.func)(value, 0.0)
                    }
                    2 => {
                        let right = stack.pop()?.as_f32()?;
                        let left = stack.pop()?.as_f32()?;
                        (entry.func)(left, right)
                    }
                    _ => return None,
                };
                stack.push(MapperSignalValue::F(result));
            }
            _ => return None,
        }
    }

    stack.pop()
}

#[derive(Clone)]
struct ParsedExpr {
    nodes: CompiledExpr,
    is_float: bool,
    has_var: bool,
}

impl ParsedExpr {
    fn from_node(node: InternalNode) -> Self {
        Self {
            nodes: vec![node],
            is_float: node.is_float != 0,
            has_var: node.tok.token_type == TokenType::Var,
        }
    }
}

fn fold_constant_expr(expr: &mut ParsedExpr) {
    if expr.has_var || expr.nodes.is_empty() {
        return;
    }

    if let Some(value) = eval_compiled_nodes(
        &expr.nodes,
        &MapperExpr {
            node: ExprNode::new(),
            vector_size: 1,
            history_size: 1,
            history_pos: 0,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        },
    ) {
        expr.nodes = vec![node_from_token(
            if expr.is_float {
                token_with_float(value.as_f32().unwrap_or(0.0))
            } else {
                token_with_int(value.as_i32().unwrap_or(0))
            },
            if expr.is_float { 1 } else { 0 },
        )];
    }
}

fn promote_to_float(expr: &mut ParsedExpr) {
    if !expr.is_float {
        expr.nodes.push(node_from_token(token_with_type(TokenType::ToFloat), 1));
        expr.is_float = true;
    }
}

fn combine_binary_expr(mut lhs: ParsedExpr, mut rhs: ParsedExpr, op: char) -> ParsedExpr {
    let is_float = lhs.is_float || rhs.is_float;
    if is_float {
        promote_to_float(&mut lhs);
        promote_to_float(&mut rhs);
    }

    let mut nodes = lhs.nodes;
    nodes.extend(rhs.nodes);
    nodes.push(node_from_token(token_with_op(op), if is_float { 1 } else { 0 }));

    let mut out = ParsedExpr {
        nodes,
        is_float,
        has_var: lhs.has_var || rhs.has_var,
    };
    fold_constant_expr(&mut out);
    out
}

fn parse_constant_index(expr: ParsedExpr) -> Result<i32, &'static str> {
    if expr.has_var {
        return Err("Unexpected variable reference.");
    }
    let value = eval_compiled_nodes(
        &expr.nodes,
        &MapperExpr {
            node: ExprNode::new(),
            vector_size: 1,
            history_size: 1,
            history_pos: 0,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        },
    )
    .ok_or("Expected value.")?;

    Ok(if expr.is_float {
        value.as_f32().unwrap_or(0.0) as i32
    } else {
        value.as_i32().unwrap_or(0)
    })
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    lookahead: Option<Token>,
    input_is_float: i32,
    vector_size: i32,
    oldest_samps: f32,
    vars_allowed: bool,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, input_is_float: i32, vector_size: i32) -> Self {
        Self {
            input,
            pos: 0,
            lookahead: None,
            input_is_float,
            vector_size,
            oldest_samps: 0.0,
            vars_allowed: true,
        }
    }

    fn peek(&mut self) -> Result<Token, &'static str> {
        if let Some(token) = self.lookahead {
            return Ok(token);
        }
        let token = lex_one(self.input, &mut self.pos).map_err(|_| "Error in lexical analysis.")?;
        self.lookahead = Some(token);
        Ok(token)
    }

    fn next(&mut self) -> Result<Token, &'static str> {
        let token = self.peek()?;
        self.lookahead = None;
        Ok(token)
    }

    fn expect(&mut self, token_type: TokenType, message: &'static str) -> Result<Token, &'static str> {
        let token = self.next()?;
        if token.token_type == token_type {
            Ok(token)
        } else {
            Err(message)
        }
    }

    fn parse_assignment(&mut self) -> Result<ParsedExpr, &'static str> {
        let first = self.next()?;
        if first.token_type != TokenType::Var || first.var != Some('y') {
            return Err("Error in y= prefix.");
        }

        let eq = self.next()?;
        if eq.token_type != TokenType::Op || eq.op != Some('=') {
            return Err("Error in y= prefix.");
        }

        let expr = self.parse_expr()?;
        let end = self.next()?;
        if end.token_type != TokenType::End {
            return Err("Expected END.");
        }
        Ok(expr)
    }

    fn parse_expr(&mut self) -> Result<ParsedExpr, &'static str> {
        let mut expr = self.parse_term()?;
        loop {
            let token = self.peek()?;
            match token.token_type {
                TokenType::Op if matches!(token.op, Some('+') | Some('-')) => {
                    let op = token.op.unwrap_or('+');
                    self.next()?;
                    let rhs = self.parse_term()?;
                    expr = combine_binary_expr(expr, rhs, op);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<ParsedExpr, &'static str> {
        let mut expr = self.parse_value()?;
        loop {
            let token = self.peek()?;
            match token.token_type {
                TokenType::Op if matches!(token.op, Some('*') | Some('/')) => {
                    let op = token.op.unwrap_or('*');
                    self.next()?;
                    let rhs = self.parse_value()?;
                    expr = combine_binary_expr(expr, rhs, op);
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_value(&mut self) -> Result<ParsedExpr, &'static str> {
        let token = self.next()?;
        match token.token_type {
            TokenType::Int => Ok(ParsedExpr::from_node(node_from_token(token, 0))),
            TokenType::Float => Ok(ParsedExpr::from_node(node_from_token(token, 1))),
            TokenType::Var => self.parse_var(token),
            TokenType::OpenParen => {
                let expr = self.parse_expr()?;
                self.expect(TokenType::CloseParen, "Expected ')'.")?;
                Ok(expr)
            }
            TokenType::Func => self.parse_function(token),
            TokenType::Op if token.op == Some('-') => {
                let expr = self.parse_value()?;
                Ok(combine_binary_expr(
                    ParsedExpr::from_node(node_from_token(token_with_int(0), 0)),
                    expr,
                    '-',
                ))
            }
            _ => Err("Expected value."),
        }
    }

    fn parse_var(&mut self, token: Token) -> Result<ParsedExpr, &'static str> {
        if !self.vars_allowed {
            return Err("Unexpected variable reference.");
        }

        let mut node = node_from_token(token, self.input_is_float);
        let mut saw_vector = false;
        let mut saw_history = false;

        loop {
            let next = self.peek()?;
            match next.token_type {
                TokenType::OpenSquare if !saw_vector => {
                    self.next()?;
                    let previous = self.vars_allowed;
                    self.vars_allowed = false;
                    let expr = self.parse_expr()?;
                    self.vars_allowed = previous;
                    self.expect(TokenType::CloseSquare, "Expected ']'.")?;
                    node.vector_index = parse_constant_index(expr)?;
                    if node.vector_index > 0 {
                        return Err("Vector indexing not yet implemented.");
                    }
                    if node.vector_index < 0 || node.vector_index >= self.vector_size {
                        return Err("Vector index outside input size.");
                    }
                    saw_vector = true;
                }
                TokenType::OpenCurly if !saw_history => {
                    self.next()?;
                    let previous = self.vars_allowed;
                    self.vars_allowed = false;
                    let expr = self.parse_expr()?;
                    self.vars_allowed = previous;
                    self.expect(TokenType::CloseCurly, "Expected '}'.")?;
                    node.history_index = parse_constant_index(expr)?;
                    if self.oldest_samps > node.history_index as f32 {
                        self.oldest_samps = node.history_index as f32;
                    }
                    saw_history = true;
                }
                _ => break,
            }
        }

        Ok(ParsedExpr::from_node(node))
    }

    fn parse_function(&mut self, token: Token) -> Result<ParsedExpr, &'static str> {
        let func = expr_func_from_i32(token.int_value.unwrap_or(-1));
        if func == ExprFunc::Unknown {
            return Err("Unknown function.");
        }
        let entry = function_entry(func).ok_or("Unknown function.")?;

        let mut args = Vec::new();
        if entry.arity > 0 {
            self.expect(TokenType::OpenParen, "Expected '('.")?;
            for index in 0..entry.arity {
                let arg = self.parse_expr()?;
                args.push(arg);
                if index + 1 < entry.arity {
                    self.expect(TokenType::Comma, "Expected ','.")?;
                }
            }
            self.expect(TokenType::CloseParen, "Expected ')'.")?;
        }

        let mut nodes = Vec::new();
        let mut has_var = false;
        for mut arg in args {
            promote_to_float(&mut arg);
            has_var |= arg.has_var;
            nodes.extend(arg.nodes);
        }
        nodes.push(node_from_token(token, 1));

        let mut expr = ParsedExpr {
            nodes,
            is_float: true,
            has_var,
        };
        fold_constant_expr(&mut expr);
        Ok(expr)
    }
}

pub fn mapper_expr_new_from_string(
    s: &str,
    input_is_float: i32,
    output_is_float: i32,
    vector_size: i32,
) -> MapperExpr {
    fn default_expr() -> MapperExpr {
        MapperExpr {
            node: ExprNode::new(),
            vector_size: 1,
            history_size: 1,
            history_pos: -1,
            input_history: vec![MapperSignalValue::I32(0)],
            output_history: vec![MapperSignalValue::I32(0)],
        }
    }

    if s.is_empty() {
        return default_expr();
    }
    let _ = expr_lex(vec![s]);
    let mut parser = Parser::new(s, input_is_float, vector_size.max(1));
    let Ok(mut parsed) = parser.parse_assignment() else {
        return default_expr();
    };

    if output_is_float != 0 {
        promote_to_float(&mut parsed);
    }

    if vector_size > 1
        && parsed
            .nodes
            .iter()
            .any(|node| node.tok.token_type == TokenType::Var && node.vector_index > 0)
    {
        trace!("vector indexing not yet implemented");
        return default_expr();
    }

    if parser.oldest_samps < -100.0 {
        trace!("Expression contains history reference of {}", parser.oldest_samps);
        return default_expr();
    }

    let history_size = ((-parser.oldest_samps).ceil() as i32 + 1).max(1);
    MapperExpr {
        node: build_expr_chain(&parsed.nodes),
        vector_size: vector_size.max(1),
        history_size,
        history_pos: -1,
        input_history: vec![MapperSignalValue::I32(0); vector_size.max(1) as usize * history_size as usize],
        output_history: vec![MapperSignalValue::I32(0); history_size as usize],
    }
}

pub fn mapper_expr_evaluate<'a>(
    mapper: &mut MapperExpr,
    input: &'a MapperSignalValue,
) -> MapperSignalValue {
    let history_size = mapper.history_size.max(1);
    mapper.history_pos = (mapper.history_pos + 1).rem_euclid(history_size);

    let vector_size = mapper.vector_size.max(1) as usize;
    let input_index = mapper.history_pos as usize * vector_size;
    if let Some(slot) = mapper.input_history.get_mut(input_index) {
        *slot = *input;
    }

    let compiled = flatten_expr(&mapper.node);
    let result = eval_compiled_nodes(&compiled, mapper).unwrap_or(MapperSignalValue::I32(0));

    if let Some(slot) = mapper.output_history.get_mut(mapper.history_pos as usize) {
        *slot = result;
    }

    result
}
