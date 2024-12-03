mod ast;
mod lexer;
use lalrpop_util::ParseError;

pub use ast::Ast;

lalrpop_util::lalrpop_mod!(pub grammar, "/parser/grammar.rs");

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    ParseError,
    DanglingEscape,
    UnnecessaryEscape,
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
    pub fn parse(code: &str) -> Result<ast::Ast, Error> {
        let tokens = lexer::Lexer::new(code).map(|tok| {
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
}

#[cfg(test)]
mod tests {
    use super::{
        Ast,
        Ast::{Char, Digit, Optional, Or, Seq, CS},
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
            Ast::parse(r"\d?.\d\d"),
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
}
