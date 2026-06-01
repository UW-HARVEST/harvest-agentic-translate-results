use crate::{vm, lexer, parser, ast, token};
pub const SIMPLE_LANG_COMPILER_H: bool = true;

/// Replicates: void emit(Instruction* instructions, int* count, OpCode opcode, const char* operand);
pub fn emit(
    instructions: &mut Vec<vm::Instruction>,
    count: &mut i32,
    opcode: vm::OpCode,
    operand: &str,
) {
    let instr = vm::new_instruction(opcode, operand);
    if (*count as usize) < instructions.len() {
        instructions[*count as usize] = instr;
    } else {
        instructions.push(instr);
    }
    *count += 1;
}

/// Replicates: void compile_expression(ASTNode* node, Instruction* instructions, int* count);
pub fn compile_expression(
    node: &ast::ASTNode,
    instructions: &mut Vec<vm::Instruction>,
    count: &mut i32,
) {
    match node.type_ {
        token::TokenType::TOKEN_INT => {
            emit(instructions, count, vm::OpCode::LOAD_CONST, &node.value);
        }
        token::TokenType::TOKEN_IDENTIFIER => {
            emit(instructions, count, vm::OpCode::LOAD_NAME, &node.value);
        }
        token::TokenType::TOKEN_PLUS => {
            if let Some(left) = &node.left {
                compile_expression(left, instructions, count);
            }
            if let Some(right) = &node.right {
                compile_expression(right, instructions, count);
            }
            emit(instructions, count, vm::OpCode::BINARY_ADD, "");
        }
        token::TokenType::TOKEN_MINUS => {
            if let Some(left) = &node.left {
                compile_expression(left, instructions, count);
            }
            if let Some(right) = &node.right {
                compile_expression(right, instructions, count);
            }
            emit(instructions, count, vm::OpCode::BINARY_SUB, "");
        }
        _ => {}
    }
}

/// Replicates: Instruction* compile_statement(ASTNode* ast, int* instr_count);
pub fn compile_statement(ast_node: &ast::ASTNode, instr_count: &mut i32) -> Vec<vm::Instruction> {
    let mut instructions: Vec<vm::Instruction> = Vec::new();
    *instr_count = 0;

    match ast_node.type_ {
        token::TokenType::TOKEN_LET | token::TokenType::TOKEN_ASSIGN => {
            if let Some(left) = &ast_node.left {
                compile_expression(left, &mut instructions, instr_count);
            }
            emit(
                &mut instructions,
                instr_count,
                vm::OpCode::STORE_NAME,
                &ast_node.value,
            );
        }
        token::TokenType::TOKEN_DIS => {
            if let Some(left) = &ast_node.left {
                compile_expression(left, &mut instructions, instr_count);
            }
            emit(
                &mut instructions,
                instr_count,
                vm::OpCode::STK_DIS,
                &ast_node.value,
            );
        }
        _ => {}
    }

    instructions
}

/// Replicates: Instruction* compile_asts(ASTNode** asts, int* instr_count);
pub fn compile_asts(asts: &[Box<ast::ASTNode>], instr_count: &mut i32) -> Vec<vm::Instruction> {
    *instr_count = 0;

    let mut all_instr: Vec<vm::Instruction> = Vec::new();
    for ast_node in asts.iter() {
        let mut count: i32 = 0;
        let stmt_instr = compile_statement(ast_node, &mut count);
        for j in 0..count as usize {
            all_instr.push(stmt_instr[j].clone());
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
