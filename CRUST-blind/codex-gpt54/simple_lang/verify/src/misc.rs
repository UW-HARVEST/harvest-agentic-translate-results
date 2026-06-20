use crate::{vm, ast, token, settings};
pub const SIMPLE_LANG_MISC_H: bool = true;

fn get_token_type_name(token_type: &token::TokenType) -> &'static str {
    match token_type {
        token::TokenType::TOKEN_INT => "TOKEN_INT",
        token::TokenType::TOKEN_IDENTIFIER => "TOKEN_IDENTIFIER",
        token::TokenType::TOKEN_ASSIGN => "TOKEN_ASSIGN",
        token::TokenType::TOKEN_PLUS => "TOKEN_PLUS",
        token::TokenType::TOKEN_MINUS => "TOKEN_MINUS",
        token::TokenType::TOKEN_SEMICOLON => "TOKEN_SEMICOLON",
        token::TokenType::TOKEN_LET => "TOKEN_LET",
        token::TokenType::TOKEN_EOF => "TOKEN_EOF",
        token::TokenType::TOKEN_DIS => "TOKEN_DIS",
    }
}

/// Replicates: void show_opcode(Instruction* instruction, int instr_count);
pub fn show_opcode(instruction: &vm::Instruction, instr_count: i32) {
    println!("bytecode:");
    println!(
        "{:<15} \t ({})",
        get_opcode_name(instruction.opcode),
        instruction.operand
    );
    for _ in 1..instr_count {
        println!("{:<15} \t *", get_opcode_name(instruction.opcode));
    }
    println!();
}
/// Replicates: void show_tokens(Token* tokens);
/// Interpreted as working with a slice of tokens in Rust.
pub fn show_tokens(tokens: &[token::Token]) {
    println!("tokens:");
    for token in tokens {
        if token.token_type == token::TokenType::TOKEN_EOF {
            break;
        }
        println!("{:<15} \t ({})", get_token_type_name(&token.token_type), token.value);
    }
    println!();
}
/// Replicates: void print_asts(ASTNode** asts);
/// Interpreted as working with a slice of ASTNode pointers in Rust.
pub fn print_asts(asts: &[Box<ast::ASTNode>]) {
    println!("AST:");
    for ast in asts {
        print_ast(ast, 0);
        println!();
    }
    println!();
}
/// Replicates: char* read_file(const char* source_path);
pub fn read_file(source_path: &str) -> String {
    match std::fs::read_to_string(source_path) {
        Ok(contents) => contents,
        Err(err) => {
            eprintln!("Could not open file: {err}");
            std::process::exit(-1);
        }
    }
}
/// Replicates: void print_ast(ASTNode* node, int level);
pub fn print_ast(node: &ast::ASTNode, level: i32) {
    for _ in 0..level {
        print!("  ");
    }

    match node.type_ {
        token::TokenType::TOKEN_INT => println!("INT: {}", node.value),
        token::TokenType::TOKEN_IDENTIFIER => println!("IDENTIFIER: {}", node.value),
        token::TokenType::TOKEN_ASSIGN => println!("ASSIGN: {}", node.value),
        token::TokenType::TOKEN_LET => println!("LET: {}", node.value),
        token::TokenType::TOKEN_PLUS => println!("PLUS"),
        token::TokenType::TOKEN_MINUS => println!("MINUS"),
        token::TokenType::TOKEN_DIS => println!("DISPLAY"),
        _ => println!("UNKNOWN"),
    }

    if let Some(left) = node.left.as_deref() {
        print_ast(left, level + 1);
    }
    if let Some(right) = node.right.as_deref() {
        print_ast(right, level + 1);
    }
}
/// Replicates: char* get_opcode_name(OpCode opcode);
pub fn get_opcode_name(opcode: vm::OpCode) -> String {
    match opcode {
        vm::OpCode::LOAD_CONST => "LOAD_CONST",
        vm::OpCode::LOAD_NAME => "LOAD_NAME",
        vm::OpCode::STORE_NAME => "STORE_NAME",
        vm::OpCode::BINARY_SUB => "BINARY_SUB",
        vm::OpCode::BINARY_ADD => "BINARY_ADD",
        vm::OpCode::STK_DIS => "STK_DIS",
    }
    .to_string()
}
