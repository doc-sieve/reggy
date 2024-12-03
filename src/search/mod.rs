use regex_automata::Anchored;
use regex_automata::util::primitives::StateID;
use regex_automata::util::start::Config as StartConfig;
use regex_automata::dfa::{Automaton, dense};

use crate::Ast;

#[derive(Debug)]
pub struct Match {
    pub pos: (usize, usize),
    pub id: usize
}

#[derive(Debug)]
pub struct Search {
    pub dfa: dense::DFA<Vec<u32>>,
    state: StateID,
    pos: usize
}

impl Search {
    pub fn new(patterns: &[Ast]) -> Self {
        let transpiled_patterns = patterns.iter()
            .map(Ast::to_regex)
            .collect::<Vec<_>>();

        let build_cfg = dense::Config::new();
        
        let dfa = dense::Builder::new()
            .configure(build_cfg)
            .build_many(&transpiled_patterns).unwrap();

        let state = dfa.start_state(&StartConfig::new().anchored(Anchored::Yes)).unwrap();

        Self { dfa, state, pos: 0 }
    }

    pub fn step_word(&mut self, haystack: &str) -> Option<Match> {
        let mut last_match = None;

        for (i, &b) in haystack.as_bytes().iter().enumerate() {
            self.state = self.dfa.next_state(self.state, b);
            if self.dfa.is_special_state(self.state) {
                if self.dfa.is_match_state(self.state) {
                    last_match = Some(Match {
                        id: self.dfa.match_pattern(self.state, 0).as_usize(),
                        pos: (0, i),
                    });
                } else if self.dfa.is_dead_state(self.state) {
                    return last_match;
                }
            }
        }

        self.state = self.dfa.next_eoi_state(self.state);
        if self.dfa.is_match_state(self.state) {
            last_match = Some(Match {
                id: self.dfa.match_pattern(self.state, 0).as_usize(),
                pos: (0, haystack.len()),
            });
        }
        
        last_match
    }
}

#[cfg(test)]
mod tests {
    use super::{Search, Ast};

    #[test]
    fn simple_search() {
        let p1 = Ast::parse("foo?").unwrap();
        let mut m = Search::new(&[p1]);
        println!("{:?}", m.step_word("fo"));
    }
}