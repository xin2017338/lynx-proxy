//! Evaluate compiled [`MatchProgram`] IR against request facts.
//!
//! This module must not import the AST; [`crate::compile`] is the sole AST entry point.

use crate::facts::RequestFacts;
use crate::ir::{EvalPlan, MatchProgram, PathMatcher, Predicate, SegmentPattern};
use crate::query::query_params_subset_match;

/// Evaluate a compiled match program against request facts.
pub fn eval_program(program: &MatchProgram, facts: &RequestFacts) -> bool {
    eval_plan(&program.plan, &program.predicates, facts)
}

pub fn eval_predicate(pred: &Predicate, facts: &RequestFacts) -> bool {
    match pred {
        Predicate::HostEq(expected) => host_matches(expected, &facts.host),
        Predicate::FuzzyContains(expected) => request_contains(expected, facts),
        Predicate::SchemeEq(expected) => facts
            .scheme
            .as_deref()
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case(expected)),
        Predicate::PortEq(expected) => facts.port == Some(*expected),
        Predicate::PathGlob(matcher) => match_path(matcher, &facts.path),
        Predicate::MethodEq(expected) => facts.method.eq_ignore_ascii_case(expected),
        Predicate::QueryContains(expected) => facts
            .query
            .as_deref()
            .is_some_and(|query| query.contains(expected.as_ref())),
        Predicate::QueryParamsAll(expected) => {
            query_params_subset_match(expected, facts.query.as_deref())
        }
        Predicate::HeaderEq { key, value } => header_matches(facts, key, value),
    }
}

fn eval_plan(plan: &EvalPlan, predicates: &[Predicate], facts: &RequestFacts) -> bool {
    match plan {
        EvalPlan::Pred(index) => predicates
            .get(*index)
            .is_some_and(|pred| eval_predicate(pred, facts)),
        EvalPlan::All(plans) => plans
            .iter()
            .all(|child| eval_plan(child, predicates, facts)),
        EvalPlan::Any(plans) => plans
            .iter()
            .any(|child| eval_plan(child, predicates, facts)),
        EvalPlan::Not(inner) => !eval_plan(inner, predicates, facts),
    }
}

fn host_matches(expected: &str, actual: &str) -> bool {
    if actual.eq_ignore_ascii_case(expected) {
        return true;
    }

    let expected_lower = expected.to_ascii_lowercase();
    let actual_lower = actual.to_ascii_lowercase();

    if expected_lower.contains('.') {
        return actual_lower.ends_with(&format!(".{expected_lower}"));
    }

    actual_lower.contains(&expected_lower)
}

fn request_contains(expected: &str, facts: &RequestFacts) -> bool {
    let needle = expected.to_ascii_lowercase();
    let mut full = String::with_capacity(
        facts.host.len() + facts.path.len() + facts.query.as_ref().map_or(0, |q| q.len() + 1),
    );
    full.push_str(&facts.host);
    full.push_str(&facts.path);
    if let Some(query) = facts.query.as_deref()
        && !query.is_empty()
    {
        full.push('?');
        full.push_str(query);
    }
    full.to_ascii_lowercase().contains(&needle)
}

fn header_matches(facts: &RequestFacts, key: &str, expected: &str) -> bool {
    match facts
        .headers
        .binary_search_by(|(header_key, _)| header_key.as_str().cmp(key))
    {
        Ok(index) => {
            let (_, actual) = &facts.headers[index];
            if expected.is_empty() {
                true
            } else {
                actual.eq_ignore_ascii_case(expected)
            }
        }
        Err(_) => false,
    }
}

fn match_path(matcher: &PathMatcher, path: &str) -> bool {
    match matcher {
        PathMatcher::Exact(expected) => path_prefix_match(expected.as_ref(), path),
        PathMatcher::Segments(segments) => {
            let path_segments = split_path_segments(path);
            match_path_segments(segments, &path_segments, 0, 0)
        }
    }
}

/// Literal paths without globs match the path itself or any deeper path under that prefix.
fn path_prefix_match(prefix: &str, path: &str) -> bool {
    if !path.starts_with(prefix) {
        return false;
    }
    if path.len() == prefix.len() {
        return true;
    }
    prefix.ends_with('/') || path.as_bytes().get(prefix.len()) == Some(&b'/')
}

fn split_path_segments(path: &str) -> Vec<&str> {
    path.trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn match_path_segments(pattern: &[SegmentPattern], path: &[&str], pi: usize, ti: usize) -> bool {
    if pi == pattern.len() {
        return ti == path.len();
    }

    match &pattern[pi] {
        SegmentPattern::Literal(literal) => {
            if ti >= path.len() {
                return false;
            }
            if !path[ti].eq_ignore_ascii_case(literal) {
                return false;
            }
            match_path_segments(pattern, path, pi + 1, ti + 1)
        }
        SegmentPattern::SingleWildcard => {
            if ti >= path.len() {
                return false;
            }
            match_path_segments(pattern, path, pi + 1, ti + 1)
        }
        SegmentPattern::MultiWildcard => {
            if pi + 1 == pattern.len() {
                return true;
            }
            for next in ti..=path.len() {
                if match_path_segments(pattern, path, pi + 1, next) {
                    return true;
                }
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{PathMatcher, SegmentPattern};
    use std::sync::Arc;

    #[test]
    fn path_glob_single_wildcard() {
        let matcher = PathMatcher::Segments(vec![
            SegmentPattern::Literal(Arc::from("api")),
            SegmentPattern::SingleWildcard,
            SegmentPattern::Literal(Arc::from("v1")),
        ]);
        assert!(match_path(&matcher, "/api/users/v1"));
        assert!(!match_path(&matcher, "/api/users/extra/v1"));
    }

    #[test]
    fn path_glob_multi_wildcard() {
        let matcher = PathMatcher::Segments(vec![
            SegmentPattern::Literal(Arc::from("api")),
            SegmentPattern::MultiWildcard,
            SegmentPattern::Literal(Arc::from("track")),
        ]);
        assert!(match_path(&matcher, "/api/v1/events/track"));
        assert!(!match_path(&matcher, "/api/track/extra"));
    }
}
