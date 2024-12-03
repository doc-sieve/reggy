use regex_automata::dfa::{dense, Automaton};
use regex_automata::util::primitives::StateID;
use regex_automata::util::start::Config as StartConfig;
use regex_automata::{Anchored, MatchKind};

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

        let build_cfg = dense::Config::new()
            .match_kind(MatchKind::All);

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
        let new_state = self.dfa
            .start_state(&StartConfig::new().anchored(Anchored::Yes))
            .unwrap();

        self.state.push((self.pos, new_state));

        for &b in haystack.as_bytes() {
            for state_i in 0..self.state.len() {
                self.state[state_i].1 = self.dfa.next_state(self.state[state_i].1, b);
            }
        }

        self.pos += haystack.len();

        let mut matches = vec![];

        for state_i in 0..self.state.len() {
            let try_finish_state = self.dfa.next_eoi_state(self.state[state_i].1);
            if self.dfa.is_match_state(try_finish_state) {
                for pattern_i in 0..self.dfa.match_len(try_finish_state) {
                    matches.push(Match {
                        id: self.dfa.match_pattern(try_finish_state, pattern_i).as_usize(),
                        pos: (self.state[state_i].0, self.pos)
                    });
                }
            }
        }

        self.state.retain(|s| !self.dfa.is_dead_state(s.1));

        matches
    }

    pub fn step(&mut self, haystack: &str) -> Vec<Match> {
        haystack.split_word_bounds().flat_map(|w| self.step_word(w)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Ast, Search};

    #[test]
    fn simple_search() {
        let pattern_strs = [
            "woz?",
            "foo b(!aR)",
            "foo bar"
        ];
        
        let patterns: Vec<_> = pattern_strs.iter().map(|p| Ast::parse(p).unwrap()).collect();

        let mut s = Search::new(&patterns);


        let mut haystack = "Foo bar wo foo";
        
        println!("---- Matching step \"{haystack}\"");
        for m in s.step(haystack) {
            println!(
                "Match( pos: {:?}, pattern: \"{}\" )",
                m.pos,
                pattern_strs[m.id]
            )
        }

        haystack = " baR woz";
        
        println!("---- Matching step \"{haystack}\"");
        for m in s.step(haystack) {
            println!(
                "Match( pos: {:?}, pattern: \"{}\" )",
                m.pos,
                pattern_strs[m.id]
            )
        }
    }
}
