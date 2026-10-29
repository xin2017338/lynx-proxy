use crate::ast::Url;

pub fn is_bare_fuzzy_keyword_url(url: &Url) -> bool {
    let Some(host) = &url.host else {
        return false;
    };
    url.scheme.is_none()
        && url.port.is_none()
        && url.path.is_none()
        && url.query.is_none()
        && is_fuzzy_keyword_candidate(&host.value)
}

pub fn is_fuzzy_keyword_candidate(value: &str) -> bool {
    if value.eq_ignore_ascii_case("localhost") || value.contains('.') {
        return false;
    }
    if !value.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return false;
    }
    !value
        .split('.')
        .all(|segment| !segment.is_empty() && segment.chars().all(|ch| ch.is_ascii_digit()))
}
