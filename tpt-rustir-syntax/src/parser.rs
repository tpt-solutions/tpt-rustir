//! Recursive-descent parser built with `chumsky` combinators.
//!
//! Grammar (low to high precedence):
//! ```text
//! expr    := ann
//! ann     := arrow (":" arrow)?
//! arrow   := sigma ("->" arrow)?          -- right-assoc
//! sigma   := app ("*" sigma)?             -- right-assoc
//! app     := atom+                        -- left-assoc application
//! atom    := ident | nat-literal
//!          | "(" ident ":" expr ")" ("->" expr | "*" expr)   -- dependent binder
//!          | "(" expr ("," expr)? ")"                        -- grouping / pair
//!          | "\" ident ":" expr "=>" expr                    -- lambda
//!          | "let" ident ":" expr "=" expr "in" expr
//! ```

use chumsky::prelude::*;

use crate::ast::{Expr, ExprKind};

const KEYWORDS: &[&str] = &["let", "in"];

pub fn parser() -> impl Parser<char, Expr, Error = Simple<char>> {
    let ident = text::ident().padded().try_map(|s: String, span| {
        if KEYWORDS.contains(&s.as_str()) {
            Err(Simple::custom(span, format!("`{s}` is a reserved keyword")))
        } else {
            Ok(s)
        }
    });

    recursive(|expr: Recursive<char, Expr, Simple<char>>| {
        let nat_lit = text::int(10)
            .padded()
            .map_with_span(|s: String, span| Expr::new(ExprKind::NatLit(s.parse().unwrap()), span));

        let var = ident.map_with_span(|name, span| Expr::new(ExprKind::Var(name), span));

        let lambda = just('\\')
            .padded()
            .ignore_then(ident)
            .then_ignore(just(':').padded())
            .then(expr.clone())
            .then_ignore(just("=>").padded())
            .then(expr.clone())
            .map_with_span(|((name, ty), body), span| {
                Expr::new(ExprKind::Lambda(name, Box::new(ty), Box::new(body)), span)
            });

        let let_expr = text::keyword("let")
            .padded()
            .ignore_then(ident)
            .then_ignore(just(':').padded())
            .then(expr.clone())
            .then_ignore(just('=').padded())
            .then(expr.clone())
            .then_ignore(text::keyword("in").padded())
            .then(expr.clone())
            .map_with_span(|(((name, ann_ty), val), body), span| {
                Expr::new(
                    ExprKind::Let(name, Box::new(ann_ty), Box::new(val), Box::new(body)),
                    span,
                )
            });

        let dependent_binder = just('(')
            .padded()
            .ignore_then(ident)
            .then_ignore(just(':').padded())
            .then(expr.clone())
            .then_ignore(just(')').padded())
            .then(choice((
                just("->")
                    .padded()
                    .ignore_then(expr.clone())
                    .map(|b| (true, b)),
                just('*')
                    .padded()
                    .ignore_then(expr.clone())
                    .map(|b| (false, b)),
            )))
            .map_with_span(|((name, ty), (is_pi, body)), span| {
                let kind = if is_pi {
                    ExprKind::Pi(name, Box::new(ty), Box::new(body))
                } else {
                    ExprKind::Sigma(name, Box::new(ty), Box::new(body))
                };
                Expr::new(kind, span)
            });

        let paren_group = just('(')
            .padded()
            .ignore_then(expr.clone())
            .then(just(',').padded().ignore_then(expr.clone()).or_not())
            .then_ignore(just(')').padded())
            .map_with_span(|(a, maybe_b), span| match maybe_b {
                Some(b) => Expr::new(ExprKind::Pair(Box::new(a), Box::new(b)), span),
                None => a,
            });

        let atom = choice((
            dependent_binder,
            paren_group,
            lambda,
            let_expr,
            nat_lit,
            var,
        ));

        let app = atom
            .clone()
            .then(atom.repeated())
            .map_with_span(|(head, args), span| {
                args.into_iter().fold(head, |f, a| {
                    Expr::new(ExprKind::App(Box::new(f), Box::new(a)), span.clone())
                })
            });

        let sigma = app
            .clone()
            .then(just('*').padded().ignore_then(app.clone()).repeated())
            .map_with_span(|(first, rest), span| {
                let mut parts = vec![first];
                parts.extend(rest);
                let mut it = parts.into_iter().rev();
                let last = it.next().unwrap();
                it.fold(last, |acc, next| {
                    Expr::new(
                        ExprKind::Sigma("_".to_string(), Box::new(next), Box::new(acc)),
                        span.clone(),
                    )
                })
            });

        let arrow = sigma
            .clone()
            .then(just("->").padded().ignore_then(sigma.clone()).repeated())
            .map_with_span(|(first, rest), span| {
                let mut parts = vec![first];
                parts.extend(rest);
                let mut it = parts.into_iter().rev();
                let last = it.next().unwrap();
                it.fold(last, |acc, next| {
                    Expr::new(
                        ExprKind::Pi("_".to_string(), Box::new(next), Box::new(acc)),
                        span.clone(),
                    )
                })
            });

        arrow
            .clone()
            .then(just(':').padded().ignore_then(arrow).or_not())
            .map_with_span(|(e, ann), span| match ann {
                Some(ty) => Expr::new(ExprKind::Ann(Box::new(e), Box::new(ty)), span),
                None => e,
            })
    })
    .padded()
    .then_ignore(end())
}

pub fn parse(src: &str) -> Result<Expr, Vec<Simple<char>>> {
    parser().parse(src)
}
