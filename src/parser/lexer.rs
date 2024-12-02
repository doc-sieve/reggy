#[derive(Debug, Clone)]
pub enum Tok {
    Byte { b: u8, escaped: bool },
    Digit,
    Or,
    QMark,
    LParen,
    RParen,
    Exclam,
    Error(super::Error),
}

#[derive(Debug)]
pub struct Token {
    start: usize,
    pub data: Tok,
}

impl Token {
    fn byte(start: usize, b: u8, escaped: bool) -> Self {
        Self {
            start,
            data: Tok::Byte { b, escaped },
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

    fn reserved(start: usize, b: u8) -> Option<Self> {
        Some(Self {
            start,
            data: match b {
                b'|' => Tok::Or,
                b'?' => Tok::QMark,
                b'(' => Tok::LParen,
                b')' => Tok::RParen,
                b'!' => Tok::Exclam,
                _ => return None,
            },
        })
    }

    pub fn bounds(&self) -> (usize, usize) {
        match &self.data {
            Tok::Byte { escaped: true, .. } => (self.start, self.start + 2),
            Tok::Digit => (self.start, self.start + 2),
            _ => (self.start, self.start + 1),
        }
    }
}

pub struct Lexer<'a> {
    code: &'a str,
    i: usize,
    escape: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(code: &'a str) -> Self {
        Self {
            code,
            i: 0,
            escape: false,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        while self.i < self.code.len() {
            let byte = self.code.as_bytes()[self.i];
            if !self.code.is_char_boundary(self.i) {
                if self.escape {
                    self.escape = false;
                    self.i += 1;
                    return Some(Token::error(self.i - 1, super::Error::UnnecessaryEscape));
                } else {
                    self.i += 1;
                    return Some(Token::byte(self.i - 1, byte, false));
                }
            }

            match Token::reserved(self.i, byte) {
                Some(tok) => {
                    if self.escape {
                        self.escape = false;
                        self.i += 1;
                        return Some(Token::byte(self.i - 1, byte, true));
                    } else {
                        self.i += 1;
                        return Some(tok);
                    }
                }
                None => {
                    if byte == b'\\' {
                        if self.escape {
                            self.escape = false;
                            self.i += 1;
                            return Some(Token::byte(self.i - 1, byte, true));
                        } else {
                            self.escape = true;
                            self.i += 1;
                            continue;
                        }
                    } else if self.escape {
                        self.escape = false;
                        self.i += 1;

                        return match byte {
                            b'd' => Some(Token::digit(self.i - 1)),
                            _ => Some(Token::error(self.i - 2, super::Error::UnnecessaryEscape)),
                        };
                    } else {
                        self.i += 1;
                        return Some(Token::byte(self.i, byte, false));
                    }
                }
            }
        }

        None
    }
}
