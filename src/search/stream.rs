use std::collections::VecDeque;
use std::io::{self, BufRead};
use std::str::Utf8Error;

use crate::{Match, Search};

/// A stream search for the provided BufReader
/// Will return [`StreamSearchError`]s for invalid UTF-8 or IO errors
pub struct StreamSearch<'a, R: io::BufRead> {
    search: &'a mut Search,
    reader: R,
    res_buf: VecDeque<Match>,
    closed: bool,
}

impl<'a, R: BufRead> StreamSearch<'a, R> {
    pub(super) fn new(search: &'a mut Search, reader: R) -> Self {
        Self {
            search,
            reader,
            res_buf: VecDeque::new(),
            closed: false,
        }
    }
}

impl<'a, R: io::BufRead> Iterator for StreamSearch<'a, R> {
    type Item = Result<Match, SearchStreamError>;

    fn next(&mut self) -> Option<Result<Match, SearchStreamError>> {
        if let Some(res) = self.res_buf.pop_front() {
            return Some(Ok(res));
        } else if self.closed {
            return None;
        }

        loop {
            let buf = self.reader.fill_buf();
            match buf {
                Ok(buf) => {
                    if buf.is_empty() {
                        self.closed = true;
                        self.res_buf = VecDeque::from(self.search.finish());
                        return self.res_buf.pop_front().map(Ok);
                    } else {
                        match self.search.next_bytes(buf) {
                            Ok(res) => {
                                self.res_buf = VecDeque::from(res);
                                if let Some(res) = self.res_buf.pop_front() {
                                    let len = buf.len();
                                    self.reader.consume(len);

                                    return Some(Ok(res));
                                }
                            }
                            Err(e) => return Some(Err(e)),
                        }
                        let len = buf.len();
                        self.reader.consume(len);
                    }
                }
                Err(e) => return Some(Err(e.into())),
            }

            if let Some(res) = self.res_buf.pop_front() {
                return Some(Ok(res));
            } else if self.closed {
                return None;
            }
        }
    }
}

/// An error raised while searching a stream
#[derive(Debug)]
pub enum SearchStreamError {
    IOError(io::Error),
    Utf8Error,
}

impl From<io::Error> for SearchStreamError {
    fn from(e: io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<Utf8Error> for SearchStreamError {
    fn from(_: Utf8Error) -> Self {
        Self::Utf8Error
    }
}
