#[derive(Debug, PartialEq)]
pub enum Ast {
    Char(char),
    Digit,
    Space,
    Seq(Vec<Ast>),
    Or(Vec<Ast>),
    Optional(Box<Ast>),
    ZeroOrMore(Box<Ast>),
    OneOrMore(Box<Ast>),
    CS(Box<Ast>),
}

impl Ast {
    pub(super) fn char(c: char) -> Self {
        Self::Char(c)
    }

    pub(super) fn digit() -> Self {
        Self::Digit
    }

    pub(super) fn space() -> Self {
        Self::Space
    }

    pub(super) fn is_cs(&self) -> bool {
        match &self {
            Self::Char(_) => false,
            Self::Digit => true,
            Self::Space => true,
            Self::CS(_) => true,
            Self::Optional(inner) => inner.is_cs(),
            Self::ZeroOrMore(inner) => inner.is_cs(),
            Self::OneOrMore(inner) => inner.is_cs(),
            Self::Or(inner) => inner.iter().all(|i| i.is_cs()),
            Self::Seq(inner) => inner.iter().all(|i| i.is_cs()),
        }
    }

    pub(super) fn case_sensitive(inner: Ast) -> Self {
        if inner.is_cs() {
            inner
        } else {
            Self::CS(Box::new(inner))
        }
    }

    pub(super) fn then(lhs: Ast, rhs: Ast) -> Self {
        if let Self::Seq(mut inner) = lhs {
            if matches!(rhs, Ast::Space) && matches!(inner.last(), Some(Ast::Space)) {
            } else {
                inner.push(rhs);
            }

            Self::Seq(inner)
        } else {
            Self::Seq(vec![lhs, rhs])
        }
    }

    pub(super) fn or(lhs: Ast, rhs: Ast) -> Self {
        if let Self::Or(mut inner) = lhs {
            inner.push(rhs);
            Self::Or(inner)
        } else {
            Self::Or(vec![lhs, rhs])
        }
    }

    pub(super) fn optional(inner: Ast) -> Self {
        Self::Optional(Box::new(inner))
    }

    pub(super) fn zero_or_more(inner: Ast) -> Self {
        Self::ZeroOrMore(Box::new(inner))
    }

    pub(super) fn one_or_more(inner: Ast) -> Self {
        Self::OneOrMore(Box::new(inner))
    }
}
