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

    fn parse_statement(&mut self) -> Statement {
        if let Token::Identifier(name) = self.current_token.clone() {
            self.advance(); // consume identifier

            match self.current_token {
                // If it's followed by an =
                // then it's a variable definition or a type definition
                Token::Equals => {
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
                }
                // If it's followed by (), then it's a function call
                Token::LParen => {
                    self.advance(); // consume '('
                    let arg = self.parse_expression();
                    self.expect(Token::RParen);

                    Statement::Expression(Expression::Call {
                        callee: Box::new(Expression::Identifier(name)),
                        args: vec![arg],
                    })
                }
                _ => panic!(
                    "Expected '=' or '(' after identifier, but found {:?}",
                    self.current_token
                ),
            }
        } else {
            panic!("Expected statement, found {:?}", self.current_token);
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

        // If a primary expression is followed by '{', it's a Match Block
        if self.current_token == Token::LBrace {
            left = self.parse_match_expression(left);
        }
        left
    }

    fn parse_primary_expression(&mut self) -> Expression {
        match &self.current_token {
            Token::StringLit(_) | Token::Number(_) | Token::LBracket => {
                Expression::Literal(self.parse_literal())
            }
            Token::Identifier(id) => {
                let name = id.clone();
                self.advance();
                // Check if it's a function call: name(...)
                if self.current_token == Token::LParen {
                    self.advance();
                    let mut args = vec![];
                    if self.current_token != Token::RParen {
                        args.push(self.parse_expression());
                        while self.current_token == Token::Comma {
                            self.advance();
                            args.push(self.parse_expression());
                        }
                    }
                    self.expect(Token::RParen);
                    Expression::Call {
                        callee: Box::new(Expression::Identifier(name)),
                        args,
                    }
                } else {
                    Expression::Identifier(name)
                }
            }
            _ => panic!("Expected expression, found {:?}", self.current_token),
        }
    }

    fn parse_literal(&mut self) -> Literal {
        match self.current_token.clone() {
            Token::Number(n) => {
                self.advance();
                Literal::Int(n)
            }
            Token::StringLit(s) => {
                self.advance();
                Literal::String(s)
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
}
