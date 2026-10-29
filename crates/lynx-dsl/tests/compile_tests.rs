use lynx_dsl::{CompileError, EvalPlan, MatchProgram, Predicate, compile_match_expr};

#[test]
fn compiles_valid_story_examples() {
    for example in common::editor_story_fixtures::DSL_STORY_EXAMPLES {
        if example.label == "Comment only" {
            continue;
        }
        compile_match_expr(example.value)
            .unwrap_or_else(|error| panic!("failed to compile {:?}: {error}", example.label));
    }
}

#[test]
fn compiles_extra_valid_story_examples() {
    for example in common::editor_story_fixtures::DSL_STORY_EXTRA_VALID {
        compile_match_expr(example.value)
            .unwrap_or_else(|error| panic!("failed to compile {:?}: {error}", example.label));
    }
}

#[test]
fn rejects_invalid_syntax() {
    for example in common::editor_story_fixtures::DSL_STORY_INVALID {
        assert!(
            compile_match_expr(example.value).is_err(),
            "expected compile failure for {:?}",
            example.label
        );
    }
}

#[test]
fn rejects_empty_and_comment_only_documents() {
    assert_eq!(compile_match_expr("   ").unwrap_err(), CompileError::Empty);
    assert_eq!(
        compile_match_expr("# notes only").unwrap_err(),
        CompileError::Empty
    );
}

#[test]
fn compiles_host_and_path_and_into_all_plan() {
    let program = compile_match_expr("example.com AND /api").unwrap();
    assert_eq!(program.predicates.len(), 2);
    assert!(matches!(program.predicates[0], Predicate::HostEq(_)));
    assert!(matches!(program.predicates[1], Predicate::PathGlob(_)));
    assert!(matches!(program.plan, EvalPlan::All(_)));
}

#[test]
fn compiles_host_spaced_path_as_single_primary() {
    let program = compile_match_expr("example.com /api").unwrap();
    assert_eq!(program.predicates.len(), 2);
    assert!(matches!(program.predicates[0], Predicate::HostEq(_)));
    assert!(matches!(program.predicates[1], Predicate::PathGlob(_)));
}

#[test]
fn compiles_bare_keyword_to_fuzzy_predicate() {
    let program = compile_match_expr("apple").unwrap();
    assert_eq!(program.predicates.len(), 1);
    assert!(matches!(
        &program.predicates[0],
        Predicate::FuzzyContains(value) if value.as_ref() == "apple"
    ));
}

#[test]
fn compiles_not_or_grouping() {
    let program = compile_match_expr("NOT (example.com OR /api)").unwrap();
    assert!(matches!(program.plan, EvalPlan::Not(_)));
    if let EvalPlan::Not(inner) = program.plan {
        assert!(matches!(*inner, EvalPlan::Any(_)));
    } else {
        panic!("expected Not plan");
    }
}

#[test]
fn compiles_scheme_host_port_path() {
    let program = compile_match_expr("https://example.com:8080/status").unwrap();
    assert_eq!(program.predicates.len(), 4);
    assert!(matches!(program.predicates[0], Predicate::SchemeEq(_)));
    assert!(matches!(program.predicates[1], Predicate::HostEq(_)));
    assert!(matches!(program.predicates[2], Predicate::PortEq(8080)));
    assert!(matches!(program.predicates[3], Predicate::PathGlob(_)));
}

#[test]
fn compiles_bare_protocol_to_a_single_scheme_predicate() {
    let program = compile_match_expr("HTTPS://").unwrap();
    assert_eq!(program.predicates.len(), 1);
    assert!(matches!(
        &program.predicates[0],
        Predicate::SchemeEq(scheme) if scheme.as_ref() == "https"
    ));
    assert!(matches!(program.plan, EvalPlan::Pred(0)));
}

#[test]
fn compiles_cli_method_and_headers() {
    let program = compile_match_expr("example.com -X POST --header Authorization=Bearer").unwrap();
    assert!(
        program
            .predicates
            .iter()
            .any(|pred| matches!(pred, Predicate::HostEq(_)))
    );
    assert!(program.predicates.iter().any(|pred| matches!(
        pred,
        Predicate::MethodEq(method) if method.as_ref() == "POST"
    )));
    assert!(program.predicates.iter().any(|pred| matches!(
        pred,
        Predicate::HeaderEq { key, value }
            if key.as_ref() == "authorization" && value.as_ref() == "Bearer"
    )));
}

#[test]
fn compiles_top_level_or() {
    let program = compile_match_expr("example.com AND /api OR /health").unwrap();
    assert!(matches!(program.plan, EvalPlan::Any(_)));
}

fn predicate_kinds(program: &MatchProgram) -> Vec<&'static str> {
    program
        .predicates
        .iter()
        .map(|pred| match pred {
            Predicate::HostEq(_) => "host",
            Predicate::FuzzyContains(_) => "fuzzy",
            Predicate::SchemeEq(_) => "scheme",
            Predicate::PortEq(_) => "port",
            Predicate::PathGlob(_) => "path",
            Predicate::MethodEq(_) => "method",
            Predicate::QueryContains(_) => "query",
            Predicate::QueryParamsAll(_) => "query_params",
            Predicate::HeaderEq { .. } => "header",
        })
        .collect()
}

#[test]
fn predicate_inventory_for_complex_rule() {
    let program =
        compile_match_expr("(example.com OR /api/) AND NOT https://example.com/health").unwrap();
    let kinds = predicate_kinds(&program);
    assert!(kinds.contains(&"host"));
    assert!(kinds.contains(&"path"));
    assert!(kinds.contains(&"scheme"));
}

mod common;
