use regex_automata::dfa::{dense, Automaton};
use regex_automata::util::primitives::StateID;
use regex_automata::util::start::Config as StartConfig;
use regex_automata::{Anchored, MatchKind};

use unicode_segmentation::UnicodeSegmentation;

use crate::Ast;

type DFA = dense::DFA<Vec<u32>>;

#[derive(Debug, Clone, Copy)]
pub struct Match {
    pub pos: (usize, usize),
    pub id: usize,
}

#[derive(Debug, Clone)]
struct HalfMatch {
    start: usize,
    candidate: Option<Vec<Match>>,
    id: StateID
}

impl HalfMatch {
    fn new(start: usize, dfa: &DFA) -> Self {
        let start_cfg = StartConfig::new().anchored(Anchored::Yes);
        let new_state = dfa.start_state(&start_cfg).unwrap();

        Self {
            start,
            candidate: None,
            id: new_state
        }
    }
}

#[derive(Debug, Clone)]
pub struct Search {
    pub dfa: DFA,
    state: Vec<HalfMatch>,
    pos: usize,
}

impl Search {
    pub fn new(patterns: &[Ast]) -> Self {
        let transpiled_patterns = patterns.iter().map(Ast::to_regex).collect::<Vec<_>>();

        let build_cfg = dense::Config::new().match_kind(MatchKind::All);

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
        self.state.push(HalfMatch::new(self.pos, &self.dfa));

        for &b in haystack.as_bytes() {
            for state_i in 0..self.state.len() {
                self.state[state_i].id = self.dfa.next_state(self.state[state_i].id, b);
            }
        }

        self.pos += haystack.len();

        let mut matches: Vec<Match> = vec![];

        for state_i in 0..self.state.len() {
            let try_finish_state = self.dfa.next_eoi_state(self.state[state_i].id);
            
            if self.dfa.is_match_state(try_finish_state) {
                let mut candidate = self.state[state_i]
                    .candidate
                    .take()
                    .unwrap_or(vec![]);

                for pattern_i in 0..self.dfa.match_len(try_finish_state) {
                    let pattern_id = self
                        .dfa
                        .match_pattern(try_finish_state, pattern_i)
                        .as_usize();

                    let mut found = false;
                    for c in &mut candidate {
                        if c.id == pattern_id {
                            found = true;
                            c.pos.1 = self.pos;
                            break;
                        }
                    }

                    if !found {
                        candidate.push(Match {
                            pos: (self.state[state_i].start, self.pos),
                            id: pattern_id
                        })
                    }
                }

                self.state[state_i].candidate = Some(candidate);
            }
        }

        self.state.retain(|state| {
            if self.dfa.is_dead_state(state.id) {
                if let Some(candidate) = &state.candidate {
                    matches.extend(candidate);
                }
                false
            } else {
                true
            }
        });

        matches
    }

    pub fn step(&mut self, haystack: &str) -> Vec<Match> {
        haystack
            .split_word_bounds()
            .flat_map(|w| self.step_word(w))
            .collect()
    }

    pub fn finish(&mut self) -> Vec<Match> {
        std::mem::replace(&mut self.state, vec![])
            .iter()
            .flat_map(|s| s.candidate.clone())
            .flatten()
            .collect()
    }

    pub fn reset(&mut self) {
        self.state = vec![];
        self.pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{Ast, Search};

    #[test]
    fn simple_search() {
        let pattern_strs = [
            "woz?",
            "foo( bar)?",
            "foo b(!aR)"
        ];

        let haystacks = [
            "Foo bar wo foo",
            " baR woz"
        ];

        let patterns: Vec<_> = pattern_strs
            .iter()
            .map(|p| Ast::parse(p).unwrap())
            .collect();

        let mut s = Search::new(&patterns);

        for haystack in haystacks {
            println!("Matching step \"{haystack}\"");
            for m in s.step(haystack) {
                println!(
                    "\tMatch( pos: {:?}, pattern: \"{}\" )",
                    m.pos, pattern_strs[m.id]
                )
            }    
        }

        println!("Finalizing");
        for m in s.finish() {
            println!(
                "\tMatch( pos: {:?}, pattern: \"{}\" )",
                m.pos, pattern_strs[m.id]
            )
        }
    }
}
