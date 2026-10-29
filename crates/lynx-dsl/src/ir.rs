//! Match program intermediate representation (IR).
//!
//! [`MatchProgram`] is the cached runtime form: a flat predicate pool plus a
//! boolean [`EvalPlan`]. It contains no AST nodes, source spans, or parse-time
//! strings.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Compiled match program: flat predicate pool + boolean evaluation plan.
///
/// This is the runtime representation cached across requests (Phase 2).
/// It contains no AST nodes, spans, or parse-time strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchProgram {
    pub predicates: Vec<Predicate>,
    pub plan: EvalPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Predicate {
    HostEq(Arc<str>),
    FuzzyContains(Arc<str>),
    SchemeEq(Arc<str>),
    PortEq(u16),
    PathGlob(PathMatcher),
    MethodEq(Arc<str>),
    QueryContains(Arc<str>),
    QueryParamsAll(Vec<(Arc<str>, Arc<str>)>),
    HeaderEq { key: Arc<str>, value: Arc<str> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PathMatcher {
    Exact(Arc<str>),
    Segments(Vec<SegmentPattern>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentPattern {
    Literal(Arc<str>),
    SingleWildcard,
    MultiWildcard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalPlan {
    All(Vec<EvalPlan>),
    Any(Vec<EvalPlan>),
    Not(Box<EvalPlan>),
    Pred(usize),
}

impl MatchProgram {
    pub fn new(predicates: Vec<Predicate>, plan: EvalPlan) -> Self {
        Self { predicates, plan }
    }
}
