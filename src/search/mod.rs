use std::collections::HashMap;

use regex_automata::util::{
    start::Config as StartConfig,
    primitives::{StateID, PatternID}
};
use regex_automata::dfa::Automaton;
use regex_automata::{dfa, MatchKind, Anchored};

use unicode_segmentation::UnicodeSegmentation;

use crate::{Ast, Error};

type Dfa = dfa::dense::DFA<Vec<u32>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub id: PatternID,
    pub span: (usize, usize),
}

impl Match {
    pub fn new(id: impl Into<PatternID>, span: (usize, usize)) -> Self {
        Match { span, id: id.into() }
    }
}

#[derive(Debug, Clone)]
struct VisitedWord {
    start: usize,
    ws_folded_start: usize,
    state: StateID,
    candidate_ends: HashMap<PatternID, usize>
}

impl VisitedWord {
    fn new(start: usize, ws_folded_start: usize, dfa: &Dfa) -> Self {
        let start_cfg = StartConfig::new().anchored(Anchored::Yes);
        let new_state = dfa.start_state(&start_cfg).unwrap();

        Self {
            start,
            ws_folded_start,
            state: new_state,
            candidate_ends: HashMap::default(),
        }
    }

    fn dump(&self) -> Vec<Match> {
        self.candidate_ends.iter().map(|(id, end)| Match {
            id: *id,
            span: (self.start, *end)
        }).collect()
    }
}

fn dfa_matches_at(dfa: &Dfa, id: StateID) -> Vec<PatternID> {
    let try_end = dfa.next_eoi_state(id);
    if dfa.is_match_state(try_end) {
        (0..dfa.match_len(try_end))
            .map(|pid| dfa.match_pattern(id, pid))
            .collect()
    } else {
        vec![]
    }
}

fn word_is_whitespace(word: &str) -> bool {
    for b in word.as_bytes() {
        if !matches!(b, b' '|b'\t'|b'\n') {
            return false;
        }
    }

    true
}

#[derive(Debug, Clone)]
pub struct Search {
    dfa: Dfa,
    pos: usize,
    ws_folded_pos: usize,
    state: Vec<VisitedWord>,
    pattern_max_lens: Vec<usize>,
}

impl Search {
    pub fn compile(patterns: &[impl AsRef<str>]) -> Result<Self, Error> {
        let mut compiled_patterns = Vec::with_capacity(patterns.len());
        for pattern in patterns {
            compiled_patterns.push(Ast::parse(pattern)?);
        }
        Ok(Self::new(&compiled_patterns))
    }

    pub fn new(patterns: &[Ast]) -> Self {
        let transpiled_patterns = patterns.iter().map(Ast::to_regex).collect::<Vec<_>>();
        let pattern_max_lens = patterns.iter().map(Ast::max_bytes).collect();

        let build_cfg = dfa::dense::Config::new().match_kind(MatchKind::All);
        let dfa = dfa::dense::Builder::new()
            .configure(build_cfg)
            .build_many(&transpiled_patterns)
            .unwrap();

        Self {
            dfa,
            pos: 0,
            ws_folded_pos: 0,
            state: vec![],
            pattern_max_lens
        }
    }

    fn step_word(&mut self, haystack: &str) -> Vec<Match> {
        let mut matches = vec![];

        self.state.push(VisitedWord::new(self.pos, self.ws_folded_pos, &self.dfa));
        self.pos += haystack.len();

        let curr_word_is_whitespace = word_is_whitespace(haystack);
        if curr_word_is_whitespace {
            self.ws_folded_pos += 1;
        } else {
            self.ws_folded_pos += haystack.len();
        }

        self.state.retain_mut(|word| {
            if curr_word_is_whitespace {
                let next = self.dfa.next_state(word.state, b' ');
                if self.dfa.is_dead_state(next) {
                    matches.extend(word.dump());
                    return false;
                }
                word.state = next;
            } else {
                for &b in haystack.as_bytes() {
                    let next = self.dfa.next_state(word.state, b);
                    if self.dfa.is_dead_state(next) {
                        matches.extend(word.dump());
                        return false;
                    }
                    word.state = next;
                }
            }

            for better in dfa_matches_at(&self.dfa, word.state) {
                word.candidate_ends.insert(better, self.pos);
            }

            word.candidate_ends.retain(|candidate_pattern, candidate_pos| {
                let max_folded_pattern_len = self.pattern_max_lens[candidate_pattern.as_usize()];
                if self.ws_folded_pos - word.ws_folded_start >= max_folded_pattern_len {
                    matches.push(Match {
                        id: *candidate_pattern,
                        span: (word.start, *candidate_pos)
                    });
                    false
                } else {
                    true
                }

            });

            true
        });

        matches
    }

    pub fn next(&mut self, haystack: impl AsRef<str>) -> Vec<Match> {
        haystack
            .as_ref()
            .split_word_bounds()
            .flat_map(|w| self.step_word(w))
            .collect()
    }

    pub fn finish(&mut self) -> Vec<Match> {
        self.pos = 0;
        self.ws_folded_pos = 0;

        std::mem::take(&mut self.state)
            .iter()
            .flat_map(VisitedWord::dump)
            .collect()
    }

    pub fn reset(&mut self) {
        self.pos = 0;
        self.ws_folded_pos = 0;
        self.state.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::Search;

    #[test]
    fn definitely_complete() {
        let mut s = Search::compile(&["ab"]).unwrap();
        let haystacks = ["ab a", "b", "ab"];

        for haystack in haystacks {
            println!("Matching step \"{haystack}\"");
            for m in s.next(haystack) {
                println!("\t{:?}", m);
            }
        }

        println!("Finalizing");
        for m in s.finish() {
            println!("\t{:?}", m);
        }
    }
}
