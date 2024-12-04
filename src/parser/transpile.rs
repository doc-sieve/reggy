use regex_syntax::is_meta_character;

use regex_syntax::ast as re_ast;

use super::Ast;

fn dummy_span() -> re_ast::Span {
    re_ast::Span {
        start: re_ast::Position {
            offset: 0,
            line: 0,
            column: 0,
        },
        end: re_ast::Position {
            offset: 0,
            line: 0,
            column: 0,
        },
    }
}

fn perl_class(kind: re_ast::ClassPerlKind) -> re_ast::Ast {
    re_ast::Ast::class_perl(re_ast::ClassPerl {
        span: dummy_span(),
        kind,
        negated: false,
    })
}

fn repetition_op(kind: re_ast::RepetitionKind) -> re_ast::RepetitionOp {
    re_ast::RepetitionOp {
        span: dummy_span(),
        kind,
    }
}

fn repetition(inner: &Ast, kind: re_ast::RepetitionKind, cs: bool) -> re_ast::Ast {
    let inner = match inner {
        Ast::Digit | Ast::Char(_) | Ast::Space | Ast::CS(_) => inner.to_regex_ast_inner(cs, false),
        wrap => re_ast::Ast::group(re_ast::Group {
            span: dummy_span(),
            kind: re_ast::GroupKind::NonCapturing(group_flag_default()),
            ast: Box::new(wrap.to_regex_ast_inner(cs, true)),
        }),
    };

    re_ast::Ast::repetition(re_ast::Repetition {
        span: dummy_span(),
        op: repetition_op(kind),
        greedy: true,
        ast: Box::new(inner),
    })
}

fn group_flag_default() -> re_ast::Flags {
    let flag_items = vec![];

    re_ast::Flags {
        span: dummy_span(),
        items: flag_items,
    }
}

fn group_flag_cs() -> re_ast::Flags {
    let flag_items = vec![
        re_ast::FlagsItem {
            span: dummy_span(),
            kind: re_ast::FlagsItemKind::Negation,
        },
        re_ast::FlagsItem {
            span: dummy_span(),
            kind: re_ast::FlagsItemKind::Flag(re_ast::Flag::CaseInsensitive),
        },
    ];

    re_ast::Flags {
        span: dummy_span(),
        items: flag_items,
    }
}

fn group_flag_cim() -> re_ast::Flags {
    let flag_items = vec![
        re_ast::FlagsItem {
            span: dummy_span(),
            kind: re_ast::FlagsItemKind::Flag(re_ast::Flag::MultiLine),
        },
        re_ast::FlagsItem {
            span: dummy_span(),
            kind: re_ast::FlagsItemKind::Flag(re_ast::Flag::CaseInsensitive),
        },
    ];

    re_ast::Flags {
        span: dummy_span(),
        items: flag_items,
    }
}

impl Ast {
    fn to_regex_ast_inner(&self, cs: bool, already_grouped: bool) -> re_ast::Ast {
        match self {
            Self::Char(c) => {
                let kind = if is_meta_character(*c) {
                    re_ast::LiteralKind::Meta
                } else {
                    re_ast::LiteralKind::Verbatim
                };

                re_ast::Ast::literal(re_ast::Literal {
                    span: dummy_span(),
                    kind,
                    c: *c,
                })
            }
            Self::Seq(inner) => re_ast::Concat {
                span: dummy_span(),
                asts: inner
                    .iter()
                    .map(|i| Ast::to_regex_ast_inner(i, cs, false))
                    .collect(),
            }
            .into_ast(),

            Self::Digit => perl_class(re_ast::ClassPerlKind::Digit),

            // Self::Space => re_ast::Ast::repetition(re_ast::Repetition {
            //     span: dummy_span(),
            //     op: repetition_op(re_ast::RepetitionKind::OneOrMore),
            //     greedy: true,
            //     ast: Box::new(perl_class(re_ast::ClassPerlKind::Space)),
            // }),
            Self::Space => re_ast::Ast::literal(re_ast::Literal {
                span: dummy_span(),
                kind: re_ast::LiteralKind::Verbatim,
                c: ' ',
            }),

            Self::Optional(inner) => repetition(inner, re_ast::RepetitionKind::ZeroOrOne, cs),

            Self::Or(inner) => {
                let alternation = re_ast::Ast::alternation(re_ast::Alternation {
                    span: dummy_span(),
                    asts: inner
                        .iter()
                        .map(|i| Ast::to_regex_ast_inner(i, cs, false))
                        .collect(),
                });

                if already_grouped {
                    alternation
                } else {
                    re_ast::Ast::group(re_ast::Group {
                        span: dummy_span(),
                        kind: re_ast::GroupKind::NonCapturing(group_flag_default()),
                        ast: Box::new(alternation),
                    })
                }
            }

            Self::CS(inner) => {
                if cs {
                    re_ast::Ast::group(re_ast::Group {
                        span: dummy_span(),
                        kind: re_ast::GroupKind::NonCapturing(group_flag_default()),
                        ast: Box::new(inner.to_regex_ast_inner(true, true)),
                    })
                } else {
                    re_ast::Ast::group(re_ast::Group {
                        span: dummy_span(),
                        kind: re_ast::GroupKind::NonCapturing(group_flag_cs()),
                        ast: Box::new(inner.to_regex_ast_inner(true, true)),
                    })
                }
            }
        }
    }

    pub fn to_regex_ast(&self) -> re_ast::Ast {
        re_ast::Ast::group(re_ast::Group {
            span: dummy_span(),
            kind: re_ast::GroupKind::NonCapturing(group_flag_cim()),
            ast: Box::new(self.to_regex_ast_inner(false, true)),
        })
    }

    pub fn to_regex(&self) -> String {
        self.to_regex_ast().to_string()
    }
}
