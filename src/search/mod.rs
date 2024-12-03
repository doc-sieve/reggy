use regex_automata::dfa::{dense, Automaton};
use regex_automata::util::primitives::StateID;
use regex_automata::util::start::Config as StartConfig;
use regex_automata::Anchored;

use unicode_segmentation::UnicodeSegmentation;

use crate::Ast;

#[derive(Debug)]
pub struct Match {
    pub pos: (usize, usize),
    pub id: usize,
}

#[derive(Debug)]
pub struct Search {
    pub dfa: dense::DFA<Vec<u32>>,
    state: Vec<(usize, StateID)>,
    pos: usize,
}

impl Search {
    pub fn new(patterns: &[Ast]) -> Self {
        let transpiled_patterns = patterns.iter().map(Ast::to_regex).collect::<Vec<_>>();

        let build_cfg = dense::Config::new();

        let dfa = dense::Builder::new()
            .configure(build_cfg)
            .build_many(&transpiled_patterns)
            .unwrap();

        Self {
            dfa,
            state: vec![],
            pos: 0,
        }
    }

    fn step_word(&mut self, haystack: &str) -> Vec<Match> {
        let new_state = self
            .dfa
            .start_state(&StartConfig::new().anchored(Anchored::Yes))
            .unwrap();

        self.state.push((self.pos, new_state));

        for &b in haystack.as_bytes() {
            for state_i in 0..self.state.len() {
                self.state[state_i].1 = self.dfa.next_state(self.state[state_i].1, b);
            }
        }

        self.pos += haystack.len();

        println!("{}", haystack);

        vec![]
    }

    pub fn step(&mut self, haystack: &str) -> Vec<Match> {
        haystack.unicode_words().flat_map(|w| self.step_word(w)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Ast, Search};

    #[test]
    fn simple_search() {
        let p1 = Ast::parse("b").unwrap();
        let mut m = Search::new(&[p1]);
        println!("{:?}", m.step("foo; bar"));
    }
}
