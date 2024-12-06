# `reggy`

A friendly regular expression dialect for text analytics. Typical regex features are removed/adjusted to make natural language queries easier. Unicode-aware and able to search a stream with several patterns at once. 

## Should I Use `reggy`?

If you are working on a text processing problem with streaming datasets or hand-tuned regexes for natural language, you may find the feature set compelling.

| Crate                                             | Match Streams? | Case Insensitivity?                                                                | Pattern Flexibility? |
|---------------------------------------------------|----------------|------------------------------------------------------------------------------------|----------------------|
| [`aho-corasick`]( https://docs.rs/aho-corasick/ ) | ✅             | simple ASCII                                                                       | string set           |
| [`regex`]( https://docs.rs/regex )                | ❌             | [Unicode best-effort](https://www.unicode.org/reports/tr18/#Simple_Loose_Matches)  | full-featured regex  |
| `reggy`                                           | ✅             | [Unicode best-effort]( https://www.unicode.org/reports/tr18/#Simple_Loose_Matches) | regex subset         |

## API Usage

Use the high-level [`Pattern`](https://doc-sieve.github.io/reggy/reggy/struct.Pattern.html) struct for simple search.
```rust
let mut p = Pattern::new("dogs?")?;
assert_eq!(
    p.findall_spans("cat dog dogs cats"),
    vec![(4, 7), (8, 12)]
);
```

Use the [`Ast`](https://doc-sieve.github.io/reggy/reggy/enum.Ast.html) struct to transpile to [normal](https://docs.rs/regex/) regex syntax.
```rust
let ast = Ast::parse(r"dog(gy)?|dawg|(!CAT|KITTY CAT)")?;
assert_eq!(
    ast.to_regex(),
    r"\b(?mi:dog(?:gy)?|dawg|(?-i:CAT|KITTY\s+CAT))\b"
);
```

### Stream a File

In this example, we will count the matches of a set of patterns within a file without loading it into memory. Use the [`Search`](https://doc-sieve.github.io/reggy/reggy/struct.Search.html) struct to search a stream with several patterns at once.

Create a `BufReader` for the text.
```rust
use std::fs::File;
use std::io::{self, BufReader};

let f = File::open("tests/samples/republic_plato.txt")?;
let f = BufReader::new(f);
```

Compile the search object.

```rust
let patterns = [
    r"yes|(very )?true|certainly|quite so|I have no objection|I agree",
    r"\?",
];

let mut pattern_counts = [0; 2];

let mut search = Search::compile(&patterns).unwrap();
```

Call `Search::iter` to create a [`StreamSearch`](https://doc-sieve.github.io/reggy/reggy/struct.StreamSearch.html). Any IO errors or malformed UTF-8 will be return a [`SearchStreamError`](https://doc-sieve.github.io/reggy/reggy/struct.SearchStreamError.html). 

```rust
for result in search.iter(f) {
    match result {
        Ok(m) => {
            pattern_counts[m.id] += 1;
        }
        Err(e) => {
            println!("Stream Error {e:?}");
            break;
        }
    }
}

println!("Assent Count:   {}", pattern_counts[0]);
println!("Question Count: {}", pattern_counts[1]);
// Assent Count:   1467
// Question Count: 1934
```

### Walk a Stream Manually

```rust
let mut search = Search::compile(&[
    r"$#?#?#.##",
    r"(John|Jane) Doe"
])?;
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
Note that `"John Doe"` matched across the chunk boundary, and spans are relative to the start of the stream.
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

`reggy` is case-insensitive by default. Spaces match any amount of whitespace (i.e. `\s+`). All the reserved characters mentioned below (`\`, `(`, `)`, `{`, `}`, `,`, `?`, `|`, `#`, and `!`) may be escaped with a backslash for a literal match. Patterns are surrounded by implicit [unicode word boundaries](https://unicode.org/reports/tr29) (i.e. `\b`). Empty patterns or subpatterns are not permitted.

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

*Match exactly n times with* `{n}`*, or between n and m times with* `{n,m}`

`(very ){1,4}strange` matches `very very very strange`

## Definitely-Complete Matches

`reggy` follows "leftmost-longest", greedy matching semantics. A pattern may match after one step of a stream, yet may match a longer form depending on the next step. For example, `abb?` will match `s.next("ab")`, but a subsequent call to `s.next("b")` would create a longer match, `"abb"`, which should supercede the match `"ab"`.

`Search` only yields matches once they are definitely complete and cannot be superceded by future `next` calls. Each pattern has a [maximum byte length](https://doc-sieve.github.io/reggy/reggy/enum.Ast.html#method.max_bytes) `L`, counting contiguous whitespace as 1 byte.[^1] Once `reggy` has streamed at most `L` bytes past the start of a match without superceding it, that match will be yielded.

As a consequence, **results of a given `Search` are the same regardless of how a given haystack stream is chunked**. `Search::next` returns `Match`es as soon as it practically can while respecting this invariant.

## Implementation

The pattern language is parsed with [`lalrpop`](https://lalrpop.github.io/lalrpop) ([grammar](https://github.com/doc-sieve/reggy/blob/main/src/parser/grammar.lalrpop)).

The search routines use a [`regex_automata::dense::DFA`](https://docs.rs/regex-automata/latest/regex_automata/dfa/dense/struct.DFA.html). Compared to other regex engines, the dense DFA is memory-intensive and slow to construct, but searches are fast. Unicode word boundaries are handled by the [`unicode_segmentation`](https://docs.rs/unicode-segmentation/latest) crate.

[^1]: This is why unbounded quantifiers are absent from `reggy`. When a pattern requires `*` or `+`, users should choose an upper limit (`{0,n}`, `{1,n}`) instead.