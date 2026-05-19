// syntax/src/grammar.rs

/// Every terminal symbol the Fer lexer can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // ---- Literals ----
    IntLiteral,    // 42
    FloatLiteral,  // 3.14
    StringLiteral, // `hello` (non-interpolated)
    TrueKw,        // true
    FalseKw,       // false

    // ---- String interpolation (reserved for later) ----
    StringStart, // opening ` (followed by text or expr)
    StringPart,  // plain text inside a string
    StringEnd,   // closing `

    ExprStart, // { inside a string
    ExprEnd,   // } inside a string

    RegexLiteral, // /pattern/flags

    // ---- Identifier ----
    Identifier, // user-defined name

    // ---- Keywords ----
    Struct,   // struct
    Enum,     // enum
    Exports,  // exports
    And,      // and
    Or,       // or
    Not,      // not
    Contains, // contains
    Less,     // less
    More,     // more
    Least,    // least
    Most,     // most
    Equals,   // equals
    In,       // in
    Matches,  // matches
    Starts,   // starts
    Ends,     // ends

    // ---- Delimiters ----
    LBrace,   // {
    RBrace,   // }
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    Comma,    // ,
    Dot,      // .
    Colon,    // :
    Arrow,    // ->
    At,       // @ (for import paths)

    // ---- Operators ----
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    Eq,      // =
    Lt,      // <
    Gt,      // >
    LtEq,    // <=
    GtEq,    // >=

    // ---- Special ----
    Eof,
    Error,
}

/// Operator precedence (higher = binds tighter) and associativity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
}

/// A row in the precedence table.
#[derive(Debug, Clone, Copy)]
pub struct OpInfo {
    pub kind: TokenKind,
    pub prec: u8,
    pub assoc: Assoc,
}

/// Static precedence table for binary operators.
pub const BINARY_OPS: &[OpInfo] = &[
    // Logical or
    OpInfo {
        kind: TokenKind::Or,
        prec: 1,
        assoc: Assoc::Left,
    },
    // Logical and
    OpInfo {
        kind: TokenKind::And,
        prec: 2,
        assoc: Assoc::Left,
    },
    // Comparisons (all same precedence, non-associative in theory, but we'll treat as left)
    // <
    OpInfo {
        kind: TokenKind::Lt,
        prec: 3,
        assoc: Assoc::Left,
    },
    // less
    OpInfo {
        kind: TokenKind::Less,
        prec: 3,
        assoc: Assoc::Left,
    },
    // >
    OpInfo {
        kind: TokenKind::Gt,
        prec: 3,
        assoc: Assoc::Left,
    },
    // more
    OpInfo {
        kind: TokenKind::More,
        prec: 3,
        assoc: Assoc::Left,
    },
    // <=
    OpInfo {
        kind: TokenKind::LtEq,
        prec: 3,
        assoc: Assoc::Left,
    },
    // least
    OpInfo {
        kind: TokenKind::Least,
        prec: 3,
        assoc: Assoc::Left,
    },
    // >=
    OpInfo {
        kind: TokenKind::GtEq,
        prec: 3,
        assoc: Assoc::Left,
    },
    // most
    OpInfo {
        kind: TokenKind::Most,
        prec: 3,
        assoc: Assoc::Left,
    },
    // equals
    OpInfo {
        kind: TokenKind::Equals,
        prec: 3,
        assoc: Assoc::Left,
    },
    // contains
    OpInfo {
        kind: TokenKind::Contains,
        prec: 3,
        assoc: Assoc::Left,
    },
    // matches
    OpInfo {
        kind: TokenKind::Matches,
        prec: 3,
        assoc: Assoc::Left,
    },
    // starts
    OpInfo {
        kind: TokenKind::Starts,
        prec: 3,
        assoc: Assoc::Left,
    },
    // ends
    OpInfo {
        kind: TokenKind::Ends,
        prec: 3,
        assoc: Assoc::Left,
    },
    // in
    OpInfo {
        kind: TokenKind::In,
        prec: 3,
        assoc: Assoc::Left,
    },
    // Arithmetic
    // +
    OpInfo {
        kind: TokenKind::Plus,
        prec: 5,
        assoc: Assoc::Left,
    },
    // -
    OpInfo {
        kind: TokenKind::Minus,
        prec: 5,
        assoc: Assoc::Left,
    },
    // *
    OpInfo {
        kind: TokenKind::Star,
        prec: 6,
        assoc: Assoc::Left,
    },
    // /
    OpInfo {
        kind: TokenKind::Slash,
        prec: 6,
        assoc: Assoc::Left,
    },
    // %
    OpInfo {
        kind: TokenKind::Percent,
        prec: 6,
        assoc: Assoc::Left,
    },
];

/// Look up precedence and associativity for a binary operator.
pub fn prec_of(kind: TokenKind) -> Option<(u8, Assoc)> {
    for info in BINARY_OPS {
        if info.kind == kind {
            return Some((info.prec, info.assoc));
        }
    }
    None
}

/// All keywords as string slices, for documentation and testing.
pub const KEYWORDS: &[&str] = &[
    "struct", "enum", "exports", "and", "or", "not", "contains", "less", "more", "least", "most",
    "equals", "in", "matches", "starts", "ends", "true", "false",
];

/// Map a keyword string to its TokenKind, if it is one.
pub fn keyword_token(word: &str) -> Option<TokenKind> {
    match word {
        "struct" => Some(TokenKind::Struct),
        "enum" => Some(TokenKind::Enum),
        "exports" => Some(TokenKind::Exports),
        "and" => Some(TokenKind::And),
        "or" => Some(TokenKind::Or),
        "not" => Some(TokenKind::Not),
        "contains" => Some(TokenKind::Contains),
        "less" => Some(TokenKind::Less),
        "more" => Some(TokenKind::More),
        "least" => Some(TokenKind::Least),
        "most" => Some(TokenKind::Most),
        "equals" => Some(TokenKind::Equals),
        "in" => Some(TokenKind::In),
        "matches" => Some(TokenKind::Matches),
        "starts" => Some(TokenKind::Starts),
        "ends" => Some(TokenKind::Ends),
        "true" => Some(TokenKind::TrueKw),
        "false" => Some(TokenKind::FalseKw),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_token_recognizes_all() {
        for kw in KEYWORDS {
            let tok = keyword_token(kw).expect("keyword must be recognized");
            // basic sanity: it's not an identifier literal
            assert!(matches!(tok, TokenKind::Identifier) == false);
        }
    }

    #[test]
    fn keyword_token_rejects_non_keywords() {
        assert_eq!(keyword_token("foo"), None);
        assert_eq!(keyword_token("bar"), None);
        assert_eq!(keyword_token(""), None);
    }

    #[test]
    fn prec_of_unknown_ops_returns_none() {
        assert_eq!(prec_of(TokenKind::Identifier), None);
        assert_eq!(prec_of(TokenKind::Eof), None);
    }

    #[test]
    fn precedence_order_is_respected() {
        let plus = prec_of(TokenKind::Plus).unwrap().0;
        let star = prec_of(TokenKind::Star).unwrap().0;
        assert!(star > plus, "Star should bind tighter than plus");
    }
}
