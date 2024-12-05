//! A friendly regular expression dialect for text analytics. Typical regex features are removed/adjusted to make natural language queries easier. Unicode-aware and able to search a stream with several patterns at once. 

mod parser;
mod search;

pub use parser::{Ast, Error};
pub use search::{Match, Search};

/// A high-level interface for matching a single `reggy` pattern
#[derive(Clone)]
pub struct Pattern {
    s: Search,
}

impl Pattern {
    pub fn new(code: &str) -> Result<Self, Error> {
        let ast = Ast::parse(code)?;
        Ok(Self {
            s: Search::new(std::slice::from_ref(&ast)),
        })
    }

    pub fn findall(&mut self, haystack: &str) -> Vec<(usize, usize)> {
        let mut res: Vec<_> = self.s.next(haystack).iter().map(|m| m.span).collect();
        res.extend(self.s.finish().iter().map(|m| m.span));
        self.s.reset();
        res
    }
}

#[cfg(test)]
mod test {
    use super::{Ast, Match, Pattern, Search};

    #[test]
    fn readme_high_level() {
        let mut p = Pattern::new("dogs?").unwrap();
        assert_eq!(p.findall("cat dog dogs cats"), vec![(4, 7), (8, 12)])
    }

    #[test]
    fn readme_compile() {
        let ast = Ast::parse(r"do(gg.)?|(!CAT|CAR FAR)").unwrap();

        assert_eq!(ast.to_regex(), r"(?mi:do(?:gg\.)?|(?-i:CAT|CAR FAR))");
    }

    #[test]
    fn readme_match_incremental() {
        let mut search = Search::compile(&[r"$#?#?#.##", r"(John|Jane) Doe"]).unwrap();

        // call step() to begin searching a stream
        let jane_match = Match::new(1, (0, 8));
        assert_eq!(search.next("Jane Doe paid John"), vec![jane_match]);

        // call step() again to continue with the same search state
        // note "John Doe" matches across the step boundary
        let john_match = Match::new(1, (14, 22));
        let money_match_1 = Match::new(0, (23, 29));
        let money_match_2 = Match::new(0, (41, 48));
        assert_eq!(
            search.next(" Doe $45.66 instead of $499.00"),
            vec![john_match, money_match_1, money_match_2]
        );

        // call finish() to retrieve any pending matches once the stream is done
        assert_eq!(search.finish(), vec![]);
    }

    #[test]
    fn readme_case_sensitive_substr() {
        let mut p = Pattern::new("United States of America|(!USA)").unwrap();
        assert_eq!(
            p.findall("United states of america Usa USA"),
            vec![(0, 24), (29, 32)]
        );
    }

    #[test]
    fn leftmost_semantics() {
        let mut s = Pattern::new("a b|a").unwrap();
        assert_eq!(vec![(0, 3)], s.findall("a b"));
    }
}
