# Lexer literal rules

Status: accepted (ticket GN0SM0R). To be folded into quantities.md 2
when that file is next open for editing.

- Q007: the lexer recognises a built-in unit atom (quantities.md 3)
  after spaces or tabs following a number. The fix-it deletes the
  whitespace. `%` is excluded because it may be an operator. The joined
  literal is still emitted so parsing continues. A number is never
  legally followed by a word, so `5 in` is always the split mistake.
- A fraction or mixed number is part of a literal only when a unit
  follows it immediately. A bare `3/4` is number, `/`, number and a bare
  `1-1/2` is a subtraction; the parser reads division and subtraction.
- Literals stay as exact source text (`1-1/2`, `1e-3`); sema converts
  them to rationals, so no float is ever built.
- Compound-literal order is judged by built-in unit sizes within one of
  Length, Time, Angle, Mass; any other unit is a lex error there.
- Lexical errors use E001 syntax-error (lints.md).
