use crate::{vm, lexer, parser, ast};
use crate::token::TokenType;
pub const SIMPLE_LANG_COMPILER_H: bool = true;

/// Replicates: Instruction* compile_asts(ASTNode** asts, int* instr_count);
/// In Rust: returns a vector of Instructions, updates an integer by reference.
pub fn compile_asts(asts: &[Box<ast::ASTNode>], instr_count: &mut i32) -> Vec<vm::Instruction> {
    *instr_count = 0;
    let mut all_instr: Vec<vm::Instruction> = Vec::new();
    for a in asts.iter() {
        let mut count: i32 = 0;
        let chunk = compile_statement(a, &mut count);
        for i in 0..count as usize {
            all_instr.push(chunk[i].clone());
        }
        *instr_count += count;
    }
    all_instr
}

/// Replicates: Instruction* compile(const char* source, int* instr_count);
pub fn compile(source: &str, instr_count: &mut i32) -> Vec<vm::Instruction> {
    let tokens = lexer::tokenize(source);
    let ast_nodes = parser::parse(&tokens);
    compile_asts(&ast_nodes, instr_count)
}

/// Replicates: void compile_expression(ASTNode* node, Instruction* instructions, int* count);
/// In Rust: modifies a mutable vector of Instructions and a count reference.
pub fn compile_expression(
    node: &ast::ASTNode,
    instructions: &mut Vec<vm::Instruction>,
    count: &mut i32,
) {
    match node.type_ {
        TokenType::TOKEN_INT => {
            emit(instructions, count, vm::OpCode::LOAD_CONST, &node.value);
        }
        TokenType::TOKEN_IDENTIFIER => {
            emit(instructions, count, vm::OpCode::LOAD_NAME, &node.value);
        }
        TokenType::TOKEN_PLUS => {
            if let Some(ref left) = node.left {
                compile_expression(left, instructions, count);
            }
            if let Some(ref right) = node.right {
                compile_expression(right, instructions, count);
            }
            emit(instructions, count, vm::OpCode::BINARY_ADD, "");
        }
        TokenType::TOKEN_MINUS => {
            if let Some(ref left) = node.left {
                compile_expression(left, instructions, count);
            }
            if let Some(ref right) = node.right {
                compile_expression(right, instructions, count);
            }
            emit(instructions, count, vm::OpCode::BINARY_SUB, "");
        }
        _ => {}
    }
}

/// Replicates: Instruction* compile_statement(ASTNode* ast, int* instr_count);
pub fn compile_statement(ast: &ast::ASTNode, instr_count: &mut i32) -> Vec<vm::Instruction> {
    let mut instructions: Vec<vm::Instruction> = Vec::new();
    *instr_count = 0;

    match ast.type_ {
        TokenType::TOKEN_LET | TokenType::TOKEN_ASSIGN => {
            if let Some(ref left) = ast.left {
                compile_expression(left, &mut instructions, instr_count);
            }
            emit(
                &mut instructions,
                instr_count,
                vm::OpCode::STORE_NAME,
                &ast.value,
            );
        }
        TokenType::TOKEN_DIS => {
            if let Some(ref left) = ast.left {
                compile_expression(left, &mut instructions, instr_count);
            }
            emit(
                &mut instructions,
                instr_count,
                vm::OpCode::STK_DIS,
                &ast.value,
            );
        }
        _ => {}
    }

    instructions
}

/// Replicates: void emit(Instruction* instructions, int* count, OpCode opcode, const char* operand);
pub fn emit(
    instructions: &mut Vec<vm::Instruction>,
    count: &mut i32,
    opcode: vm::OpCode,
    operand: &str,
) {
    instructions.push(vm::new_instruction(opcode, operand));
    *count += 1;
}
