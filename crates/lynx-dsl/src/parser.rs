use pest::Parser;
use pest_derive::Parser;

use crate::ast::{CliArg, CliArgValue, CliArgs, Primary, Program, Span, Spanned, Url};
use crate::error::ParseError;
use crate::error::ParseErrorInfo;
use crate::expr_parser::parse_expression;
use crate::expr_parser::parse_expression_partial;
use crate::expr_parser::parse_grouped_expression;
use crate::span::span_from_pair;

#[derive(Parser)]
#[grammar = "dsl.pest"]
pub struct DslParser;

pub fn prepare_source(source: &str) -> String {
    mask_line_comments(source)
}

pub fn mask_line_comments(source: &str) -> String {
    let mut masked = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '#' {
            masked.push(' ');
            for next in chars.by_ref() {
                if next == '\n' {
                    masked.push('\n');
                    break;
                }
                masked.push(' ');
            }
        } else {
            masked.push(ch);
        }
    }
    masked
}

pub fn parse_program(source: &str) -> Result<Program, ParseError> {
    let prepared = prepare_source(source);
    let expr = parse_expression(&prepared, 0)?;
    Ok(Program { expr })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ParseProgramOutcome {
    pub program: Program,
    pub error: Option<ParseErrorInfo>,
}

/// Parse for inspection UIs: returns a partial AST when possible, plus the first error.
pub fn parse_program_partial(source: &str) -> ParseProgramOutcome {
    let prepared = prepare_source(source);
    let (expr, error) = parse_expression_partial(&prepared, 0);
    ParseProgramOutcome {
        program: Program { expr },
        error: error.map(ParseErrorInfo::from),
    }
}

pub fn parse_primary_fragment(source: &str, base_offset: usize) -> Result<Primary, ParseError> {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Syntax {
            span: Span::new(base_offset, base_offset + 1),
            message: "empty primary".to_string(),
        });
    }

    let local_start = source.find(trimmed).unwrap_or(0);
    let absolute_start = base_offset + local_start;

    if trimmed.starts_with('(') {
        let expr = parse_grouped_expression(trimmed, absolute_start)?;
        return Ok(Primary::Grouped(Box::new(expr)));
    }

    let pairs = DslParser::parse(Rule::fragment, trimmed).map_err(|error| {
        use pest::error::InputLocation;
        let pos = match error.location {
            InputLocation::Pos(position) => position,
            InputLocation::Span((start, _)) => start,
        };
        ParseError::Syntax {
            span: Span::new(absolute_start + pos, absolute_start + pos + 1),
            message: error.to_string(),
        }
    })?;

    let fragment = pairs.into_iter().next().ok_or_else(|| ParseError::Syntax {
        span: Span::new(absolute_start, absolute_start + trimmed.len().max(1)),
        message: "empty fragment parse".to_string(),
    })?;

    let primary_pair = fragment
        .into_inner()
        .find(|pair| pair.as_rule() == Rule::primary)
        .ok_or_else(|| ParseError::Syntax {
            span: Span::new(absolute_start, absolute_start + trimmed.len().max(1)),
            message: "missing primary".to_string(),
        })?;

    parse_primary(primary_pair, trimmed, absolute_start)
}

fn spanned_text<'i>(
    pair: pest::iterators::Pair<'i, Rule>,
    source: &str,
    base_offset: usize,
) -> Spanned<String> {
    let span = span_from_pair(pair.as_span(), source);
    let value = pair.as_str().to_string();
    Spanned::new(
        value,
        Span::new(base_offset + span.start, base_offset + span.end),
    )
}

fn parse_primary(
    pair: pest::iterators::Pair<'_, Rule>,
    source: &str,
    base_offset: usize,
) -> Result<Primary, ParseError> {
    let pair_span = span_from_pair(pair.as_span(), source);
    let mut inner = pair.into_inner();
    let child = inner.next().ok_or_else(|| ParseError::Syntax {
        span: Span::new(base_offset + pair_span.start, base_offset + pair_span.end),
        message: "empty primary".to_string(),
    })?;

    match child.as_rule() {
        Rule::cli_only => Ok(Primary::CliOnly(parse_cli_args(
            child.into_inner().next().unwrap(),
            source,
            base_offset,
        )?)),
        Rule::url_primary => {
            let mut parts = child.into_inner();
            let url_pair = parts.next().unwrap();
            let url = parse_url(url_pair, source, base_offset)?;
            let cli = parts
                .next()
                .map(|p| parse_cli_args(p, source, base_offset))
                .transpose()?;
            Ok(Primary::Url { url, cli })
        }
        _ => Err(ParseError::Syntax {
            span: Span::new(base_offset + pair_span.start, base_offset + pair_span.end),
            message: format!("unexpected primary rule {:?}", child.as_rule()),
        }),
    }
}

fn parse_query_spanned(
    pair: pest::iterators::Pair<'_, Rule>,
    source: &str,
    base_offset: usize,
) -> Spanned<String> {
    let spanned = spanned_text(pair, source, base_offset);
    let body = spanned
        .value
        .strip_prefix('?')
        .unwrap_or(spanned.value.as_str())
        .to_string();
    Spanned::new(body, spanned.span)
}

#[derive(Default)]
struct UrlParts {
    scheme: Option<Spanned<String>>,
    host: Option<Spanned<String>>,
    port: Option<Spanned<String>>,
    path: Option<Spanned<String>>,
    query: Option<Spanned<String>>,
}

fn apply_url_part(
    part: pest::iterators::Pair<'_, Rule>,
    source: &str,
    base_offset: usize,
    parts: &mut UrlParts,
) {
    match part.as_rule() {
        Rule::scheme => parts.scheme = Some(spanned_text(part, source, base_offset)),
        Rule::host => parts.host = Some(spanned_text(part, source, base_offset)),
        Rule::port => parts.port = Some(spanned_text(part, source, base_offset)),
        Rule::path => parts.path = Some(spanned_text(part, source, base_offset)),
        Rule::query => parts.query = Some(parse_query_spanned(part, source, base_offset)),
        Rule::host_only | Rule::host_with_port | Rule::host_spaced => {
            for inner in part.into_inner() {
                apply_url_part(inner, source, base_offset, parts);
            }
        }
        _ => {}
    }
}

fn parse_url(
    pair: pest::iterators::Pair<'_, Rule>,
    source: &str,
    base_offset: usize,
) -> Result<Url, ParseError> {
    let span = span_from_pair(pair.as_span(), source);
    let child = pair.into_inner().next().ok_or_else(|| ParseError::Syntax {
        span: Span::new(base_offset + span.start, base_offset + span.end),
        message: "empty url".to_string(),
    })?;
    let mut parts = UrlParts::default();

    match child.as_rule() {
        Rule::scheme_url | Rule::scheme_only | Rule::host_url => {
            for part in child.into_inner() {
                apply_url_part(part, source, base_offset, &mut parts);
            }
        }
        Rule::path_url => {
            for part in child.into_inner() {
                apply_url_part(part, source, base_offset, &mut parts);
            }
        }
        Rule::query_only => {
            let query_pair = child
                .into_inner()
                .find(|p| p.as_rule() == Rule::query)
                .unwrap();
            parts.query = Some(parse_query_spanned(query_pair, source, base_offset));
        }
        _ => {
            return Err(ParseError::Syntax {
                span: Span::new(base_offset + span.start, base_offset + span.end),
                message: format!("unexpected url rule {:?}", child.as_rule()),
            });
        }
    }

    Ok(Url {
        scheme: parts.scheme,
        host: parts.host,
        port: parts.port,
        path: parts.path,
        query: parts.query,
        span: Span::new(base_offset + span.start, base_offset + span.end),
    })
}

fn parse_cli_args(
    pair: pest::iterators::Pair<'_, Rule>,
    source: &str,
    base_offset: usize,
) -> Result<CliArgs, ParseError> {
    let span = span_from_pair(pair.as_span(), source);
    let args = pair
        .into_inner()
        .filter(|p| p.as_rule() == Rule::cli_arg)
        .map(|p| parse_cli_arg(p, source, base_offset))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CliArgs {
        args,
        span: Span::new(base_offset + span.start, base_offset + span.end),
    })
}

fn parse_cli_arg(
    pair: pest::iterators::Pair<'_, Rule>,
    source: &str,
    base_offset: usize,
) -> Result<CliArg, ParseError> {
    let span = span_from_pair(pair.as_span(), source);
    let mut inner = pair.into_inner();
    let flag_pair = inner.next().ok_or_else(|| ParseError::Syntax {
        span: Span::new(base_offset + span.start, base_offset + span.end),
        message: "missing cli flag".to_string(),
    })?;
    let flag = spanned_text(flag_pair, source, base_offset);
    let value = inner
        .next()
        .map(|p| parse_cli_arg_value(p, source, base_offset))
        .transpose()?;
    Ok(CliArg {
        flag,
        value,
        span: Span::new(base_offset + span.start, base_offset + span.end),
    })
}

fn parse_cli_arg_value(
    pair: pest::iterators::Pair<'_, Rule>,
    source: &str,
    base_offset: usize,
) -> Result<CliArgValue, ParseError> {
    let pair_span = span_from_pair(pair.as_span(), source);
    let child = pair.into_inner().next().ok_or_else(|| ParseError::Syntax {
        span: Span::new(base_offset + pair_span.start, base_offset + pair_span.end),
        message: "empty cli arg value".to_string(),
    })?;
    match child.as_rule() {
        Rule::eq_cli_value => {
            let value_pair = child
                .into_inner()
                .find(|p| p.as_rule() == Rule::cli_value)
                .unwrap();
            Ok(CliArgValue::Eq(spanned_text(
                value_pair,
                source,
                base_offset,
            )))
        }
        Rule::cli_value => Ok(CliArgValue::Bare(spanned_text(child, source, base_offset))),
        _ => Err(ParseError::Syntax {
            span: Span::new(base_offset + pair_span.start, base_offset + pair_span.end),
            message: "unexpected cli arg value".to_string(),
        }),
    }
}

pub fn has_parse_errors(source: &str) -> bool {
    parse_program(source).is_err()
}

// Backwards-compatible exports used during migration.
pub fn normalize_logic_keywords(source: &str) -> String {
    source.to_string()
}
