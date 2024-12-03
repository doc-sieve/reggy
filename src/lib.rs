mod parser;
mod search;

pub use parser::{Ast, Error};
pub use search::{Match, Search};

pub struct Pattern {
    s: Search
}

impl Pattern {
    pub fn new(code: &str) -> Result<Self, Error> {
        let ast = Ast::parse(code)?;
        Ok(Self {
            s: Search::new(std::slice::from_ref(&ast))
        })
    }

    pub fn findall(&mut self, haystack: &str) -> Vec<(usize, usize)> {
        let mut res: Vec<_> = self.s.step(haystack).iter().map(|m| m.pos).collect();
        res.extend(self.s.finish().iter().map(|m| m.pos));
        self.s.reset();
        res
    }
}