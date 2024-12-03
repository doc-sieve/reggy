use std::str::Chars;

#[derive(Debug, Clone)]
pub enum Tok {
    Char { c: char, escaped: bool },
    Space,
    Digit,
    Or,
    QMark,
    LParen,
    RParen,
    Exclam,
    Star,
    Plus,
    Error(super::Error),
}

#[derive(Debug)]
pub struct Token {
    start: usize,
    pub data: Tok,
}

impl Token {
    fn char(start: usize, c: char, escaped: bool) -> Self {
        Self {
            start,
            data: Tok::Char { c, escaped },
        }
    }

    fn space(start: usize) -> Self {
        Self {
            start,
            data: Tok::Space,
        }
    }

    fn digit(start: usize) -> Self {
        Self {
            start,
            data: Tok::Digit,
        }
    }

    fn error(start: usize, error: super::Error) -> Self {
        Self {
            start,
            data: Tok::Error(error),
        }
    }

    pub fn get_error(&self) -> Option<super::Error> {
        if let Tok::Error(err) = &self.data {
            Some(err.clone())
        } else {
            None
        }
    }

    fn reserved(start: usize, c: char) -> Option<Self> {
        Some(Self {
            start,
            data: match c {
                '|' => Tok::Or,
                '?' => Tok::QMark,
                '(' => Tok::LParen,
                ')' => Tok::RParen,
                '!' => Tok::Exclam,
                '+' => Tok::Plus,
                '*' => Tok::Star,
                _ => return None,
            },
        })
    }

    pub fn bounds(&self) -> (usize, usize) {
        match &self.data {
            Tok::Char { escaped: true, .. } => (self.start, self.start + 2),
            Tok::Digit => (self.start, self.start + 2),
            _ => (self.start, self.start + 1),
        }
    }
}

pub struct Lexer<'a> {
    code: Chars<'a>,
    i: usize,
    escape: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            code: code.chars(),
            i: 0,
            escape: false,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        for c in self.code.by_ref() {
            match Token::reserved(self.i, c) {
                Some(tok) => {
                    if self.escape {
                        self.escape = false;
                        self.i += 1;
                        return Some(Token::char(self.i - 2, c, true));
                    } else {
                        self.i += 1;
                        return Some(tok);
                    }
                }
                None => {
                    if c == '\\' {
                        if self.escape {
                            self.escape = false;
                            self.i += 1;
                            return Some(Token::char(self.i - 2, c, true));
                        } else {
                            self.escape = true;
                            self.i += 1;
                            continue;
                        }
                    } else if self.escape {
                        self.escape = false;
                        self.i += 1;

                        return match c {
                            'd' => Some(Token::digit(self.i - 2)),
                            _ => Some(Token::error(self.i - 2, super::Error::UnnecessaryEscape)),
                        };
                    } else {
                        self.i += 1;
                        return match c {
                            ' ' => Some(Token::space(self.i)),
                            _ => Some(Token::char(self.i, c, false)),
                        };
                    }
                }
            }
        }

        None
    }
}
