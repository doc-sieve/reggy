# Reggy

Friendly regular expressions for text analytics. Typical regex features are removed/adjusted to make natural language queries easier and to strictly limit memory/runtime. Able to match streaming text. 

[API Docs](https://doc-sieve.github.io/reggy)

## Pattern Language

`Reggy` is case-insensitive by default. Spaces match any amount of whitespace (i.e. `\s+`). All the reserved characters mentioned below (`\`, `(`, `)`, `?`, `|`, and `!`) may be escaped with a backslash for a literal match. Patterns are surrounded by implicit word boundaries (i.e. `\b`).

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