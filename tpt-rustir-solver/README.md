# tpt-rustir-solver

Proof automation for tpt-rustir: e-graph equality saturation
([`egg`](https://docs.rs/egg)) with an optional SMT ([`z3`](https://docs.rs/z3))
fallback, wired together into a small tactic DSL.

## What's here

- `lang`: the `Expr` e-graph language (`+`, `*`, `and`/`or`/`not`, `le`,
  `Nat`/`Bool` connectives) plus two rewrite-rule sets — `definitional_rules`
  (Peano recursion) and `algebraic_rules` (commutativity, associativity, De
  Morgan, excluded middle, `le` reflexivity).
- `tactic`: the tactic engine DSL — `Simp` (equality saturation), `Induction`
  (structural induction on `Nat`), `Auto` (definitional unfolding →
  induction → SMT), `OrElse` (try alternatives in order), and `Smt`.
- `smt` (behind the `smt` feature): translates a goal into a Z3 query and
  discharges it via unsat-of-negation. Requires a system Z3/libclang install,
  so it's off by default.
- `specs`: the shared spec-string verification pipeline (`<lhs> = <rhs>`
  solver obligations, or a tpt term to kernel-type-check) used by both the
  `#[tpt::spec]`/`#[tpt::verify]` proc-macros and `cargo tpt`.
- `from_core`: translates `tpt-rustir-core` terms into the solver's `Expr`
  language where possible.

## Usage

```rust
use tpt_rustir_solver::tactic::{run, Goal, Tactic};

let goal = Goal::new("(+ (+ a b) c)", "(+ c (+ b a))");
assert!(run(&Tactic::Simp, &goal).is_ok());
```

The purely recursive definitional rules alone (no commutativity) can't prove
theorems that need generalizing over an argument — the Peano right-identity
`n + 0 = n` needs structural induction on `n`:

```rust
use tpt_rustir_solver::lang::definitional_rules;
use tpt_rustir_solver::tactic::{run, run_with_rules, Goal, Tactic};

let goal = Goal::new("(+ n zero)", "n");

// The purely recursive definitions alone can't reach this — there's no
// rule that fires on an opaque `n` without also having commutativity.
assert!(run_with_rules(&goal, &definitional_rules()).is_err());

// Structural induction on `n` proves it.
run(&Tactic::Induction { var: "n".to_string() }, &goal).unwrap();

// `Auto` tries definitional unfolding first, then falls back to induction.
run(&Tactic::Auto { induction_var: Some("n".to_string()) }, &goal).unwrap();
```

(`Tactic::Simp` uses the full `algebraic_rules` set, which *does* include
commutativity — it can actually prove this particular goal directly, by
rewriting to `zero + n` and applying the definitional rule for that. The
distinction above is specifically between the bare recursive definitions and
induction.)

Enable the SMT fallback (requires a system Z3 install):

```toml
tpt-rustir-solver = { version = "0.1", features = ["smt"] }
```

## Testing

```sh
cargo test -p tpt-rustir-solver          # default features
cargo test -p tpt-rustir-solver --features smt  # requires a system Z3 install
```

## Changelog

See [`CHANGELOG.md`](CHANGELOG.md).

## License

Dual-licensed under [MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE),
at your option.
