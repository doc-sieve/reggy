use lalrpop_util::ParseError;
mod ast;
mod lexer;
mod transpile;

pub use ast::Ast;

lalrpop_util::lalrpop_mod!(pub grammar, "/parser/grammar.rs");

/// An error raised while parsing a `reggy` pattern
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    ParseError,
    DanglingEscape,
    UnnecessaryEscape
}

impl Error {
    fn from_lalrpop(err: ParseError<usize, lexer::Tok, Error>) -> Self {
        match err {
            ParseError::User { error } => error,
            _ => Error::ParseError,
        }
    }
}

impl Ast {
    /// Try to parse a string
    pub fn parse(code: impl AsRef<str>) -> Result<ast::Ast, Error> {
        let tokens = lexer::Lexer::new(&code).map(|tok| {
            if let Some(err) = tok.get_error() {
                Err(err)
            } else {
                let (start, end) = tok.bounds();
                Ok((start, tok.data, end))
            }
        });

        grammar::AstParser::new()
            .parse(tokens)
            .map_err(Error::from_lalrpop)
    }

    /// Return the maximum number of bytes this pattern can match
    pub fn max_bytes(&self) -> usize {
        match &self {
            Self::Char(c) => c.len_utf8(),
            Self::Digit => 1,
            Self::Space => 1,
            Self::CS(inner) => inner.max_bytes(),
            Self::Optional(inner) => inner.max_bytes(),
            Self::Or(inner) => inner.iter().map(|i| i.max_bytes()).max().unwrap_or(0),
            Self::Seq(inner) => inner.iter().map(|i| i.max_bytes()).sum(),
            Self::Quantifier(inner, _, max) => inner.max_bytes() * (*max as usize)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Ast,
        Ast::{Char, Digit, Optional, Or, Seq, CS, Quantifier},
    };

    #[test]
    fn parse_group() {
        assert_eq!(
            Ast::parse(r"fallac(y|ies)"),
            Ok(Seq(vec![
                Char('f'),
                Char('a'),
                Char('l'),
                Char('l'),
                Char('a'),
                Char('c'),
                Or(vec![Char('y'), Seq(vec![Char('i'), Char('e'), Char('s')])])
            ]))
        )
    }

    #[test]
    fn parse_basic_escape() {
        assert_eq!(
            Ast::parse(r"foo\??"),
            Ok(Seq(vec![
                Char('f'),
                Char('o'),
                Char('o'),
                Optional(Box::new(Char('?')))
            ]))
        )
    }

    #[test]
    fn case_sensitive() {
        assert_eq!(
            Ast::parse(r"foo(!b|AR)"),
            Ok(Seq(vec![
                Char('f'),
                Char('o'),
                Char('o'),
                CS(Box::new(Or(vec![
                    Char('b'),
                    Seq(vec![Char('A'), Char('R'),])
                ])))
            ]))
        )
    }

    #[test]
    fn parse_digits() {
        assert_eq!(
            Ast::parse(r"#?.##"),
            Ok(Seq(vec![
                Optional(Box::new(Digit)),
                Char('.'),
                Digit,
                Digit
            ]))
        )
    }

    #[test]
    fn parse_unicode() {
        assert_eq!(
            Ast::parse(r"Ⲁ(ⲗⲗ)?ⲫⲁ"),
            Ok(Seq(vec![
                Char('Ⲁ'),
                Optional(Box::new(Seq(vec![Char('ⲗ'), Char('ⲗ'),]))),
                Char('ⲫ'),
                Char('ⲁ'),
            ]))
        )
    }

    #[test]
    fn parse_quantifiers() {
        assert_eq!(
            Ast::parse(r"a{10}b{2,3}(cde){4}"),
            Ok(Seq(vec![
                Quantifier(Box::new(Char('a')), 10, 11),
                Quantifier(Box::new(Char('b')), 2, 3),
                Quantifier(Box::new(Seq(vec![Char('c'), Char('d'),Char('e')])), 4, 5),
            ]))
        )
    }
}
