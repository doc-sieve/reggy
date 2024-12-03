mod parser;
mod search;

pub use parser::{Ast, Error};
pub use search::{Match, Search};

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
        let mut res: Vec<_> = self.s.step(haystack).iter().map(|m| m.pos).collect();
        res.extend(self.s.finish().iter().map(|m| m.pos));
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
        let ast = Ast::parse(r"do(gg)*|(!CAT|CAR)").unwrap();
        assert_eq!(r"(?i:do(?:gg)*|(?-i:CAT|CAR))", ast.to_regex());
    }

    #[test]
    fn readme_match_currency() {
        // perform an incremental search with several patterns at once
        let money = Ast::parse(r"$(\d?\d?\d,)*\d?\d?\d.\d\d").unwrap();
        let people = Ast::parse(r"(!(John|Jane) Doe)").unwrap();

        let mut search = Search::new(&[money, people]);

        // call step() to begin searching a stream
        let jane_match = Match { pos: (0, 8), id: 1 };
        assert_eq!(search.step("Jane Doe paid John"), vec![jane_match]);

        // call step() again to continue with the same search state
        // note "John Doe" matches across the step boundary
        let john_match = Match {
            pos: (14, 22),
            id: 1,
        };
        let money_match_1 = Match {
            pos: (23, 33),
            id: 0,
        };
        assert_eq!(
            search.step(" Doe $45,700.66 instead of $499.00"),
            vec![john_match, money_match_1]
        );

        // call finish() to retrieve any pending matches once the stream is done
        let money_match_2 = Match {
            pos: (45, 52),
            id: 0,
        };
        assert_eq!(search.finish(), vec![money_match_2]);
    }
}
