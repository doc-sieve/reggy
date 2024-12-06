use std::collections::HashMap;

use regex_automata::dfa::Automaton;
use regex_automata::util::{
    primitives::{PatternID, StateID},
    start::Config as StartConfig,
};
use regex_automata::{dfa, Anchored, MatchKind};

use unicode_segmentation::UnicodeSegmentation;

use crate::{Ast, Error};

type Dfa = dfa::dense::DFA<Vec<u32>>;

/// A match object returned from a [`Search`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    /// The index of the pattern matched
    pub id: usize,
    /// The byte span of the match relative to the start of the stream
    pub span: (usize, usize),
}

impl Match {
    /// Convenience function for testing
    pub fn new(id: usize, span: (usize, usize)) -> Self {
        Match { span, id }
    }
}

#[derive(Debug, Clone)]
struct VisitedWord {
    start: usize,
    ws_folded_start: usize,
    state: StateID,
    candidate_ends: HashMap<PatternID, usize>,
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
        self.candidate_ends
            .iter()
            .map(|(id, end)| Match {
                id: id.as_usize(),
                span: (self.start, *end),
            })
            .collect()
    }
}

fn dfa_matches_at(dfa: &Dfa, id: StateID) -> Vec<PatternID> {
    let try_end = dfa.next_eoi_state(id);
    if dfa.is_match_state(try_end) {
        (0..dfa.match_len(try_end))
            .map(|pid| dfa.match_pattern(try_end, pid))
            .collect()
    } else {
        vec![]
    }
}

fn word_is_whitespace(word: &str) -> bool {
    for b in word.as_bytes() {
        if !matches!(b, b' ' | b'\t' | b'\n') {
            return false;
        }
    }

    true
}

/// A compiled searcher for multiple patterns against a stream of text
#[derive(Debug, Clone)]
pub struct Search {
    dfa: Dfa,
    pos: usize,
    ws_folded_pos: usize,
    last_word_was_ws: bool,
    push_state: bool,
    state: Vec<VisitedWord>,
    pattern_max_lens: Vec<usize>,
}

impl Search {
    /// Try to compile multiple patterns, raising any parse error encountered
    pub fn compile(patterns: &[impl AsRef<str>]) -> Result<Self, Error> {
        let mut compiled_patterns = Vec::with_capacity(patterns.len());
        for pattern in patterns {
            compiled_patterns.push(Ast::parse(pattern)?);
        }
        Ok(Self::new(&compiled_patterns))
    }

    /// Compile from already-parsed ASTs
    pub fn new(patterns: &[Ast]) -> Self {
        let transpiled_patterns = patterns
            .iter()
            .map(Ast::to_regex_internal)
            .collect::<Vec<_>>();
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
            last_word_was_ws: false,
            push_state: true,
            state: vec![],
            pattern_max_lens,
        }
    }

    fn step_word(&mut self, haystack: &str) -> Vec<Match> {
        let mut matches = vec![];
        let last_pos = self.pos;
        let last_ws_folded_pos = self.ws_folded_pos;
        let last_push_state = self.push_state;

        self.push_state = true;
        self.pos += haystack.len();

        let curr_word_is_whitespace = word_is_whitespace(haystack);

        if curr_word_is_whitespace {
            if self.last_word_was_ws {
                return matches;
            }
            self.last_word_was_ws = true;
            self.ws_folded_pos += 1;
        } else {
            self.last_word_was_ws = false;
            self.ws_folded_pos += haystack.len();
        }

        if last_push_state {
            self.state
                .push(VisitedWord::new(last_pos, last_ws_folded_pos, &self.dfa));
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
                    word.state = next;
                }
            }

            for better in dfa_matches_at(&self.dfa, word.state) {
                word.candidate_ends.insert(better, self.pos);
            }

            word.candidate_ends
                .retain(|candidate_pattern, candidate_pos| {
                    let max_folded_pattern_len =
                        self.pattern_max_lens[candidate_pattern.as_usize()];
                    if self.ws_folded_pos - word.ws_folded_start >= max_folded_pattern_len {
                        let m = Match {
                            id: candidate_pattern.as_usize(),
                            span: (word.start, *candidate_pos),
                        };

                        // preserve leftmost-longest
                        let mut found = false;
                        for already in &mut matches {
                            if already.id == m.id {
                                already.span.0 = already.span.0.min(m.span.0);
                                found = true;
                                break;
                            }
                        }

                        if !found {
                            matches.push(m);
                        }
                        false
                    } else {
                        true
                    }
                });

            true
        });

        matches
    }

    /// Step through a chunk of text, yielding any matches that are definitely-complete
    pub fn next(&mut self, haystack: impl AsRef<str>) -> Vec<Match> {
        if self.pos > 0 {
            self.push_state = false;
        }

        let words = haystack.as_ref().split_word_bounds();
        words.flat_map(|w| self.step_word(w)).collect()
    }

    /// Yield any pending, not-definitely-complete matches
    pub fn peek_finish(&self) -> Vec<Match> {
        let match_iter = self.state.iter().flat_map(VisitedWord::dump);
        let mut filtered_matches: Vec<Match> = vec![];

        // preserve leftmost-longest
        let mut found = false;
        for m in match_iter {
            for already in &mut filtered_matches {
                if already.id == m.id {
                    already.span.0 = already.span.0.min(m.span.0);
                    found = true;
                    break;
                }
            }

            if !found {
                filtered_matches.push(m);
            }
        }

        filtered_matches
    }

    /// Clear the match state, yielding any pending, not-definitely-complete matches
    pub fn finish(&mut self) -> Vec<Match> {
        let res = self.peek_finish();
        self.reset();
        res
    }

    /// Clear the match state
    pub fn reset(&mut self) {
        self.pos = 0;
        self.ws_folded_pos = 0;
        self.push_state = true;
        self.state.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{Match, Search};

    #[test]
    fn whitespace() {
        let mut s = Search::compile(&["a b"]).unwrap();
        let haystacks = ["ab    a ", " \t", "b", "ab"];

        assert_eq!(s.next(haystacks[0]), vec![]);
        assert_eq!(s.next(haystacks[1]), vec![]);
        assert_eq!(s.next(haystacks[2]), vec![Match::new(0, (6, 11))]);

        assert_eq!(s.next(haystacks[3]), vec![]);
        assert_eq!(s.finish(), vec![]);
    }

    #[test]
    fn definitely_complete() {
        let mut s = Search::compile(&["abb?"]).unwrap();

        assert_eq!(s.next("ab"), vec![]);
        assert_eq!(s.finish(), vec![Match::new(0, (0, 2))]);

        s.reset();

        assert_eq!(s.next("abb"), vec![Match::new(0, (0, 3))]);
        assert_eq!(s.finish(), vec![]);
    }

    #[test]
    fn chunk_invariant_fuzz() {
        use rand::prelude::*;

        let mut rng = SmallRng::seed_from_u64(1);

        const MIN_HAYSTACK_LEN: usize = 80;
        const MIN_PATTERN_LEN: usize = 4;
        const N_PATTERNS: usize = 4;
        const N_PARTITIONS: usize = 5;
        const N_PARTITION_RUNS: usize = 5;

        fn random_pattern(rng: &mut SmallRng) -> String {
            let d: [u8; MIN_PATTERN_LEN] = rng.gen();
            let mut res = String::with_capacity(MIN_PATTERN_LEN);
            for i in 0..MIN_PATTERN_LEN {
                res.push(((d[i] % 3) + 97) as u8 as char);
                if rng.gen::<i32>() % 2 == 0 {
                    res.push('?');
                }
            }

            res
        }

        let patterns: Vec<_> = (0..N_PATTERNS).map(|_| random_pattern(&mut rng)).collect();
        println!("Patterns: {:?}", patterns);

        fn random_haystack(rng: &mut SmallRng) -> String {
            let mut res = String::with_capacity(MIN_HAYSTACK_LEN);
            for _ in 0..MIN_HAYSTACK_LEN {
                let d: u8 = rng.gen();
                res.push(((d % 3) + 97) as u8 as char);
                if rng.gen::<i32>() % 2 == 0 {
                    res.push(' ');
                }
            }

            res
        }

        let haystack = random_haystack(&mut rng);
        println!("Haystack: {:?}\n", haystack);

        let mut s = Search::compile(&patterns).unwrap();

        let mut all_match_sets = vec![];

        for i in 0..N_PARTITION_RUNS {
            s.reset();
            let mut matches = vec![];

            let d: [usize; N_PARTITIONS] = rng.gen();
            let mut d: Vec<_> = d.iter().map(|i| i % haystack.len()).collect();
            d.sort();
            println!("Partition {}: {:?}", i, d);

            let mut hay_i = 0;
            for partition in d {
                if partition > hay_i {
                    matches.extend(s.next(&haystack[hay_i..partition]));
                    hay_i = partition;
                }
            }

            matches.extend(s.next(&haystack[hay_i..haystack.len()]));
            matches.extend(s.finish());

            all_match_sets.push(matches);
        }

        s.reset();
        let mut canonical_matches = vec![];
        canonical_matches.extend(s.next(haystack));
        canonical_matches.extend(s.finish());

        for i in 0..N_PARTITION_RUNS {
            assert_eq!(
                all_match_sets[i], canonical_matches,
                "Partition {} did not match canon",
                i
            );
        }
    }
}
