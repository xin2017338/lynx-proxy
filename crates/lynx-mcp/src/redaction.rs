use std::collections::BTreeMap;

const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
];

pub fn headers(input: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    input
        .iter()
        .map(|(key, value)| {
            let value = if SENSITIVE_HEADERS
                .iter()
                .any(|name| key.eq_ignore_ascii_case(name))
            {
                "[REDACTED]".to_owned()
            } else {
                value.clone()
            };
            (key.clone(), value)
        })
        .collect()
}

pub fn url(input: &str) -> String {
    let Ok(mut url) = url::Url::parse(input) else {
        return input.to_owned();
    };
    let sensitive = [
        "token",
        "access_token",
        "refresh_token",
        "password",
        "passwd",
        "secret",
        "api_key",
        "session",
        "credential",
    ];
    let pairs: Vec<_> = url
        .query_pairs()
        .map(|(key, value)| {
            let value = if sensitive.iter().any(|name| key.eq_ignore_ascii_case(name)) {
                "[REDACTED]".into()
            } else {
                value
            };
            (key.into_owned(), value.into_owned())
        })
        .collect();
    if !pairs.is_empty() {
        url.query_pairs_mut().clear().extend_pairs(pairs);
    }
    url.to_string()
}

pub fn text_body(input: &str) -> String {
    let Ok(mut value) = serde_json::from_str::<serde_json::Value>(input) else {
        return input.to_owned();
    };
    redact_json(&mut value);
    serde_json::to_string(&value).unwrap_or_else(|_| input.to_owned())
}

fn redact_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if [
                    "token",
                    "access_token",
                    "refresh_token",
                    "password",
                    "passwd",
                    "secret",
                    "api_key",
                    "session",
                    "credential",
                ]
                .iter()
                .any(|name| key.eq_ignore_ascii_case(name))
                {
                    *value = serde_json::Value::String("[REDACTED]".to_owned());
                } else {
                    redact_json(value);
                }
            }
        }
        serde_json::Value::Array(values) => values.iter_mut().for_each(redact_json),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn redacts_query_secret() {
        assert!(
            super::url("https://example.com/?token=secret&q=ok").contains("token=%5BREDACTED%5D")
        );
    }

    #[test]
    fn redacts_nested_json_secret() {
        let output = super::text_body(r#"{"user":{"password":"secret"},"ok":true}"#);
        assert!(!output.contains("secret"));
        assert!(output.contains("[REDACTED]"));
    }
}
