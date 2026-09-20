use lynx_dsl::{RequestFacts, compile_match_expr, eval_program};

fn assert_matches(dsl: &str, facts: lynx_dsl::RequestFactsBuilder, expected: bool) {
    let program =
        compile_match_expr(dsl).unwrap_or_else(|error| panic!("compile {dsl:?}: {error}"));
    let facts = facts.build();
    assert_eq!(eval_program(&program, &facts), expected, "dsl={dsl:?}");
}

#[test]
fn host_only_matches() {
    assert_matches(
        "example.com",
        RequestFacts::builder().host("example.com"),
        true,
    );
    assert_matches(
        "example.com",
        RequestFacts::builder().host("api.example.com"),
        true,
    );
    assert_matches(
        "example.com",
        RequestFacts::builder().host("other.com"),
        false,
    );
    assert_matches(
        "example.com",
        RequestFacts::builder().host("notexample.com"),
        false,
    );
}

#[test]
fn hyphenated_host_label_matches_subdomain() {
    assert_matches(
        "example-ex",
        RequestFacts::builder().host("api.example-ex.com"),
        true,
    );
    assert_matches(
        "api.example-ex.com",
        RequestFacts::builder().host("api.example-ex.com"),
        true,
    );
    assert_matches(
        "example-ex",
        RequestFacts::builder().host("other.com"),
        false,
    );
}

#[test]
fn host_is_case_insensitive() {
    assert_matches(
        "Example.COM",
        RequestFacts::builder().host("example.com"),
        true,
    );
}

#[test]
fn host_spaced_path_matches() {
    assert_matches(
        "example.com /api",
        RequestFacts::builder()
            .host("example.com")
            .path("/api/users"),
        true,
    );
    assert_matches(
        "example.com /api",
        RequestFacts::builder()
            .host("example.com")
            .path("/web/users"),
        false,
    );
}

#[test]
fn scheme_host_port_path_matches() {
    assert_matches(
        "https://example.com:8080/status",
        RequestFacts::builder()
            .scheme("https")
            .host("example.com")
            .port(8080)
            .path("/status"),
        true,
    );
    assert_matches(
        "https://example.com:8080/status",
        RequestFacts::builder()
            .scheme("http")
            .host("example.com")
            .port(8080)
            .path("/status"),
        false,
    );
}

#[test]
fn bare_protocol_matches_only_requests_with_that_protocol() {
    assert_matches(
        "https://",
        RequestFacts::builder()
            .scheme("https")
            .host("any.example.com")
            .path("/anything"),
        true,
    );
    assert_matches(
        "https://",
        RequestFacts::builder()
            .scheme("http")
            .host("any.example.com")
            .path("/anything"),
        false,
    );
    assert_matches(
        "wss:// OR https://",
        RequestFacts::builder()
            .scheme("wss")
            .host("socket.example.com")
            .path("/events"),
        true,
    );
}

#[test]
fn path_glob_single_segment_wildcard() {
    assert_matches(
        "/api/*/v1",
        RequestFacts::builder().path("/api/users/v1"),
        true,
    );
    assert_matches(
        "/api/*/v1",
        RequestFacts::builder().path("/api/users/extra/v1"),
        false,
    );
}

#[test]
fn path_glob_multi_segment_wildcard() {
    assert_matches(
        "/api/**/track",
        RequestFacts::builder().path("/api/v1/events/track"),
        true,
    );
    assert_matches(
        "/**/admin",
        RequestFacts::builder().path("/tenant/admin"),
        true,
    );
}

#[test]
fn path_glob_without_leading_slash() {
    assert_matches(
        "*/rest/*",
        RequestFacts::builder().path("/foo/rest/bar"),
        true,
    );
}

#[test]
fn boolean_and_or_precedence() {
    assert_matches(
        "example.com AND /api OR /health",
        RequestFacts::builder()
            .host("example.com")
            .path("/web")
            .method("GET"),
        false,
    );
    assert_matches(
        "example.com AND /api OR /health",
        RequestFacts::builder().host("other.com").path("/health"),
        true,
    );
}

#[test]
fn not_expression() {
    assert_matches("NOT /health", RequestFacts::builder().path("/api"), true);
    assert_matches(
        "NOT /health",
        RequestFacts::builder().path("/health"),
        false,
    );
}

#[test]
fn grouped_expression() {
    assert_matches(
        "(example.com OR localhost) AND /api",
        RequestFacts::builder().host("localhost").path("/api/v1"),
        true,
    );
    assert_matches(
        "(example.com OR localhost) AND /api",
        RequestFacts::builder().host("other.com").path("/api/v1"),
        false,
    );
}

#[test]
fn cli_method_flag() {
    assert_matches("-X POST", RequestFacts::builder().method("POST"), true);
    assert_matches(
        "example.com -X GET",
        RequestFacts::builder().host("example.com").method("GET"),
        true,
    );
    assert_matches(
        "example.com - X POST",
        RequestFacts::builder().host("example.com").method("POST"),
        true,
    );
}

#[test]
fn cli_header_flags() {
    assert_matches(
        "--header Authorization=Bearer",
        RequestFacts::builder().header("authorization", "Bearer"),
        true,
    );
    assert_matches(
        "example.com -h x-token=abc",
        RequestFacts::builder()
            .host("example.com")
            .header("x-token", "abc"),
        true,
    );
    assert_matches(
        "example.com -h foo=bar --header baz=qux",
        RequestFacts::builder()
            .host("example.com")
            .header("foo", "bar")
            .header("baz", "qux"),
        true,
    );
}

#[test]
fn query_flag_matches_on_facts_builder() {
    assert_matches(
        "example.com AND /api AND --query foo=bar",
        RequestFacts::builder()
            .host("example.com")
            .path("/api")
            .query("foo=bar&x=1"),
        true,
    );
    assert_matches(
        "example.com AND /api AND -q x=2",
        RequestFacts::builder()
            .host("example.com")
            .path("/api")
            .query("foo=bar&x=1"),
        false,
    );
}

#[test]
fn ws_scheme_matches() {
    assert_matches(
        "ws://example.com:8080/status",
        RequestFacts::builder()
            .scheme("ws")
            .host("example.com")
            .port(8080)
            .path("/status"),
        true,
    );
}

#[test]
fn complex_grouped_story_example() {
    assert_matches(
        "(example.com OR /api/) AND NOT https://example.com/health",
        RequestFacts::builder()
            .host("example.com")
            .path("/api/users"),
        true,
    );
    assert_matches(
        "(example.com OR /api/) AND NOT https://example.com/health",
        RequestFacts::builder()
            .scheme("https")
            .host("example.com")
            .path("/health"),
        false,
    );
}

#[test]
fn cli_only_post_with_glob_path() {
    assert_matches(
        "NOT */rest/* AND -X POST",
        RequestFacts::builder().method("POST").path("/api/users"),
        true,
    );
    assert_matches(
        "NOT */rest/* AND -X POST",
        RequestFacts::builder().method("POST").path("/foo/rest/bar"),
        false,
    );
}

#[test]
fn url_query_params_subset_match() {
    assert_matches(
        "example.com/v1/graphql?operationName=GetFeed&platform=android",
        RequestFacts::builder()
            .host("example.com")
            .path("/v1/graphql")
            .query("operationName=GetFeed&platform=android&sign=abc"),
        true,
    );
    assert_matches(
        "example.com/v1/graphql?operationName=GetFeed&platform=android",
        RequestFacts::builder()
            .host("example.com")
            .path("/v1/graphql")
            .query("operationName=GetFeed"),
        false,
    );
}

#[test]
fn path_with_query_params() {
    assert_matches(
        "/v2/feed/timeline?page_size=20&region=SG",
        RequestFacts::builder()
            .path("/v2/feed/timeline")
            .query("page_size=20&region=SG&cursor=YXJy"),
        true,
    );
    assert_matches(
        "/v2/feed/timeline?page_size=20&region=SG",
        RequestFacts::builder()
            .path("/v2/feed/timeline")
            .query("page_size=10&region=SG"),
        false,
    );
}

#[test]
fn query_only_primary() {
    assert_matches(
        "?user_id=123456&fields=id,name,avatar",
        RequestFacts::builder().query("user_id=123456&fields=id,name,avatar&extra=1"),
        true,
    );
    assert_matches(
        "?user_id=123456",
        RequestFacts::builder().query("user_id=999"),
        false,
    );
}

#[test]
fn full_url_with_query_params() {
    let dsl = "https://example.com/v1/content/recommend?feed_type=for_you&limit=30";
    assert_matches(
        dsl,
        RequestFacts::builder()
            .scheme("https")
            .host("example.com")
            .path("/v1/content/recommend")
            .query("feed_type=for_you&limit=30&offset=0"),
        true,
    );
    assert_matches(
        dsl,
        RequestFacts::builder()
            .scheme("https")
            .host("example.com")
            .path("/v1/content/recommend")
            .query("feed_type=for_you&limit=30&cold_start=true"),
        true,
    );
}

// eval.rs and ir.rs must not depend on ast — compile is the sole AST entry point.
#[test]
fn eval_module_isolated_from_ast() {
    let source = include_str!("../src/eval.rs");
    assert!(
        !source.contains("use crate::ast"),
        "eval.rs should not import ast"
    );
    let ir_source = include_str!("../src/ir.rs");
    assert!(
        !ir_source.contains("use crate::ast"),
        "ir.rs should not import ast"
    );
}
