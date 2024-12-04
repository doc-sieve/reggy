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
struct Word {
    start: usize,
    state: StateID,
    candidate_ends: HashMap<PatternID, usize>
}

impl Word {
    fn new(start: usize, dfa: &Dfa) -> Self {
        let start_cfg = StartConfig::new().anchored(Anchored::Yes);
        let new_state = dfa.start_state(&start_cfg).unwrap();

        Self {
            start,
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

#[derive(Debug, Clone)]
pub struct Search {
    pub dfa: Dfa,
    pos: usize,
    state: Vec<Word>
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

        let build_cfg = dfa::dense::Config::new().match_kind(MatchKind::All);

        let dfa = dfa::dense::Builder::new()
            .configure(build_cfg)
            .build_many(&transpiled_patterns)
            .unwrap();

        Self {
            dfa,
            pos: 0,
            state: vec![]
        }
    }

    fn step_word(&mut self, haystack: &str) -> Vec<Match> {
        vec![]
    }

    pub fn next(&mut self, haystack: impl AsRef<str>) -> Vec<Match> {
        haystack
            .as_ref()
            .split_word_bounds()
            .flat_map(|w| self.step_word(w))
            .collect()
    }

    pub fn finish(&mut self) -> Vec<Match> {
        std::mem::take(&mut self.state)
            .iter()
            .flat_map(Word::dump)
            .collect()
    }

    pub fn reset(&mut self) {
        self.pos = 0;
        self.state.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::Search;

    #[test]
    fn definitely_complete() {
        let mut s = Search::compile(&["ab"]).unwrap();
        let haystacks = ["a", "b", "a", "b", "a", "b", "c ab", "a", "b", "a", "b"];

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
