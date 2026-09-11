# tpt-rustir-syntax

Surface syntax, parser, and pretty-printer for the tpt proof language, plus
its REPL — the human-facing layer over
[`tpt-rustir-core`](../tpt-rustir-core)'s kernel.

## What's here

- The tpt surface (concrete) syntax: a small dependently-typed language with
  lambdas, Pi/Sigma types, `let`, and the `Nat`/`Bool` builtins.
- A lexer/parser built on `chumsky`, producing an AST directly (there is no
  separate CST stage yet).
- Elaboration (`parse_and_lower`): AST → `tpt-rustir-core` terms.
- A pretty-printer for terms and proof states.
- Span-tracked parse errors (rendering is currently `Debug`-ish, not yet
  polished for end users).
- A REPL binary (`cargo run -p tpt-rustir-syntax --bin repl`).

## Syntax overview

| Form | Example |
| ---- | ------- |
| Lambda | `\x : Nat => succ x` |
| Application | `(\x : Nat => succ x) 2` |
| Pi (dependent function) type | inferred, e.g. `(x : Nat) -> Nat` |
| Sigma pair, with ascription | `((2, true) : (Nat * Bool))` |
| Projections | `fst p`, `snd p` |
| `let` binding | `let x : Nat = 3 in succ x` |
| Universes | `Type0` |

## Usage

```rust
let term = tpt_rustir_syntax::parse_and_lower("\\x : Nat => succ x").unwrap();
```

Or interactively:

```sh
cargo run -p tpt-rustir-syntax --bin repl
```

```
tpt> \x : Nat => succ x
  : (x : Nat) -> Nat
  = \x : Nat => succ x

tpt> ((2, true) : (Nat * Bool))
  : (x : Nat) * Bool
  = (succ (succ zero), true)
```

## Testing

```sh
cargo test -p tpt-rustir-syntax
```

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE),
at your option.
