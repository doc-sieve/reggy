# Reggy

A friendly regular expression dialect for text analytics. Typical regex features are removed/adjusted to make natural language queries easier. Unicode-aware and able to search a stream with several patterns at once. 

`cargo add reggy`

## API Usage

Use the high-level [`Pattern`](https://doc-sieve.github.io/reggy/reggy/struct.Pattern.html) struct for simple search.
```rust
let mut p = Pattern::new("dogs?").unwrap();
assert_eq!(
    p.findall("cat dog dogs cats"),
    vec![(4, 7), (8, 12)]
);
```

Use the [`Ast`](https://doc-sieve.github.io/reggy/reggy/enum.Ast.html) struct to transpile to [normal](https://docs.rs/regex/) regex syntax.[^1]
```rust
let ast = Ast::parse(r"do(gg.)?|(!CAT|CAR FAR)").unwrap();
assert_eq!(
    ast.to_regex(),
    r"(?mi:do(?:gg\.)?|(?-i:CAT|CAR FAR))"
);
```

### Search a Stream

Use the [`Search`](https://doc-sieve.github.io/reggy/reggy/struct.Search.html) struct to search a stream with several patterns at once.
```rust
let mut search = Search::compile(&[
    r"$#?#?#.##",
    r"(John|Jane) Doe",
]).unwrap();
```

Call `Search::next` to begin searching. It will yield any matches deemed [definitely-complete](#definitely-complete-matches) immediately.
```rust
let jane_match = Match::new(1, (0, 8));
assert_eq!(
    search.next("Jane Doe paid John"),
    vec![jane_match]
);
```

Call `Search::next` again to continue with the same search state.
Note that `"John Doe"` matched across the `next` boundary, and spans are relative to the start of the stream.
```rust
let john_match = Match::new(1, (14, 22));
let money_match_1 = Match::new(0, (23, 29));
let money_match_2 = Match::new(0, (41, 48));
assert_eq!(
    search.next(" Doe $45.66 instead of $499.00"),
    vec![john_match, money_match_1, money_match_2]
);
```

Call `Search::finish` to collect any not-[definitely-complete matches](#definitely-complete-matches) once the stream is closed.
```rust
assert_eq!(search.finish(), vec![]);
```

See more in the [API docs](https://doc-sieve.github.io/reggy).

## Pattern Language

`Reggy` is case-insensitive by default. Spaces match any amount of whitespace (i.e. `\s+`). All the reserved characters mentioned below (`\`, `(`, `)`, `{`, `}`, `?`, `|`, `#`, and `!`) may be escaped with a backslash for a literal match. Patterns are surrounded by implicit [unicode word boundaries](https://unicode.org/reports/tr29) (i.e. `\b`). Empty patterns or subpatterns are not permitted.

### Examples

*Make a character optional with* `?`

`dogs?` matches `dog` and `dogs`

*Create two or more alternatives with* `|`

`dog|cat` matches `dog` and `cat`

*Create a sub-pattern with* `(...)`

`the qualit(y|ies) required` matches `the quality required` and `the qualities required`

`the only( one)? around` matches `the only around` and `the only one around`

*Create a case-sensitive sub-pattern with* `(!...)`

`United States of America|(!USA)` matches `USA`, not `usa`

*Match digits with* `#`

`#.##` matches `3.14`

## Definitely-Complete Matches

`Reggy` follows greedy matching semantics. A pattern may match after one step of a stream, yet may match a longer form depending on the next step. For example, `ab|abb` will match `s.next("ab")`, but a subsequent call to `s.next("b")` would create a longer match, `"abb"`, which should supercede the match `"ab"`.

`Search` only yields matches once they are definitely complete and cannot be superceded by future `next` calls. Each pattern has a [maximum byte length](https://doc-sieve.github.io/reggy/reggy/enum.Ast.html#method.max_bytes) `L`, counting contiguous whitespace as 1 byte.[^2] Once `reggy` has streamed at most `L` bytes past the start of a match without superceding it, that match will be yielded. Matches may be yielded earlier if the DFA reaches a dead state.

As a consequence, **the `Match`es returned by a given `Search` are the same regardless of how a given haystack stream is segmented**. `Search::next` returns `Match`es as soon as it practically can while respecting this invariant.

## Implementation

The pattern language is parsed with [`lalrpop`](https://lalrpop.github.io/lalrpop) ([grammar](https://github.com/doc-sieve/reggy/blob/main/src/parser/grammar.lalrpop)).

The search routines use a [`regex_automata::dense::DFA`](https://docs.rs/regex-automata/latest/regex_automata/dfa/dense/struct.DFA.html). Compared to other regex engines, the dense DFA is memory-intensive and slow to construct, but searches are fast. Unicode word boundaries are handled by the [`unicode_segmentation`](https://docs.rs/unicode-segmentation/latest) crate.

[^1]: The resulting patterns are equivalent, except that `reggy` parses any continuous substring of spaces in the pattern as `\s+`, which is transpiled as ` `, and surrounds patterns with implicit word boundaries, which are not transpiled.

[^2]: This is why unbounded quantifiers are absent from `reggy`. When a pattern requires `*` or `+`, users should choose an upper limit (`{0,n}`, `{1,n}`) instead. Relying on the DFA's dead state to flush matches is a bad idea when the number of patterns increases; it only takes one unlucky pattern to keep the DFA alive for an indeterminate number of stream steps.