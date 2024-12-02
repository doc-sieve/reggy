mod parser;
mod search;

pub use parser::{parse, Ast, Error};
pub use search::{Search, Match};
