use regex_syntax::is_meta_character;

use super::Ast;

impl Ast {
    fn to_regex_inner(&self, cs: bool) -> String {
        match self {
            Self::Char(c) => {
                if is_meta_character(*c) {
                    format!("\\{c}")
                } else {
                    c.to_string()
                }
            }
            Self::Or(inner) => inner
                .iter()
                .map(|i| Ast::to_regex_inner(i, cs))
                .collect::<Vec<_>>()
                .join("|"),
            Self::Seq(inner) => {
                let mut acc = String::new();
                for i in inner {
                    if matches!(i, Self::Or(_)) {
                        acc.push_str(&format!("(?:{})", i.to_regex_inner(cs)));
                    } else {
                        acc.push_str(&i.to_regex_inner(cs));
                    }
                }
                acc
            }
            Self::Digit => r"\d".into(),
            Self::Space => r"\s+".into(),
            Self::Optional(inner) => match inner.as_ref() {
                Self::Char(c) => {
                    if is_meta_character(*c) {
                        format!("\\{c}?")
                    } else {
                        format!("{c}?")
                    }
                }
                Self::Digit => r"\d?".into(),
                Self::Space => r"\s*".into(),
                i => format!("(?:{})?", i.to_regex_inner(cs)),
            },
            Self::ZeroOrMore(inner) => match inner.as_ref() {
                Self::Char(c) => {
                    if is_meta_character(*c) {
                        format!("\\{c}*")
                    } else {
                        format!("{c}*")
                    }
                }
                Self::Digit => r"\d*".into(),
                Self::Space => r"\s*".into(),
                i => format!("(?:{})*", i.to_regex_inner(cs)),
            },
            Self::OneOrMore(inner) => match inner.as_ref() {
                Self::Char(c) => {
                    if is_meta_character(*c) {
                        format!("\\{c}+")
                    } else {
                        format!("{c}+")
                    }
                }
                Self::Digit => r"\d+".into(),
                Self::Space => r"\s+".into(),
                i => format!("(?:{})+", i.to_regex_inner(cs)),
            },
            Self::CS(inner) => {
                if cs {
                    format!("(?:{})", inner.to_regex_inner(true))
                } else {
                    format!("(?-i:{})", inner.to_regex_inner(true))
                }
            }
        }
    }

    pub fn to_regex(&self) -> String {
        format!("(?i:{})", self.to_regex_inner(false))
    }
}
