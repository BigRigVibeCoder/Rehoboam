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
use rh_registry::effect::value::{render_key, render_scalar};
use rh_registry::effect::{
    CompileOptions, Diagnostics, EXPRESSION_DEPTH_MAX, ErrorCode, EvalOptions, Form, ProgramKind,
    Ty, compile, compile_with,
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

/// Parentheses nest: 10^5 of them are `E_BOUND` (expression nesting above 64, §4.2).
#[test]
fn deep_parentheses_are_e_bound() {
    let g = golden();
    let n = 100_000;
    let parens = format!("{}1{}", "(".repeat(n), ")".repeat(n));
    assert_eq!(code(&g, Form::Expr, &parens), Some(ErrorCode::Bound));
    let calls = format!("{}1{}", "abs(".repeat(n), ")".repeat(n));
    assert_eq!(code(&g, Form::Expr, &calls), Some(ErrorCode::Bound));
    let at_limit = format!(
        "{}1{}",
        "(".repeat(EXPRESSION_DEPTH_MAX - 1),
        ")".repeat(EXPRESSION_DEPTH_MAX - 1)
    );
    assert_eq!(code(&g, Form::Expr, &at_limit), None);
    let above = format!(
        "{}1{}",
        "(".repeat(EXPRESSION_DEPTH_MAX),
        ")".repeat(EXPRESSION_DEPTH_MAX)
    );
    assert_eq!(code(&g, Form::Expr, &above), Some(ErrorCode::Bound));
}

fn eval_text(g: &Golden, text: &str, kind: &str) -> String {
    let world = g
        .world(&g.env, "dynasty", rh_registry::effect::Mutant::None)
        .unwrap();
    let host = world.host(&g.env);
    let person = g.env.kind("person").unwrap();
    let opts = CompileOptions {
        scope: vec![("now".into(), Ty::Int), ("x".into(), Ty::Ent(person))],
        check: rh_registry::effect::CheckOptions {
            allow_key: kind == "order",
            ..Default::default()
        },
        ..CompileOptions::default()
    };
    let program = match compile_with(Form::Expr, text, &g.env, "M7.dynasty", &opts) {
        Ok(p) => p,
        Err(r) => return format!("error:{}", r.code),
    };
    let values = [
        rh_registry::effect::Value::Int(0),
        rh_registry::effect::Value::Ent(person, 5),
    ];
    match program
        .eval(
            &g.env,
            &host,
            &values,
            &mut Diagnostics::default(),
            EvalOptions::default(),
        )
        .unwrap()
    {
        rh_registry::effect::Output::Value(v) if kind == "order" => {
            format!("key:{}", render_key(&g.env, &v))
        }
        rh_registry::effect::Output::Value(v) => render_scalar(&g.env, &v),
        other => format!("{other:?}"),
    }
}

/// GAP-A: operator chains and prefix operators do not nest. A 10^5-term chain is a valid program
/// and yields the reference's value (`effect_ref.py`, the same texts), in debug, on a thread with
/// a 2 MiB stack.
#[test]
fn long_chains_yield_the_references_values_on_two_mebibytes() {
    let n = 100_000;
    let cases: Vec<(String, &str, &str)> = vec![
        (vec!["1"; n].join(" + "), "expr", "int:100000"),
        (vec!["true"; n].join(" and "), "expr", "bool:true"),
        (format!("{}1", "- ".repeat(n)), "expr", "int:1"),
        (format!("{}1", "-".repeat(n - 1)), "expr", "int:-1"),
        (format!("{}true", "not ".repeat(n)), "expr", "bool:true"),
        (vec!["now"; n].join(" + "), "expr", "int:0"),
        (vec!["q16(1)"; n].join(" * "), "expr", "q16_16:65536"),
        (vec!["false"; n].join(" or "), "expr", "bool:false"),
        (
            format!("draw(sortition, x, {})", vec!["now"; n].join(" + ")),
            "expr",
            "int:4500000000000000000",
        ),
        (
            format!("({}, 2)", vec!["1"; n].join(" * ")),
            "order",
            "key:(int:1, int:2)",
        ),
        (
            format!("(1 + 2) + {}", vec!["1"; n].join(" + ")),
            "expr",
            "int:100003",
        ),
    ];
    let run = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let g = golden();
            cases
                .iter()
                .map(|(text, kind, want)| (eval_text(&g, text, kind), (*want).to_owned()))
                .collect::<Vec<_>>()
        })
        .unwrap()
        .join()
        .expect("a long chain overflowed a 2 MiB stack");
    for (i, (got, want)) in run.iter().enumerate() {
        assert_eq!(got, want, "chain case {i}");
    }
}

/// The deepest nesting the parser accepts (63 levels of the `expr` production) compiles on a
/// 2 MiB stack (a default thread's) and evaluates on 1 MiB in a debug build.
#[test]
fn the_deepest_accepted_programs_fit_the_stack() {
    let deep = EXPRESSION_DEPTH_MAX - 1;
    let texts = [
        format!("{}1{}", "abs(".repeat(deep), ")".repeat(deep)),
        format!("{}1{}", "(".repeat(deep), ")".repeat(deep)),
        format!(
            "{}1{}",
            "if true then ".repeat(deep / 2),
            " else 2".repeat(deep / 2)
        ),
        format!("{}1{}", "(- 1 + ".repeat(deep), ")".repeat(deep)),
    ];
    let compiled = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let g = golden();
            let opts = CompileOptions {
                scope: vec![("now".into(), Ty::Int)],
                ..CompileOptions::default()
            };
            texts
                .iter()
                .map(|t| compile_with(Form::Expr, t, &g.env, "M7.dynasty", &opts).unwrap())
                .collect::<Vec<_>>()
        })
        .unwrap()
        .join()
        .expect("compiling the deepest programs overflowed a 2 MiB stack");
    std::thread::Builder::new()
        .stack_size(1 << 20)
        .spawn(move || {
            let g = golden();
            let world = g
                .world(&g.env, "dynasty", rh_registry::effect::Mutant::None)
                .unwrap();
            let host = world.host(&g.env);
            let zero = [rh_registry::effect::Value::Int(0)];
            for p in &compiled {
                p.eval(
                    &g.env,
                    &host,
                    &zero,
                    &mut Diagnostics::default(),
                    EvalOptions::default(),
                )
                .unwrap();
            }
        })
        .unwrap()
        .join()
        .expect("evaluating the deepest programs overflowed a 1 MiB stack");
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
