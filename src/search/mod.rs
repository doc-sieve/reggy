pub use crate::parser::Ast;

pub struct Search {}

pub struct Match {
    pub position: (usize, usize),
    pub pattern: usize,
}

impl Search {
    pub fn new(_patterns: &[Ast]) -> Self {
        Self {}
    }

    pub fn next(_cont: &str) -> Vec<Match> {
        vec![]
    }
}
