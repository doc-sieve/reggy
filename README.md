# Reggy

Friendly regular expressions for text analytics. Typical regex features are removed/adjusted to make natural language queries easier. Able to incrementally match streaming text.

## API Usage

```rust
// use the high-level Pattern API for simple use cases
let mut p = Pattern::new("dogs?").unwrap();
assert_eq!(p.findall("cat dog dogs cats"), vec![(4, 7), (8, 12)])

// transpile to normal (https://docs.rs/regex/) syntax
let ast = Ast::parse(r"do(gg)*|(!CAT|CAR)").unwrap();
assert_eq!(r"(?i:do(?:gg)*|(?-i:CAT|CAR))", ast.to_regex());

// perform an incremental search with several patterns at once
let money = Ast::parse(r"$(\d?\d?\d,)*\d?\d?\d.\d\d").unwrap();
let people = Ast::parse(r"(!(John|Jane) Doe)").unwrap();

let mut search = Search::new(&[money, people]);

// call step() to begin searching a stream
let jane_match = Match { pos: (0, 8), id: 1 };
assert_eq!(search.step("Jane Doe paid John"), vec![jane_match]);

// call step() again to continue with the same search state
// note "John Doe" matches across the step boundary
let john_match = Match { pos: (14, 22), id: 1 };
let money_match_1 = Match { pos: (23, 33), id: 0 };
assert_eq!(search.step(" Doe $45,700.66 instead of $499.00"), vec![john_match, money_match_1]);

// call finish() to retrieve any pending matches once the stream is done
let money_match_2 = Match { pos: (45, 52), id: 0 };
assert_eq!(search.finish(), vec![money_match_2] );
```

## Pattern Language

`Reggy` is case-insensitive by default. Spaces match any amount of whitespace (i.e. `\s+`). All the reserved characters mentioned below (`\`, `(`, `)`, `?`, `|`, `*`, `+`, and `!`) may be escaped with a backslash for a literal match. Patterns are surrounded by implicit [unicode word boundaries](https://unicode.org/reports/tr29/) (i.e. `\b`).

### Examples

*Make a letter optional with `?`*

`dogs?` matches `dog` and `dogs`

*Create two or more options with `|`*

`dog|cat` matches `dog` and `cat`

*Perform operations on groups of characters with `(...)`*

`the qualit(y|ies) required` matches `the quality required` and `the qualities required`

`the only( one)? around` matches `the only around` and `the only one around`

*Create a case-sensitive group with `(!...)`*

`United States of America|(!USA)` matches `USA`, not `usa`

*Match digits with `\d`*

`\d.\d\d` matches `3.14`

*Match zero-or-more characters with `*`, or one-or-more characters with `+`*

`$(\d?\d?\d,)*\d?\d?\d.\d\d` matches `$20.66` and `$4,670,055.32`
