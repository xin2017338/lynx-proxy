//! Lower parsed DSL AST into [`MatchProgram`] IR for repeated evaluation.

use std::sync::Arc;

use thiserror::Error;

use crate::ast::{AndExpr, CliArg, CliArgValue, CliArgs, NotExpr, OrExpr, Primary, Program, Url};
use crate::error::ParseError;
use crate::fuzzy::is_bare_fuzzy_keyword_url;
use crate::ir::{EvalPlan, MatchProgram, PathMatcher, Predicate, SegmentPattern};
use crate::parser::parse_program;
use crate::query::parse_query_pairs;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("empty match expression")]
    Empty,
    #[error("invalid path glob: {0}")]
    InvalidPathGlob(String),
    #[error("invalid port: {0}")]
    InvalidPort(String),
    #[error("cli flag requires a value: {0}")]
    MissingCliValue(String),
}

/// Parse DSL source and lower AST into a cached-ready [`MatchProgram`].
///
/// The AST exists only on the stack inside this function; callers should retain
/// the returned IR for repeated evaluation.
pub fn compile_match_expr(source: &str) -> Result<MatchProgram, CompileError> {
    let program = parse_program(source)?;
    lower_program(&program)
}

fn lower_program(program: &Program) -> Result<MatchProgram, CompileError> {
    let Some(expr) = program.expr.as_ref() else {
        return Err(CompileError::Empty);
    };

    let mut ctx = LowerCtx::default();
    let plan = ctx.lower_or_expr(&expr.or)?;
    Ok(MatchProgram::new(ctx.predicates, plan))
}

#[derive(Default)]
struct LowerCtx {
    predicates: Vec<Predicate>,
}

impl LowerCtx {
    fn lower_or_expr(&mut self, expr: &OrExpr) -> Result<EvalPlan, CompileError> {
        if expr.branches.is_empty() {
            return Err(CompileError::Empty);
        }
        if expr.branches.len() == 1 {
            return self.lower_and_expr(&expr.branches[0]);
        }
        let plans = expr
            .branches
            .iter()
            .map(|branch| self.lower_and_expr(branch))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(EvalPlan::Any(plans))
    }

    fn lower_and_expr(&mut self, expr: &AndExpr) -> Result<EvalPlan, CompileError> {
        if expr.terms.is_empty() {
            return Err(CompileError::Empty);
        }
        if expr.terms.len() == 1 {
            return self.lower_not_expr(&expr.terms[0]);
        }
        let plans = expr
            .terms
            .iter()
            .map(|term| self.lower_not_expr(term))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(EvalPlan::All(plans))
    }

    fn lower_not_expr(&mut self, expr: &NotExpr) -> Result<EvalPlan, CompileError> {
        match expr {
            NotExpr::Not { inner, .. } => {
                let plan = self.lower_not_expr(inner)?;
                Ok(EvalPlan::Not(Box::new(plan)))
            }
            NotExpr::Primary(primary) => self.lower_primary(primary),
        }
    }

    fn lower_primary(&mut self, primary: &Primary) -> Result<EvalPlan, CompileError> {
        match primary {
            Primary::Grouped(expr) => self.lower_or_expr(&expr.or),
            Primary::CliOnly(cli) => self.lower_cli(cli),
            Primary::Url { url, cli } => {
                let mut plans = Vec::new();
                plans.push(self.lower_url(url)?);
                if let Some(cli) = cli {
                    plans.push(self.lower_cli(cli)?);
                }
                if plans.len() == 1 {
                    Ok(plans.into_iter().next().unwrap())
                } else {
                    Ok(EvalPlan::All(plans))
                }
            }
        }
    }

    fn lower_url(&mut self, url: &Url) -> Result<EvalPlan, CompileError> {
        let mut indices = Vec::new();

        if is_bare_fuzzy_keyword_url(url) {
            let keyword = url.host.as_ref().unwrap().value.to_ascii_lowercase();
            indices.push(self.push_predicate(Predicate::FuzzyContains(Arc::from(keyword))));
            return Ok(plan_all_indices(indices));
        }

        if let Some(scheme) = &url.scheme {
            let normalized = normalize_scheme(&scheme.value);
            indices.push(self.push_predicate(Predicate::SchemeEq(Arc::from(normalized.as_str()))));
        }
        if let Some(host) = &url.host {
            indices.push(self.push_predicate(Predicate::HostEq(Arc::from(
                host.value.to_ascii_lowercase(),
            ))));
        }
        if let Some(port) = &url.port {
            let port_num = parse_port(&port.value)?;
            indices.push(self.push_predicate(Predicate::PortEq(port_num)));
        }
        if let Some(path) = &url.path {
            indices.push(self.push_predicate(Predicate::PathGlob(compile_path(&path.value)?)));
        }
        if let Some(query) = &url.query {
            let pairs: Vec<(Arc<str>, Arc<str>)> = parse_query_pairs(&query.value)
                .into_iter()
                .map(|(key, value)| (Arc::from(key.as_str()), Arc::from(value.as_str())))
                .collect();
            if pairs.is_empty() {
                return Err(CompileError::Empty);
            }
            indices.push(self.push_predicate(Predicate::QueryParamsAll(pairs)));
        }

        if indices.is_empty() {
            return Err(CompileError::Empty);
        }

        Ok(plan_all_indices(indices))
    }

    fn lower_cli(&mut self, cli: &CliArgs) -> Result<EvalPlan, CompileError> {
        let mut indices = Vec::new();
        for arg in &cli.args {
            if let Some(index) = self.lower_cli_arg(arg)? {
                indices.push(index);
            }
        }
        if indices.is_empty() {
            return Err(CompileError::Empty);
        }
        Ok(plan_all_indices(indices))
    }

    fn lower_cli_arg(&mut self, arg: &CliArg) -> Result<Option<usize>, CompileError> {
        let flag = normalize_cli_flag(&arg.flag.value);

        if is_method_flag(&flag) {
            let method = cli_value(arg)?.to_ascii_uppercase();
            return Ok(Some(
                self.push_predicate(Predicate::MethodEq(Arc::from(method))),
            ));
        }

        if is_header_flag(&flag) {
            let raw = cli_value(arg)?;
            let (key, value) = split_header_assignment(raw);
            return Ok(Some(self.push_predicate(Predicate::HeaderEq {
                key: Arc::from(key.to_ascii_lowercase()),
                value: Arc::from(value),
            })));
        }

        if is_query_flag(&flag) {
            let raw = cli_value(arg)?;
            return Ok(Some(
                self.push_predicate(Predicate::QueryContains(Arc::from(raw))),
            ));
        }

        Ok(None)
    }

    fn push_predicate(&mut self, predicate: Predicate) -> usize {
        let index = self.predicates.len();
        self.predicates.push(predicate);
        index
    }
}

fn plan_all_indices(indices: Vec<usize>) -> EvalPlan {
    if indices.len() == 1 {
        EvalPlan::Pred(indices[0])
    } else {
        EvalPlan::All(indices.into_iter().map(EvalPlan::Pred).collect())
    }
}

fn normalize_scheme(raw: &str) -> String {
    raw.trim_end_matches("://").to_ascii_lowercase()
}

fn parse_port(raw: &str) -> Result<u16, CompileError> {
    let digits = raw.trim_start_matches(':');
    digits
        .parse::<u16>()
        .map_err(|_| CompileError::InvalidPort(raw.to_string()))
}

fn cli_value(arg: &CliArg) -> Result<&str, CompileError> {
    match &arg.value {
        Some(CliArgValue::Eq(value) | CliArgValue::Bare(value)) => Ok(value.value.as_str()),
        None => Err(CompileError::MissingCliValue(arg.flag.value.clone())),
    }
}

fn normalize_cli_flag(flag: &str) -> String {
    flag.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn is_method_flag(flag: &str) -> bool {
    flag.eq_ignore_ascii_case("-X") || flag.eq_ignore_ascii_case("--request")
}

fn is_header_flag(flag: &str) -> bool {
    flag.eq_ignore_ascii_case("-H") || flag.eq_ignore_ascii_case("--header")
}

fn is_query_flag(flag: &str) -> bool {
    flag.eq_ignore_ascii_case("-q") || flag.eq_ignore_ascii_case("--query")
}

fn split_header_assignment(raw: &str) -> (&str, &str) {
    if let Some((key, value)) = raw.split_once('=') {
        (key, value)
    } else {
        (raw, "")
    }
}

fn compile_path(pattern: &str) -> Result<PathMatcher, CompileError> {
    if !pattern.contains('*') {
        return Ok(PathMatcher::Exact(Arc::from(pattern)));
    }

    let segments = split_path_pattern_segments(pattern);
    if segments.is_empty() {
        return Err(CompileError::InvalidPathGlob(pattern.to_string()));
    }

    let mut compiled = Vec::with_capacity(segments.len());
    for segment in segments {
        if segment == "**" {
            compiled.push(SegmentPattern::MultiWildcard);
        } else if segment == "*" {
            compiled.push(SegmentPattern::SingleWildcard);
        } else if segment.contains('*') {
            return Err(CompileError::InvalidPathGlob(pattern.to_string()));
        } else {
            compiled.push(SegmentPattern::Literal(Arc::from(segment)));
        }
    }

    Ok(PathMatcher::Segments(compiled))
}

fn split_path_pattern_segments(pattern: &str) -> Vec<&str> {
    pattern
        .trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Predicate;

    #[test]
    fn compiles_host_and_path_and() {
        let program = compile_match_expr("example.com AND /api").unwrap();
        assert_eq!(program.predicates.len(), 2);
        assert!(matches!(program.predicates[0], Predicate::HostEq(_)));
        assert!(matches!(program.predicates[1], Predicate::PathGlob(_)));
        assert!(matches!(program.plan, EvalPlan::All(_)));
    }

    #[test]
    fn compiles_bare_keyword_as_fuzzy_predicate() {
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
    }

    #[test]
    fn rejects_empty_expression() {
        assert_eq!(compile_match_expr("   ").unwrap_err(), CompileError::Empty);
        assert_eq!(
            compile_match_expr("# comment only").unwrap_err(),
            CompileError::Empty
        );
    }
}
