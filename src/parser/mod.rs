mod ast;
mod lexer;
use lalrpop_util::ParseError;

pub use ast::Ast;

lalrpop_util::lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    #[rustfmt::skip]
    pub grammar,
    "/parser/grammar.rs"
);

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

#[cfg(test)]
mod tests {
    use super::{
        parse,
        Ast::{Byte, CaseSensitive, Digit, Or, Seq, ZeroOrOne},
    };

    #[test]
    fn parse_group() {
        assert_eq!(
            parse(r"fallac(y|ies)"),
            Ok(Seq(vec![
                Byte(b'f'),
                Byte(b'a'),
                Byte(b'l'),
                Byte(b'l'),
                Byte(b'a'),
                Byte(b'c'),
                Or(
                    Box::new(Byte(b'y')),
                    Box::new(Seq(vec![Byte(b'i'), Byte(b'e'), Byte(b's')]))
                )
            ]))
        )
    }

    #[test]
    fn parse_basic_escape() {
        assert_eq!(
            parse(r"foo\??"),
            Ok(Seq(vec![
                Byte(b'f'),
                Byte(b'o'),
                Byte(b'o'),
                ZeroOrOne(Box::new(Byte(b'?')))
            ]))
        )
    }

    #[test]
    fn case_sensitive() {
        assert_eq!(
            parse(r"foo(!b|AR)"),
            Ok(Seq(vec![
                Byte(b'f'),
                Byte(b'o'),
                Byte(b'o'),
                CaseSensitive(Box::new(Or(
                    Box::new(Byte(b'b')),
                    Box::new(Seq(vec![Byte(b'A'), Byte(b'R'),]))
                )))
            ]))
        )
    }

    #[test]
    fn parse_digits() {
        assert_eq!(
            parse(r"\d?.\d\d"),
            Ok(Seq(vec![
                ZeroOrOne(Box::new(Digit)),
                Byte(b'.'),
                Digit,
                Digit
            ]))
        )
    }

    #[test]
    fn parse_unicode() {
        assert_eq!(
            parse(r"Ⲁ(ⲗⲗ)?ⲫⲁ"),
            Ok(Seq(vec![
                Byte(226),
                Byte(178),
                Byte(128),
                ZeroOrOne(Box::new(Seq(vec![
                    Byte(226),
                    Byte(178),
                    Byte(151),
                    Byte(226),
                    Byte(178),
                    Byte(151),
                ]))),
                Byte(226),
                Byte(178),
                Byte(171),
                Byte(226),
                Byte(178),
                Byte(129)
            ]))
        )
    }
}
