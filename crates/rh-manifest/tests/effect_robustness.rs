//! The effect language's front end on hostile input (SPR-017 T-01, T-04): deep nesting is refused
//! with `E_BOUND` without overflowing the stack, no input panics, the tallest accepted tree
//! compiles and evaluates, literals compile exactly, and `derive` keeps its reserved signature.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    reason = "test helpers fail loudly"
)]

use rh_manifest::effect_cases::{Golden, splitmix64};
use rh_registry::effect::{
    CompileOptions, Diagnostics, EXPRESSION_DEPTH_MAX, ErrorCode, EvalOptions, Form, ProgramKind,
    TREE_HEIGHT_MAX, Ty, UNARY_CHAIN_MAX, compile, compile_with,
};

fn golden() -> Golden {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../specs/registry/effect_language_golden_v0.toml"
    );
    Golden::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn code(g: &Golden, form: Form, text: &str) -> Option<ErrorCode> {
    let opts = CompileOptions {
        scope: vec![("now".into(), Ty::Int)],
        ..CompileOptions::default()
    };
    compile_with(form, text, &g.env, "M7.dynasty", &opts)
        .err()
        .map(|r| r.code)
}

#[test]
fn deep_nesting_is_e_bound_without_overflowing_the_stack() {
    let g = golden();
    let n = 100_000;
    let parens = format!("{}1{}", "(".repeat(n), ")".repeat(n));
    assert_eq!(code(&g, Form::Expr, &parens), Some(ErrorCode::Bound));
    assert_eq!(
        code(&g, Form::Expr, &format!("{}true", "not ".repeat(n))),
        Some(ErrorCode::Bound)
    );
    assert_eq!(
        code(&g, Form::Expr, &format!("{}1", "- ".repeat(n))),
        Some(ErrorCode::Bound)
    );
    let chain = vec!["1"; n].join(" + ");
    assert_eq!(code(&g, Form::Expr, &chain), Some(ErrorCode::Bound));
    let logic = vec!["true"; n].join(" and ");
    assert_eq!(code(&g, Form::Expr, &logic), Some(ErrorCode::Bound));
    let effects = vec!["Promote(person: now)"; n].join("; ");
    assert!(code(&g, Form::Effects, &effects).is_some());
}

/// The deepest programs the parser accepts compile on a 2 MiB stack (a default thread's) and
/// evaluate on 1 MiB in a debug build: compilation runs at Registry load, evaluation on the
/// engine's workers, which keep room for their own frames.
#[test]
fn the_deepest_accepted_programs_fit_the_stack() {
    let terms = usize::try_from(TREE_HEIGHT_MAX).unwrap() - 1;
    let deep = EXPRESSION_DEPTH_MAX - 1;
    let texts = [
        vec!["1"; terms].join(" + "),
        vec!["true"; terms].join(" and "),
        format!("{}true", "not ".repeat(UNARY_CHAIN_MAX)),
        format!("{}1{}", "abs(".repeat(deep), ")".repeat(deep)),
        format!("{}1{}", "(".repeat(deep), ")".repeat(deep)),
        format!(
            "{}1{}",
            "if true then ".repeat(deep / 2),
            " else 2".repeat(deep / 2)
        ),
    ];
    let compiled = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let g = golden();
            let opts = CompileOptions {
                scope: vec![("now".into(), Ty::Int)],
                ..CompileOptions::default()
            };
            let programs: Vec<_> = texts
                .iter()
                .map(|t| compile_with(Form::Expr, t, &g.env, "M7.dynasty", &opts).unwrap())
                .collect();
            let taller = vec!["1"; terms + 2].join(" + ");
            assert_eq!(code(&g, Form::Expr, &taller), Some(ErrorCode::Bound));
            let too_many = format!("{}true", "not ".repeat(UNARY_CHAIN_MAX + 1));
            assert_eq!(code(&g, Form::Expr, &too_many), Some(ErrorCode::Bound));
            programs
        })
        .unwrap()
        .join()
        .expect("compiling the deepest programs overflowed a 2 MiB stack");
    let evaluated = std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || {
            let g = golden();
            let world = g
                .world(&g.env, "dynasty", rh_registry::effect::Mutant::None)
                .unwrap();
            let host = world.host(&g.env);
            let zero = [rh_registry::effect::Value::Int(0)];
            let outs: Vec<_> = compiled
                .iter()
                .map(|p| {
                    p.eval(
                        &g.env,
                        &host,
                        &zero,
                        &mut Diagnostics::default(),
                        EvalOptions::default(),
                    )
                    .unwrap()
                })
                .collect();
            drop(compiled);
            outs
        })
        .unwrap()
        .join()
        .expect("evaluating the deepest programs overflowed a 1 MiB stack");
    let want = rh_registry::effect::Value::Int(i64::try_from(terms).unwrap());
    assert_eq!(evaluated[0], rh_registry::effect::Output::Value(want));
}

#[test]
fn no_input_panics() {
    let g = golden();
    let mut texts: Vec<String> = vec![
        String::new(),
        " ".into(),
        "#".into(),
        "\u{e9}".into(),
        "\u{a0}".into(),
        "\u{0}".into(),
    ];
    for law in g.laws.values() {
        for program in law {
            for (i, _) in program.char_indices() {
                texts.push(program[..i].to_owned());
                texts.push(program[i..].to_owned());
            }
        }
    }
    for case in &g.cases {
        let text = case["text"].as_str().unwrap();
        for (i, _) in text.char_indices().step_by(3) {
            texts.push(text[..i].to_owned());
        }
    }
    // Random programs over the language's alphabet, eight seeds.
    let alphabet: Vec<&str> = vec![
        "x",
        "holder",
        "title",
        "now",
        "(",
        ")",
        "[",
        "]",
        ",",
        ":",
        ";",
        ".",
        "+",
        "-",
        "*",
        "/",
        "==",
        "<",
        ">=",
        "and",
        "or",
        "not",
        "if",
        "then",
        "else",
        "for",
        "in",
        "where",
        "when",
        "none",
        "true",
        "1",
        "0.5",
        "1e3",
        "count",
        "min",
        "lineage",
        "children",
        "person",
        "alive",
        "life_course",
        "sex",
        "draw",
        "sortition",
        "params",
        "majority_age",
        "calendar",
        "div",
        "mod",
        "\u{e9}",
        "\u{3000}",
        "Resign",
        "office",
        "person:",
    ];
    for seed in 1..=8_u64 {
        let mut state = seed;
        for _ in 0..1_500 {
            state = splitmix64(state);
            let len = usize::try_from(state % 24).unwrap();
            let mut text = String::new();
            for _ in 0..len {
                state = splitmix64(state);
                text.push_str(alphabet[usize::try_from(state % alphabet.len() as u64).unwrap()]);
                text.push(' ');
            }
            texts.push(text);
        }
    }
    let world = g
        .world(&g.env, "dynasty", rh_registry::effect::Mutant::None)
        .unwrap();
    let host = world.host(&g.env);
    for text in &texts {
        for kind in [
            ProgramKind::Candidates,
            ProgramKind::Eligible,
            ProgramKind::Order,
            ProgramKind::Effects,
            ProgramKind::Fulfilled,
        ] {
            let system = if kind == ProgramKind::Effects {
                "T1.minds"
            } else {
                "M7.dynasty"
            };
            if let Ok(program) = compile(kind, text, &g.env, system) {
                let bindings: Vec<_> = program
                    .scope()
                    .iter()
                    .map(|(_, ty)| match ty {
                        Ty::Ent(k) => rh_registry::effect::Value::Ent(*k, 7),
                        Ty::Event => {
                            rh_registry::effect::Value::Event(rh_registry::effect::EventArg::Own)
                        }
                        _ => rh_registry::effect::Value::Int(0),
                    })
                    .collect();
                let _ = program.eval(
                    &g.env,
                    &host,
                    &bindings,
                    &mut Diagnostics::default(),
                    EvalOptions::default(),
                );
            }
        }
    }
}

#[test]
fn literals_compile_exactly_half_to_even() {
    let g = golden();
    let opts = |ty: Ty| CompileOptions {
        scope: vec![("now".into(), Ty::Int)],
        expected: Some(ty),
        ..CompileOptions::default()
    };
    let world = g
        .world(&g.env, "dynasty", rh_registry::effect::Mutant::None)
        .unwrap();
    let host = world.host(&g.env);
    for seed in 1..=8_u64 {
        let mut state = seed.wrapping_mul(0x51);
        for _ in 0..400 {
            state = splitmix64(state);
            let whole = state % 200;
            state = splitmix64(state);
            let digits = usize::try_from(1 + state % 12).unwrap();
            state = splitmix64(state);
            let frac = state % 10_u64.pow(u32::try_from(digits).unwrap());
            let text = format!("{whole}.{frac:0digits$}");
            for (ty, f, max) in [
                (Ty::Q8, 8_u32, i128::from(i16::MAX)),
                (Ty::Q16, 16, i128::from(i32::MAX)),
            ] {
                // The exact rational, scaled and rounded half to even, independently of the lexer.
                let den = 10_i128.pow(u32::try_from(digits).unwrap());
                let num = (i128::from(whole) * den + i128::from(frac)) << f;
                let (q, r) = (num / den, num % den);
                let bits = if 2 * r > den || (2 * r == den && q & 1 == 1) {
                    q + 1
                } else {
                    q
                };
                let got = compile_with(Form::Expr, &text, &g.env, "M7.dynasty", &opts(ty.clone()));
                if bits > max {
                    assert_eq!(
                        got.err().map(|r| r.code),
                        Some(ErrorCode::LiteralRange),
                        "{text}"
                    );
                    continue;
                }
                let out = got
                    .unwrap()
                    .eval(
                        &g.env,
                        &host,
                        &[rh_registry::effect::Value::Int(0)],
                        &mut Diagnostics::default(),
                        EvalOptions::default(),
                    )
                    .unwrap();
                let rendered = format!("{out:?}");
                assert!(
                    rendered.contains(&format!("({bits})")),
                    "{text} as {ty:?}: {rendered}, want {bits}"
                );
            }
        }
    }
}

#[test]
fn derive_is_reserved_with_its_signature() {
    let g = golden();
    let program = compile(ProgramKind::Derive, "0.5", &g.env, "T1.minds").unwrap();
    assert_eq!(program.ty(), &Ty::Q8);
    assert_eq!(
        program
            .scope()
            .iter()
            .map(|(n, _)| n.as_str())
            .collect::<Vec<_>>(),
        ["self", "now"]
    );
    assert_eq!(
        compile(ProgramKind::Derive, "true", &g.env, "T1.minds")
            .err()
            .map(|r| r.code),
        Some(ErrorCode::Type)
    );
}
