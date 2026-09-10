# Contributing to tpt-rustir

tpt-rustir is in early, roadmap-driven development (see [`todo.md`](todo.md)
and [`spec.txt`](spec.txt)). We are not accepting pull requests at this stage
— please contribute via issues only.

## Reporting issues

Before filing, check `todo.md` to see if the item is already tracked as
planned work.

A good issue includes:

- A minimal reproduction: the `tpt` surface syntax or Rust code involved, and
  the exact command run.
- What you expected to happen vs. what actually happened.
- Which crate it concerns, if apparent (`tpt-rustir-core`, `-syntax`,
  `-solver`, `-mir`, `-macros`, or `-cli`).

## Proposing features or design changes

Open an issue describing the motivation and proposed approach before doing
any implementation work, so it can be weighed against the phased roadmap in
`todo.md`.

## License

tpt-rustir is dual-licensed under [MIT](LICENSE-MIT) or
[Apache-2.0](LICENSE-APACHE), at your option.
