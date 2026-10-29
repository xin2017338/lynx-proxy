use crate::ast::{
    AndExpr, CliArg, CliArgValue, Expr, NotExpr, OrExpr, Primary, Program, Span, Url,
};
use crate::fuzzy::is_bare_fuzzy_keyword_url;
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HighlightSpan {
    pub from: usize,
    pub to: usize,
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    AndOp,
    OrOp,
    NotOp,
    Scheme,
    Host,
    FuzzyKeyword,
    Port,
    Path,
    Query,
    LineComment,
    ShortFlag,
    LongFlag,
    CliValue,
    Paren,
}

impl HighlightKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AndOp => "AndOp",
            Self::OrOp => "OrOp",
            Self::NotOp => "NotOp",
            Self::Scheme => "Scheme",
            Self::Host => "Host",
            Self::FuzzyKeyword => "FuzzyKeyword",
            Self::Port => "Port",
            Self::Path => "Path",
            Self::Query => "Query",
            Self::LineComment => "LineComment",
            Self::ShortFlag => "ShortFlag",
            Self::LongFlag => "LongFlag",
            Self::CliValue => "CliValue",
            Self::Paren => "Paren",
        }
    }
}

pub fn collect_highlights(source: &str, program: &Program) -> Vec<HighlightSpan> {
    let mut spans = collect_comment_highlights(source);
    if let Some(expr) = &program.expr {
        highlight_expr(expr, source, &mut spans);
    }
    spans.sort_by_key(|span| (span.from, span.to));
    spans
}

fn collect_comment_highlights(source: &str) -> Vec<HighlightSpan> {
    let mut highlights = Vec::new();
    let mut offset = 0usize;
    for line in source.split_inclusive('\n') {
        if let Some(hash_index) = line.find('#') {
            let from = offset + hash_index;
            let to = offset + line.trim_end_matches('\n').len();
            if to > from {
                highlights.push(HighlightSpan {
                    from,
                    to,
                    kind: HighlightKind::LineComment.as_str().to_string(),
                });
            }
        }
        offset += line.len();
    }
    highlights
}

fn highlight_expr(expr: &Expr, source: &str, spans: &mut Vec<HighlightSpan>) {
    highlight_or_expr(&expr.or, source, spans);
}

fn and_expr_end(and: &AndExpr) -> usize {
    and.terms.last().map(not_expr_end).unwrap_or(and.span.end)
}

fn and_expr_start(and: &AndExpr) -> usize {
    and.terms
        .first()
        .map(not_expr_start)
        .unwrap_or(and.span.start)
}

fn highlight_or_expr(or: &OrExpr, source: &str, spans: &mut Vec<HighlightSpan>) {
    for window in or.branches.windows(2) {
        let left = and_expr_end(&window[0]);
        let right = and_expr_start(&window[1]);
        let search_from = left.min(window[0].span.end);
        let search_to = right.max(window[1].span.start);
        if search_from < search_to {
            highlight_operator_between(source, search_from, search_to, HighlightKind::OrOp, spans);
        }
    }
    for branch in &or.branches {
        highlight_and_expr(branch, source, spans);
    }
}

fn highlight_and_expr(and: &AndExpr, source: &str, spans: &mut Vec<HighlightSpan>) {
    for window in and.terms.windows(2) {
        let left = not_expr_end(&window[0]);
        let right = not_expr_start(&window[1]);
        highlight_operator_between(source, left, right, HighlightKind::AndOp, spans);
    }
    for term in &and.terms {
        highlight_not_expr(term, source, spans);
    }
}

fn highlight_not_expr(not_expr: &NotExpr, source: &str, spans: &mut Vec<HighlightSpan>) {
    match not_expr {
        NotExpr::Not { inner, span } => {
            if let Some(not_span) = find_not_operator(source, *span) {
                push_span(spans, not_span, HighlightKind::NotOp);
            }
            highlight_not_expr(inner, source, spans);
        }
        NotExpr::Primary(primary) => highlight_primary(primary, source, spans),
    }
}

fn highlight_primary(primary: &Primary, source: &str, spans: &mut Vec<HighlightSpan>) {
    match primary {
        Primary::CliOnly(cli) => highlight_cli_args(cli, source, spans),
        Primary::Url { url, cli } => {
            highlight_url(url, spans);
            if let Some(cli_args) = cli {
                highlight_cli_args(cli_args, source, spans);
            }
        }
        Primary::Grouped(expr) => {
            let span = expr.span;
            if source[span.start..].starts_with('(') {
                push_span(
                    spans,
                    Span::new(span.start, span.start + 1),
                    HighlightKind::Paren,
                );
            }
            let close = source[..span.end].rfind(')');
            if let Some(index) = close {
                push_span(spans, Span::new(index, index + 1), HighlightKind::Paren);
            }
            highlight_expr(expr, source, spans);
        }
    }
}

fn highlight_url(url: &Url, spans: &mut Vec<HighlightSpan>) {
    if let Some(scheme) = &url.scheme {
        push_span(spans, scheme.span, HighlightKind::Scheme);
    }
    if let Some(host) = &url.host {
        let kind = if is_bare_fuzzy_keyword_url(url) {
            HighlightKind::FuzzyKeyword
        } else {
            HighlightKind::Host
        };
        push_span(spans, host.span, kind);
    }
    if let Some(port) = &url.port {
        push_span(spans, port.span, HighlightKind::Port);
    }
    if let Some(path) = &url.path {
        push_span(spans, path.span, HighlightKind::Path);
    }
    if let Some(query) = &url.query {
        push_span(spans, query.span, HighlightKind::Query);
    }
}

fn highlight_cli_args(cli: &crate::ast::CliArgs, source: &str, spans: &mut Vec<HighlightSpan>) {
    for arg in &cli.args {
        highlight_cli_arg(arg, source, spans);
    }
}

fn highlight_cli_arg(arg: &CliArg, _source: &str, spans: &mut Vec<HighlightSpan>) {
    let kind = if arg.flag.value.starts_with("--") {
        HighlightKind::LongFlag
    } else {
        HighlightKind::ShortFlag
    };
    push_span(spans, arg.flag.span, kind);
    if let Some(value) = &arg.value {
        match value {
            CliArgValue::Eq(value) => {
                push_span(spans, value.span, HighlightKind::CliValue);
            }
            CliArgValue::Bare(value) => {
                push_span(spans, value.span, HighlightKind::CliValue);
            }
        }
    }
}

fn highlight_operator_between(
    source: &str,
    from: usize,
    to: usize,
    kind: HighlightKind,
    spans: &mut Vec<HighlightSpan>,
) {
    if from >= to || to > source.len() {
        return;
    }
    let slice = &source[from..to];
    for mat in slice.match_indices(|c: char| c.is_ascii_alphabetic()) {
        let start = from + mat.0;
        let word_end = source[start..to]
            .find(|c: char| !c.is_ascii_alphabetic())
            .map(|index| start + index)
            .unwrap_or(to);
        let word = &source[start..word_end];
        let matches = match kind {
            HighlightKind::AndOp => matches_ignore_case(word, "AND"),
            HighlightKind::OrOp => matches_ignore_case(word, "OR"),
            _ => false,
        };
        if matches {
            push_span(spans, Span::new(start, word_end), kind);
        }
    }
}

fn find_not_operator(source: &str, span: Span) -> Option<Span> {
    let slice = &source[span.start..span.end.min(source.len())];
    for mat in slice.match_indices(|c: char| c.is_ascii_alphabetic()) {
        let start = span.start + mat.0;
        let word_end = source[start..span.end]
            .find(|c: char| !c.is_ascii_alphabetic())
            .map(|index| start + index)
            .unwrap_or(span.end);
        let word = &source[start..word_end];
        if matches_ignore_case(word, "NOT") {
            return Some(Span::new(start, word_end));
        }
    }
    None
}

fn matches_ignore_case(word: &str, expected: &str) -> bool {
    word.eq_ignore_ascii_case(expected)
}

fn not_expr_start(not_expr: &NotExpr) -> usize {
    match not_expr {
        NotExpr::Not { span, .. } => span.start,
        NotExpr::Primary(primary) => primary_start(primary),
    }
}

fn not_expr_end(not_expr: &NotExpr) -> usize {
    match not_expr {
        NotExpr::Not { span, .. } => span.end,
        NotExpr::Primary(primary) => primary_end(primary),
    }
}

fn primary_start(primary: &Primary) -> usize {
    match primary {
        Primary::CliOnly(cli) => cli.span.start,
        Primary::Url { url, cli } => cli.as_ref().map(|c| c.span.start).unwrap_or(url.span.start),
        Primary::Grouped(expr) => expr.span.start,
    }
}

fn primary_end(primary: &Primary) -> usize {
    match primary {
        Primary::CliOnly(cli) => cli.span.end,
        Primary::Url { url, .. } => url.span.end,
        Primary::Grouped(expr) => expr.span.end,
    }
}

fn push_span(spans: &mut Vec<HighlightSpan>, span: Span, kind: HighlightKind) {
    if span.end > span.start {
        spans.push(HighlightSpan {
            from: span.start,
            to: span.end,
            kind: kind.as_str().to_string(),
        });
    }
}
