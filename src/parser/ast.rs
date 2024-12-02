#[derive(Debug, PartialEq)]
pub enum Ast {
    Byte(u8),
    Digit,
    Seq(Vec<Ast>),
    Or(Box<Ast>, Box<Ast>),
    ZeroOrOne(Box<Ast>),
    CaseSensitive(Box<Ast>),
}

impl Ast {
    pub fn byte(b: u8) -> Self {
        Self::Byte(b)
    }

    pub fn digit() -> Self {
        Self::Digit
    }

    pub fn case_sensitive(inner: Ast) -> Self {
        Self::CaseSensitive(Box::new(inner))
    }

    pub fn then(lhs: Ast, rhs: Ast) -> Self {
        if let Self::Seq(mut inner) = lhs {
            inner.push(rhs);
            Self::Seq(inner)
        } else {
            Self::Seq(vec![lhs, rhs])
        }
    }

    pub fn or(lhs: Ast, rhs: Ast) -> Self {
        Self::Or(Box::new(lhs), Box::new(rhs))
    }

    pub fn zero_or_one(inner: Ast) -> Self {
        Self::ZeroOrOne(Box::new(inner))
    }
}
