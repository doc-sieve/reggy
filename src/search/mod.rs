use regex_automata::dfa::dense;

use crate::Ast;

pub struct Match {
    pub pos: (usize, usize),
    pub id: usize
}

#[derive(Debug)]
pub struct Search {
    pub machine: dense::DFA<Vec<u32>>
}

impl Search {
    pub fn new(patterns: &[Ast]) -> Self {
        let transpiled_patterns = patterns.iter()
            .map(Ast::to_regex)
            .collect::<Vec<_>>();

        Self {
            machine: dense::DFA::new_many(&transpiled_patterns).unwrap()
        }
    }

    pub fn step(&mut self) -> Vec<Match> {

        vec![]
    }
}

mod tests {
    use super::{Search, Ast};

    #[test]
    fn simple_search() {
        let p1 = Ast::parse("foo|bar").unwrap();
        let p2 = Ast::parse("baz|bim").unwrap();
        let m = Search::new(&[p1, p2]);
        println!("{:?}", m);
    }
}