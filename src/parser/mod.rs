// src/parser/mod.rs
pub mod ast;

use crate::lexer::Lexer;
use crate::lexer::token::Token;
use crate::parser::ast::*;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        Parser {
            lexer,
            current_token,
        }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn expect(&mut self, token: Token) {
        if self.current_token == token {
            self.advance();
        } else {
            panic!("Expected {:?}, but found {:?}", token, self.current_token);
        }
    }

    /// Parses the entire file into a Module AST
    pub fn parse_module(&mut self) -> Module {
        let mut module = Module {
            path: String::new(),
            imports: vec![],
            body: vec![],
            exports: vec![],
        };

        // Parse Path Comment (must be at the top)
        if let Token::PathComment(path) = &self.current_token {
            module.path = path.clone();
            self.advance();
        }

        // Parse Statements until EOF
        while self.current_token != Token::Eof {
            match self.current_token {
                // If it starts with '{', it must be an Import
                Token::LBrace => {
                    module.imports.push(self.parse_import());
                }
                // Otherwise, it's a regular statement
                _ => {
                    module.body.push(self.parse_statement());
                }
            }
        }

        module
    }

    fn parse_import(&mut self) -> Import {
        self.advance(); // consume '{'
        let mut items = vec![];

        // Parse items inside { ... }
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if let Token::Identifier(name) = self.current_token.clone() {
                self.advance();
                let mut alias = None;

                // Check for renaming: name = alias
                if self.current_token == Token::Equals {
                    self.advance();
                    if let Token::Identifier(alias_name) = self.current_token.clone() {
                        alias = Some(alias_name);
                        self.advance();
                    }
                }
                items.push(ImportItem { name, alias });

                // Skip optional comma
                if self.current_token == Token::Comma {
                    self.advance();
                }
            } else {
                break;
            }
        }

        self.expect(Token::RBrace);
        self.expect(Token::Equals);

        // 2. Parse Source Path (e.g., @fer/std, ./constants)
        let source = self.parse_import_source();

        Import { items, source }
    }

    fn parse_import_source(&mut self) -> String {
        let mut path = String::new();
        // Here we handle @, ., / to build the source string
        // (Implementation same as before, condensed for brevity)
        while matches!(
            self.current_token,
            Token::At | Token::Dot | Token::Slash | Token::Identifier(_)
        ) {
            match &self.current_token {
                Token::At => path.push('@'),
                Token::Dot => path.push('.'),
                Token::Slash => path.push('/'),
                Token::Identifier(s) => path.push_str(s),
                _ => break,
            }
            self.advance();
        }
        path
    }

    /// Parses a single statement.
    /// In Fer, a statement can be a Declaration, a Type Definition, or just an Expression.
    fn parse_statement(&mut self) -> Statement {
        match &self.current_token {
            // Case 1: Potentially a Declaration or Type Definition (name = ...)
            Token::Identifier(identifier_name) => {
                let name = identifier_name.clone();

                // For now, we'll use a simple "peek-like" logic by checking current token after identifier
                // Actually, the cleanest way is to check the current_token after advancing past the ID
                self.advance();

                if self.current_token == Token::Equals {
                    self.advance(); // consume '='

                    match self.current_token {
                        Token::Enum => self.parse_enum_definition(name),
                        Token::Struct => self.parse_struct_definition(name),
                        _ => {
                            let value = self.parse_expression();
                            Statement::Declaration {
                                name,
                                value,
                                is_mut: false,
                            }
                        }
                    }
                } else if self.current_token == Token::LParen {
                    // It's a function call: print(...)
                    // We've already advanced past the name, so we reconstruct the call
                    self.advance(); // consume '('
                    let mut arguments = vec![];
                    if self.current_token != Token::RParen {
                        arguments.push(self.parse_expression());
                        while self.current_token == Token::Comma {
                            self.advance();
                            arguments.push(self.parse_expression());
                        }
                    }
                    self.expect(Token::RParen);

                    Statement::Expression(Expression::Call {
                        callee: Box::new(Expression::Identifier(name)),
                        args: arguments,
                    })
                } else {
                    // It's just a standalone identifier expression (e.g., just a variable name)
                    // We return it as an expression statement.
                    // Note: In a more complex parser, we'd handle binary ops here too.
                    Statement::Expression(Expression::Identifier(name))
                }
            }

            // Case 2: It's an expression statement starting with something else (like a Number or String)
            _ => {
                let expression = self.parse_expression();
                Statement::Expression(expression)
            }
        }
    }

    fn parse_enum_definition(&mut self, name: String) -> Statement {
        self.advance(); // consume 'enum'
        self.expect(Token::LBrace);
        let mut variants = vec![];
        while let Token::Identifier(v) = &self.current_token {
            variants.push(v.clone());
            self.advance();
        }
        self.expect(Token::RBrace);
        Statement::TypeDefinition {
            name,
            kind: TypeKind::Enum(variants),
        }
    }

    fn parse_struct_definition(&mut self, name: String) -> Statement {
        self.advance(); // consume 'struct'
        self.expect(Token::LBrace);
        let mut fields = vec![];
        while let Token::Identifier(field_name) = self.current_token.clone() {
            self.advance();
            self.expect(Token::Equals);
            if let Token::Identifier(type_name) = self.current_token.clone() {
                fields.push(Field {
                    name: field_name,
                    type_name,
                });
                self.advance();
            }
        }
        self.expect(Token::RBrace);
        Statement::TypeDefinition {
            name,
            kind: TypeKind::Struct(fields),
        }
    }

    // Expression Entry
    // Process the basic expression and check if it is followed by a Match block.
    fn parse_expression(&mut self) -> Expression {
        let mut left = self.parse_primary_expression();

        // Loop through consecutive binary operations, supporting operations like 1 + 1 * 2, etc.
        while matches!(
            self.current_token,
            Token::Plus | Token::Minus | Token::Star | Token::Slash
        ) {
            let op = match self.current_token {
                Token::Plus => Op::Add,
                Token::Minus => Op::Sub,
                Token::Star => Op::Mul,
                Token::Slash => Op::Div,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_primary_expression();
            left = Expression::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        if self.current_token == Token::LBrace {
            left = self.parse_match_expression(left);
        }
        left
    }

    /// Primary expressions are the building blocks: literals, IDs, or grouped expressions.
    fn parse_primary_expression(&mut self) -> Expression {
        match &self.current_token {
            // Strings are always parsed as InterpolatedString for consistency
            Token::StringLit(raw_content) => {
                let content = raw_content.clone();
                self.advance();
                self.parse_interpolated_string(content)
            }

            // Numbers are literals
            Token::Number(number_value) => {
                let value = *number_value;
                self.advance();
                Expression::Literal(Literal::Int(value))
            }

            // Identifiers (when appearing inside an expression)
            Token::Identifier(identifier_name) => {
                let name = identifier_name.clone();
                self.advance();

                // Handle function calls within expressions
                if self.current_token == Token::LParen {
                    self.advance();
                    let mut arguments = vec![];
                    if self.current_token != Token::RParen {
                        arguments.push(self.parse_expression());
                        while self.current_token == Token::Comma {
                            self.advance();
                            arguments.push(self.parse_expression());
                        }
                    }
                    self.expect(Token::RParen);
                    Expression::Call {
                        callee: Box::new(Expression::Identifier(name)),
                        args: arguments,
                    }
                } else {
                    Expression::Identifier(name)
                }
            }

            Token::LBracket => Expression::Literal(self.parse_literal()),

            _ => panic!(
                "Expected primary expression, found {:?}",
                self.current_token
            ),
        }
    }

    fn parse_literal(&mut self) -> Literal {
        match self.current_token.clone() {
            Token::Number(n) => {
                self.advance();
                Literal::Int(n)
            }
            Token::StringLit(raw) => {
                self.advance();

                Literal::String(raw)
            }
            Token::LBracket => {
                self.advance();
                let mut elms = vec![];
                while self.current_token != Token::RBracket {
                    elms.push(self.parse_expression());
                }
                self.advance();
                Literal::Array(elms)
            }
            _ => panic!("Expected literal value, found {:?}", self.current_token),
        }
    }

    /// Real recursive parser for strings. No abbreviations!
    fn parse_interpolated_string(&mut self, raw_content: String) -> Expression {
        let processed_content = self.preprocess_multiline_string(raw_content);
        let mut string_parts = vec![];
        let mut current_plain_text = String::new();
        let mut characters = processed_content.chars().peekable();

        while let Some(character) = characters.next() {
            if character == '{' {
                // If there's pending plain text, wrap it as a Literal Expression
                if !current_plain_text.is_empty() {
                    string_parts.push(Expression::Literal(Literal::String(
                        current_plain_text.clone(),
                    )));
                    current_plain_text.clear();
                }

                // Extract content inside {} with nested brace support
                let mut expression_inner_text = String::new();
                let mut brace_depth = 1;
                while let Some(next_character) = characters.next() {
                    if next_character == '{' {
                        brace_depth += 1;
                    }
                    if next_character == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            break;
                        }
                    }
                    expression_inner_text.push(next_character);
                }

                // RECURSIVE SUB-PARSING
                let sub_source_code = format!(
                    "/// @/internal/interpolation.fer\n{}",
                    expression_inner_text
                );
                let sub_lexer = Lexer::new(&sub_source_code);
                let mut sub_parser = Parser::new(sub_lexer);
                let sub_module = sub_parser.parse_module();

                if let Some(Statement::Expression(parsed_expression)) =
                    sub_module.body.into_iter().next()
                {
                    string_parts.push(parsed_expression);
                }
            } else {
                current_plain_text.push(character);
            }
        }

        // Push final remaining text
        if !current_plain_text.is_empty() {
            string_parts.push(Expression::Literal(Literal::String(current_plain_text)));
        }

        Expression::InterpolatedString(string_parts)
    }

    /// Handles indentation and backslash line continuation
    fn preprocess_multiline_string(&mut self, raw_content: String) -> String {
        // Step 1: Backslash Continuation
        let mut step1_result = String::new();
        let mut characters = raw_content.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\\' {
                if let Some('\n') | Some('\r') = characters.peek() {
                    if characters.peek() == Some(&'\r') {
                        characters.next();
                    }
                    characters.next(); // Consume newline
                    while characters.peek() == Some(&' ') {
                        characters.next();
                    } // Consume leading spaces
                    continue;
                }
            }
            step1_result.push(character);
        }

        // Step 2: Indentation Stripping
        let lines: Vec<&str> = step1_result.split('\n').collect();
        if lines.len() <= 1 {
            return step1_result;
        }

        let last_line = lines.last().unwrap_or(&"");
        let base_indentation_count = last_line.chars().take_while(|c| *c == ' ').count();

        let mut processed_lines = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if index == 0 && line.trim().is_empty() {
                continue;
            }
            if index == lines.len() - 1
                && line.len() <= base_indentation_count
                && line.trim().is_empty()
            {
                continue;
            }

            let stripped_line = if line.len() >= base_indentation_count
                && line.starts_with(&" ".repeat(base_indentation_count))
            {
                &line[base_indentation_count..]
            } else {
                line.trim_start()
            };
            processed_lines.push(stripped_line);
        }

        processed_lines.join("\n")
    }

    fn parse_match_expression(&mut self, target: Expression) -> Expression {
        self.expect(Token::LBrace);
        let mut arms = vec![];

        while self.current_token != Token::RBrace {
            arms.push(self.parse_match_arm());
        }
        self.expect(Token::RBrace);

        Expression::Match {
            target: Box::new(target),
            arms,
        }
    }

    fn parse_match_arm(&mut self) -> MatchArm {
        let pattern = match &self.current_token {
            // Case: Comparison patterns like >= 90
            Token::Greater | Token::GreaterEq | Token::Less | Token::LessEq | Token::DoubleEq => {
                let op = self.current_token.clone();
                self.advance();
                let val = self.parse_literal();
                Pattern::Compare(op, val)
            }
            // Case: Default branch {}
            Token::LBrace => Pattern::Default,
            // Case: Direct value match 90
            _ => Pattern::Literal(self.parse_literal()),
        };

        // Parse the body block { ... }
        // If it's the default arm, we are already at the LBrace
        self.expect(Token::LBrace);
        let body = self.parse_expression();
        self.expect(Token::RBrace);

        MatchArm {
            pattern,
            body: Box::new(body),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    // Helper function to reduce boilerplate
    fn parse(source: &str) -> Module {
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        parser.parse_module()
    }

    #[test]
    // Test that the parser correctly extracts the mandatory path comment
    fn test_mandatory_header() {
        let source = "/// @/test.fer\nx = 1";
        let module = parse(source);
        assert_eq!(module.path, "@/test.fer");
    }

    #[test]
    // Test that the parser correctly handles imports with renaming
    /// /// @/test.fer
    /// { a b = alias } = @fer/std
    fn test_import_shorthand() {
        let source = "/// @/test.fer\n{ a b = alias } = @fer/std";
        let module = parse(source);
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.imports[0].source, "@fer/std");
        assert_eq!(module.imports[0].items[1].alias, Some("alias".to_string()));
    }

    #[test]
    // Test that the parser correctly handles array literals with multiple lines
    /// /// @/test.fer
    /// x = [
    ///   1
    ///   2
    ///   3
    /// ]
    fn test_array_multiline() {
        let source = r#"
/// @/test.fer
x = [
  1
  2
  3
]"#;
        let module = parse(source);
        // Check if body[0] is an array with 3 elements
        if let Statement::Declaration { value, .. } = &module.body[0] {
            if let Expression::Literal(Literal::Array(elms)) = value {
                assert_eq!(elms.len(), 3);
            }
        }
    }

    #[test]
    /// Test that the parser correctly handles function calls
    /// /// @/test.fer
    /// print(`hi`)
    fn test_function_call() {
        let source = "/// @/test.fer\nprint(`hi`)";
        let module = parse(source);
        assert!(matches!(
            module.body[0],
            Statement::Expression(Expression::Call { .. })
        ));
    }

    #[test]
    /// /// @/test.fer
    /// Status = enum { nice pass failed }
    fn test_enum_definition() {
        let source = "/// @/test.fer\nStatus = enum { nice pass failed }";
        let module = parse(source);

        if let Statement::TypeDefinition { name, kind } = &module.body[0] {
            assert_eq!(name, "Status");
            if let TypeKind::Enum(variants) = kind {
                assert_eq!(variants, &vec!["nice", "pass", "failed"]);
            } else {
                panic!("Expected Enum kind");
            }
        } else {
            panic!("Expected TypeDefinition statement");
        }
    }

    #[test]
    /// /// @/test.fer
    /// Candidate = struct {
    ///   id = i32
    ///   nickname = string
    /// }
    fn test_struct_definition() {
        let source = "/// @/test.fer\nCandidate = struct { id = i32 nickname = string }";
        let module = parse(source);

        if let Statement::TypeDefinition { name, kind } = &module.body[0] {
            assert_eq!(name, "Candidate");
            if let TypeKind::Struct(fields) = kind {
                assert_eq!(fields[0].name, "id");
                assert_eq!(fields[0].type_name, "i32");
            }
        }
    }

    #[test]
    fn test_match_expression() {
        let source = r#"
/// @/test.fer
result = score {
  >= 90 { `A` }
  { `F` }
}"#;
        let module = parse(source);

        if let Statement::Declaration { name, value, .. } = &module.body[0] {
            assert_eq!(name, "result");
            if let Expression::Match { arms, .. } = value {
                assert_eq!(arms.len(), 2);
                // Verify first arm: >= 90
                assert!(matches!(
                    arms[0].pattern,
                    Pattern::Compare(Token::GreaterEq, Literal::Int(90))
                ));
                // Verify second arm: Default
                assert!(matches!(arms[1].pattern, Pattern::Default));
            } else {
                panic!("Expected Match expression");
            }
        }
    }

    #[test]
    fn test_string_interpolation_ast() {
        // This test ensures { 1 + 1 } is parsed as a real recursive AST
        let source_code = r#"
/// @/test.fer
message = `Result: { 1 + 1 }`
"#;
        let module_ast = parse(source_code);

        if let Statement::Declaration { value, .. } = &module_ast.body[0] {
            if let Expression::InterpolatedString(string_parts) = value {
                // Should be: [Literal("Result: "), BinaryOp(1 + 1)]
                assert_eq!(string_parts.len(), 2);

                // Check the recursive part
                if let Expression::BinaryOp { op, .. } = &string_parts[1] {
                    assert_eq!(*op, Op::Add);
                } else {
                    panic!("The second part of interpolation should be a BinaryOp(+)!");
                }
            } else {
                panic!("Expected InterpolatedString expression, check your parser routing!");
            }
        }
    }

    #[test]
    fn test_multiline_indent_stripping() {
        let source = r#"
/// @/test.fer
multiple = `
  line1
  line2
  `
"#;
        let module = parse(source);
        if let Statement::Declaration { value, .. } = &module.body[0] {
            if let Expression::InterpolatedString(parts) = value {
                if let Expression::Literal(Literal::String(s)) = &parts[0] {
                    // It should be exactly "line1\nline2", no leading 2 spaces!
                    assert_eq!(s, "line1\nline2");
                }
            }
        }
    }
}
