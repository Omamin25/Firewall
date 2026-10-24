use crate::model::L7Detail;

const MAX_HEADER_SCAN: usize = 2048;
const METHODS: [&str; 9] = [
    "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", "TRACE", "CONNECT",
];

/// Parses a cleartext HTTP/1.x request line or status line plus the `Host` header.
/// Bodies are ignored; only the first ~2 KB of headers is scanned.
pub fn parse(payload: &[u8]) -> Option<L7Detail> {
    let window = payload.get(..payload.len().min(MAX_HEADER_SCAN))?;
    let text = std::str::from_utf8(window).ok().or_else(|| {
        // A request split mid-UTF-8 still yields a usable prefix.
        std::str::from_utf8(&window[..window.len().saturating_sub(3)]).ok()
    })?;
    let mut lines = text.split("\r\n");
    let first = lines.next()?;

    if let Some(status) = parse_status_line(first) {
        return Some(L7Detail::Http {
            method: None,
            host: None,
            path: None,
            status: Some(status),
        });
    }

    let (method, path) = parse_request_line(first)?;
    let host = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("host")
            .then(|| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    });

    Some(L7Detail::Http {
        method: Some(method),
        host,
        path: Some(path),
        status: None,
    })
}

fn parse_request_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split(' ');
    let method = parts.next()?;
    let path = parts.next()?;
    let version = parts.next()?;
    if !METHODS.contains(&method) || !version.starts_with("HTTP/") {
        return None;
    }
    Some((method.to_owned(), path.to_owned()))
}

fn parse_status_line(line: &str) -> Option<u16> {
    let rest = line.strip_prefix("HTTP/")?;
    let (_, after_version) = rest.split_once(' ')?;
    let code = after_version.split(' ').next()?;
    let code: u16 = code.parse().ok()?;
    (100..600).contains(&code).then_some(code)
}
