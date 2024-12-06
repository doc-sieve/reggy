use core::str;
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead};
use std::str::Utf8Error;

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


#[derive(Debug, Clone)]
enum Utf8RaggedEdge {
    Zero,
    One(u8),
    Two(u8, u8),
    Three(u8, u8, u8)
}

/// An error raised while searching a stream 
#[derive(Debug)]
pub enum SearchStreamError {
    IOError(io::Error),
    Utf8Error
}

impl From::<io::Error> for SearchStreamError {
    fn from(e: io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From::<Utf8Error> for SearchStreamError {
    fn from(_: Utf8Error) -> Self {
        Self::Utf8Error
    }
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
    utf8_ragged_edge: Utf8RaggedEdge
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
            utf8_ragged_edge: Utf8RaggedEdge::Zero
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
    /// This function panics if called while the stream is partially-through a utf-8 char
    /// (after a call to next_bytes)
    pub fn next(&mut self, haystack: impl AsRef<str>) -> Vec<Match> {
        if !matches!(self.utf8_ragged_edge, Utf8RaggedEdge::Zero) {
            panic!("utf-8 ragged edge");
        }

        if self.pos > 0 {
            self.push_state = false;
        }

        let words = haystack.as_ref().split_word_bounds();
        words.flat_map(|w| self.step_word(w)).collect()
    }

    /// Step through a chunk of bytes, yielding any matches that are definitely-complete
    /// This function panics if given definitely-invalid utf8
    /// An incomplete character may fill the last 1-3 bytes
    /// In which case, the character fragment will be completed on the next call to next_bytes
    pub fn next_bytes(&mut self, haystack: &[u8]) -> Result<Vec<Match>, SearchStreamError> {
        // no choice but to re-allocate? :(
        let mut v = vec![];
        let haystack_adj = match self.utf8_ragged_edge {
            Utf8RaggedEdge::Zero => haystack,
            Utf8RaggedEdge::One(a) => {
                v.reserve(haystack.len() + 1);
                v.push(a);
                v.extend(haystack);
                v.as_slice()
            },
            Utf8RaggedEdge::Two(a, b) => {
                v.reserve(haystack.len() + 1);
                v.push(a);
                v.push(b);
                v.extend(haystack);
                v.as_slice()
            }
            Utf8RaggedEdge::Three(a, b, c) => {
                v.reserve(haystack.len() + 1);
                v.push(a);
                v.push(b);
                v.push(c);
                v.extend(haystack);
                v.as_slice()
            }
        };

        let haystack_str = match str::from_utf8(haystack_adj) {
            Ok(s) => {
                self.utf8_ragged_edge = Utf8RaggedEdge::Zero;
                s
            },
            Err(e) => {
                let error_before_end = e.error_len();
                if error_before_end.is_some() {
                    return Err(SearchStreamError::Utf8Error)
                } else {
                    let s = std::str::from_utf8(&haystack_adj[..e.valid_up_to()]).unwrap();
                    match haystack_adj.len() - e.valid_up_to() {
                        1 => {
                            self.utf8_ragged_edge = Utf8RaggedEdge::One(
                                haystack_adj[haystack_adj.len() - 1]
                            )
                        },
                        2 => {
                            self.utf8_ragged_edge = Utf8RaggedEdge::Two(
                                haystack_adj[haystack_adj.len() - 2],
                                haystack_adj[haystack_adj.len() - 1]
                            )
                        },
                        3 => {
                            self.utf8_ragged_edge = Utf8RaggedEdge::Three(
                                haystack_adj[haystack_adj.len() - 3],
                                haystack_adj[haystack_adj.len() - 2],
                                haystack_adj[haystack_adj.len() - 1],
                            )
                        }
                        _ => panic!()
                    }
                    s
                }
            }
        };

        let words = haystack_str.split_word_bounds();
        Ok(words.flat_map(|w| self.step_word(w)).collect())
    }

    /// Iterate over a buffered reader
    pub fn iter<'a, R: BufRead>(&'a mut self, reader: R) -> StreamSearch<'a, R> {
        StreamSearch {
            search: self,
            reader,
            res_buf: VecDeque::new(),
            closed: false
        }
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

pub struct StreamSearch<'a, R: io::BufRead> {
    search: &'a mut Search,
    reader: R,
    res_buf: VecDeque<Match>,
    closed: bool
}

impl<'a, R: io::BufRead> Iterator for StreamSearch<'a, R> {
    type Item = Result<Match, SearchStreamError>;

    fn next(&mut self) -> Option<Result<Match, SearchStreamError>> {
        if let Some(res) = self.res_buf.pop_front() {
            return Some(Ok(res))
        } else if self.closed {
            return None
        }

        let buf = self.reader.fill_buf();

        match buf {
            Ok(buf) => {
                if buf.is_empty() {
                    self.closed = true;
                    self.res_buf = VecDeque::from(self.search.finish());
                } else {
                    match self.search.next_bytes(buf) {
                        Ok(res) => self.res_buf = VecDeque::from(res),
                        Err(e) => return Some(Err(e))
                    }
                    let len = buf.len();
                    self.reader.consume(len);
                }
            },
            Err(e) => return Some(Err(e.into())),
        }

        self.res_buf.pop_front().map(|m| Ok(m))
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
}
