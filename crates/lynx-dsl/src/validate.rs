use serde::{Deserialize, Serialize};

use crate::format::validate_dsl_document;
use crate::highlight::{HighlightSpan, collect_highlights};
use crate::parser::parse_program;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub from: usize,
    pub to: usize,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub highlights: Vec<HighlightSpan>,
}

pub fn validate(source: &str) -> ValidationResult {
    if source.trim().is_empty() {
        return ValidationResult {
            is_valid: false,
            diagnostics: vec![Diagnostic {
                from: 0,
                to: source.len().max(1),
                severity: "error".to_string(),
                message: "Empty document".to_string(),
            }],
            highlights: Vec::new(),
        };
    }

    match parse_program(source) {
        Ok(program) => {
            let highlights = collect_highlights(source, &program);
            ValidationResult {
                is_valid: true,
                diagnostics: Vec::new(),
                highlights,
            }
        }
        Err(error) => {
            let (from, to, message) = diagnostic_from_parse_error(&error, source);
            ValidationResult {
                is_valid: false,
                diagnostics: vec![Diagnostic {
                    from,
                    to,
                    severity: "error".to_string(),
                    message,
                }],
                highlights: collect_comment_highlights_only(source),
            }
        }
    }
}

pub fn collect_syntax_diagnostics(source: &str) -> Vec<Diagnostic> {
    validate(source).diagnostics
}

pub fn validation_for_editor(source: &str) -> ValidationResult {
    let format_result = validate_dsl_document(source);
    if !format_result.is_valid {
        let diagnostics = if source.trim().is_empty() {
            vec![]
        } else {
            collect_syntax_diagnostics(source)
        };
        return ValidationResult {
            is_valid: false,
            diagnostics,
            highlights: collect_comment_highlights_only(source),
        };
    }

    validate(source)
}

fn diagnostic_from_parse_error(
    error: &crate::error::ParseError,
    source: &str,
) -> (usize, usize, String) {
    match error {
        crate::error::ParseError::Syntax { span, message: _ } => {
            let from = span.start.min(source.len());
            let to = span.end.max(from + 1).min(source.len());
            let snippet = source.get(from..to).unwrap_or("").trim();
            let message = if snippet.is_empty() {
                "Syntax error".to_string()
            } else {
                format!("Syntax error: {snippet}")
            };
            (from, to, message)
        }
        crate::error::ParseError::TrailingInput { span } => {
            let from = span.start.min(source.len());
            let to = span.end.max(from + 1).min(source.len());
            (from, to, "Syntax error".to_string())
        }
    }
}

fn collect_comment_highlights_only(source: &str) -> Vec<HighlightSpan> {
    parse_program(source)
        .ok()
        .map(|program| collect_highlights(source, &program))
        .unwrap_or_else(|| {
            let mut spans = Vec::new();
            let mut offset = 0usize;
            for line in source.split_inclusive('\n') {
                if let Some(hash_index) = line.find('#') {
                    let from = offset + hash_index;
                    let to = offset + line.trim_end_matches('\n').len();
                    if to > from {
                        spans.push(HighlightSpan {
                            from,
                            to,
                            kind: "LineComment".to_string(),
                        });
                    }
                }
                offset += line.len();
            }
            spans
        })
}
