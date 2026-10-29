use lynx_dsl::{format_dsl, has_parse_errors, parse_program, parse_program_partial};

fn node_kinds(source: &str, kind: &str) -> Vec<String> {
    let validation = lynx_dsl::validate(source);
    validation
        .highlights
        .into_iter()
        .filter(|span| span.kind == kind)
        .map(|span| source[span.from..span.to].to_string())
        .collect()
}

#[test]
fn parses_host_only_url() {
    assert!(!has_parse_errors("example.com"));
    assert_eq!(node_kinds("example.com", "Host"), vec!["example.com"]);
}

#[test]
fn highlights_bare_keyword_as_fuzzy_keyword() {
    let input = "apple";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "FuzzyKeyword"), vec![input]);
    assert!(node_kinds(input, "Host").is_empty());
}

#[test]
fn parses_hyphenated_domain() {
    let input = "api.example-ex.com";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "Host"), vec![input]);
}

#[test]
fn parses_single_host_label_with_hyphens() {
    let input = "example-ex";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "Host"), vec![input]);
}

#[test]
fn parses_host_with_port_and_path() {
    let input = "example.com:5678/a";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "Host"), vec!["example.com"]);
    assert_eq!(node_kinds(input, "Port"), vec![":5678"]);
    assert_eq!(node_kinds(input, "Path"), vec!["/a"]);
}

#[test]
fn parses_scheme_url() {
    let input = "https://example.com:5678/a";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "Scheme"), vec!["https://"]);
    assert_eq!(node_kinds(input, "Host"), vec!["example.com"]);
    assert_eq!(node_kinds(input, "Port"), vec![":5678"]);
    assert_eq!(node_kinds(input, "Path"), vec!["/a"]);
}

#[test]
fn parses_url_with_query() {
    let input = "example.com/api?foo=bar&x=1";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "Host"), vec!["example.com"]);
    assert_eq!(node_kinds(input, "Path"), vec!["/api"]);
    assert_eq!(node_kinds(input, "Query"), vec!["?foo=bar&x=1"]);
}

#[test]
fn parses_query_only_operand() {
    let input = "?user_id=123&fields=id,name";
    assert!(!has_parse_errors(input));
    assert_eq!(
        node_kinds(input, "Query"),
        vec!["?user_id=123&fields=id,name"]
    );
}

#[test]
fn parses_supported_schemes() {
    for scheme in ["http://", "https://", "ws://", "wss://"] {
        let input = format!("{scheme}example.com");
        assert!(!has_parse_errors(&input), "failed for {input}");
        assert_eq!(node_kinds(&input, "Scheme"), vec![scheme]);
    }
}

#[test]
fn parses_bare_protocol_matchers() {
    for scheme in ["http://", "https://", "ws://", "wss://"] {
        assert!(!has_parse_errors(scheme), "failed for {scheme}");
        assert_eq!(node_kinds(scheme, "Scheme"), vec![scheme]);
        assert!(node_kinds(scheme, "Host").is_empty());
    }
}

#[test]
fn formats_bare_protocol_in_boolean_expression() {
    assert_eq!(
        format_dsl("https:// or wss://").unwrap(),
        Some("https://\nOR wss://".to_string())
    );
}

#[test]
fn parses_path_only_operands() {
    assert_eq!(node_kinds("/a", "Path"), vec!["/a"]);
    assert_eq!(node_kinds("/a*", "Path"), vec!["/a*"]);
    assert_eq!(node_kinds("/a/", "Path"), vec!["/a/"]);
}

#[test]
fn parses_boolean_operators() {
    let input = "example.com AND /a OR NOT https://example.com/";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "AndOp"), vec!["AND"]);
    assert_eq!(node_kinds(input, "OrOp"), vec!["OR"]);
    assert_eq!(node_kinds(input, "NotOp"), vec!["NOT"]);
}

#[test]
fn does_not_treat_whitespace_as_and() {
    let input = "example.com /api";
    assert!(!has_parse_errors(input));
    assert!(node_kinds(input, "AndOp").is_empty());
    assert_eq!(node_kinds(input, "Host"), vec!["example.com"]);
    assert_eq!(node_kinds(input, "Path"), vec!["/api"]);
}

#[test]
fn parses_cli_flags() {
    let input = "example.com -h x-token=b --header foo=bar --header-include xxx";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "ShortFlag"), vec!["-h"]);
    assert_eq!(
        node_kinds(input, "LongFlag"),
        vec!["--header", "--header-include"]
    );
    assert_eq!(
        node_kinds(input, "CliValue"),
        vec!["x-token=b", "foo=bar", "xxx"]
    );
}

#[test]
fn parses_cli_x_post() {
    let input = "NOT */rest/* AND -X POST";
    assert!(!has_parse_errors(input));
    assert_eq!(node_kinds(input, "ShortFlag"), vec!["-X"]);
    assert_eq!(node_kinds(input, "CliValue"), vec!["POST"]);
    assert_eq!(node_kinds(input, "Path"), vec!["*/rest/*"]);
}

#[test]
fn parses_glob_paths() {
    for input in ["/api/*/v1", "/api/**/track", "/**/a", "*/a", "**/a"] {
        assert!(!has_parse_errors(input), "failed for {input}");
    }
    assert_eq!(node_kinds("*/a", "Path"), vec!["*/a"]);
    assert_eq!(node_kinds("/**/a", "Path"), vec!["/**/a"]);
}

#[test]
fn ignores_line_comments() {
    assert!(!has_parse_errors("# host"));
    assert!(!has_parse_errors("# host\nexample.com"));
    assert!(!has_parse_errors("example.com AND /api # trailing"));
}

#[test]
fn rejects_invalid_syntax() {
    assert!(has_parse_errors("example.com AND ("));
}

#[test]
fn partial_parse_keeps_valid_prefix_on_error() {
    let outcome = parse_program_partial("example.com AND (");
    assert!(outcome.program.expr.is_some());
    assert!(outcome.error.is_some());
    assert!(has_parse_errors("example.com AND ("));
}

#[test]
fn format_and_spacing() {
    assert_eq!(
        format_dsl("example.com   and   /api").unwrap(),
        Some("example.com AND /api".to_string())
    );
}

#[test]
fn format_top_level_or() {
    assert_eq!(
        format_dsl("example.com and /api OR https://example.com/health").unwrap(),
        Some("example.com AND /api\nOR https://example.com/health".to_string())
    );
}

#[test]
fn comment_only_document() {
    let input = "# notes only";
    assert!(!has_parse_errors(input));
    assert_eq!(format_dsl(input).unwrap(), Some(input.to_string()));
}

#[test]
fn parse_program_empty() {
    let program = parse_program("   ").unwrap();
    assert!(program.expr.is_none());
}

#[test]
fn highlights_or_inside_grouped_expression() {
    let input = "(\n  example.com\n  OR /api/\n)\nAND NOT https://example.com/health";
    assert!(!has_parse_errors(input), "parse failed");
    assert_eq!(
        node_kinds(input, "OrOp"),
        vec!["OR"],
        "expected inner OR to be highlighted"
    );
}

#[test]
fn highlights_full_host_inside_grouped_multiline() {
    let input = "(\n  example.com\n  OR /api/\n)\nAND NOT https://example.com/health";
    let hosts = node_kinds(input, "Host");
    assert!(
        hosts.iter().any(|host| host == "example.com"),
        "expected grouped host highlight to cover full hostname, got {hosts:?}"
    );
}
