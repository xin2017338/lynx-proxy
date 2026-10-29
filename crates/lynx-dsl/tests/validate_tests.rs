use lynx_dsl::validate;

#[test]
fn validate_invalid_input_does_not_panic() {
    let input = "https://example.com/rest/abc?a=123 AND (";
    let result = validate(input);
    assert!(!result.is_valid);
    assert!(!result.diagnostics.is_empty());
}
