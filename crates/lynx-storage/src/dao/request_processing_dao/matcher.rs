use std::collections::HashSet;

use anyhow::Result;
use axum::{body::HttpBody, extract::Request};
use lynx_dsl::{MatchProgram, RequestFacts, compile_match_expr, eval_program};

use super::types::RequestRule;

#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub rule: RequestRule,
    pub program: MatchProgram,
}

/// IR-based matcher (matchExpr → MatchProgram), evaluated on request facts.
pub struct RuleMatcher;

impl RuleMatcher {
    pub fn compile_rules(rules: &[RequestRule]) -> Result<Vec<CompiledRule>> {
        let mut compiled = Vec::with_capacity(rules.len());
        for rule in rules {
            let expr = rule.capture.match_expr.trim();
            let program = compile_match_expr(expr).map_err(|error| {
                anyhow::anyhow!("Rule {} matchExpr invalid: {error}", rule.id.unwrap_or(-1))
            })?;
            compiled.push(CompiledRule {
                rule: rule.clone(),
                program,
            });
        }
        Ok(compiled)
    }

    pub fn find_matching_rules<T: HttpBody>(
        compiled_rules: &[CompiledRule],
        disabled_project_ids: &HashSet<String>,
        request: &Request<T>,
    ) -> Result<Vec<RequestRule>> {
        let facts = request_facts_from_request(request);
        let mut matching = Vec::new();
        for compiled in compiled_rules {
            if !compiled.rule.enabled {
                continue;
            }
            if disabled_project_ids.contains(&compiled.rule.project) {
                continue;
            }
            if eval_program(&compiled.program, &facts) {
                matching.push(compiled.rule.clone());
            }
        }
        Ok(matching)
    }
}

fn request_facts_from_request<T: HttpBody>(request: &Request<T>) -> RequestFacts {
    let uri = request.uri();
    let scheme = uri.scheme_str().map(|s| s.to_string());
    let query = uri.query().map(|q| q.to_string());
    let path = uri.path().to_string();
    let method = request.method().as_str().to_string();

    let (host, port) = host_and_port(uri.host(), uri.port_u16(), request.headers());

    let mut headers: Vec<(String, String)> = Vec::new();
    for (name, value) in request.headers().iter() {
        let key = name.as_str().to_ascii_lowercase();
        let val = value.to_str().unwrap_or_default().to_string();
        headers.push((key, val));
    }
    headers.sort_by(|(l, _), (r, _)| l.cmp(r));

    RequestFacts {
        scheme,
        host,
        port,
        path,
        query,
        method,
        headers,
    }
}

fn host_and_port(
    uri_host: Option<&str>,
    uri_port: Option<u16>,
    headers: &http::HeaderMap,
) -> (String, Option<u16>) {
    if let Some(host) = uri_host {
        return (host.to_string(), uri_port);
    }

    let host_value = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    if let Some((host, port)) = host_value.rsplit_once(':')
        && let Ok(port) = port.parse::<u16>()
    {
        return (host.to_string(), Some(port));
    }

    (host_value.to_string(), None)
}
