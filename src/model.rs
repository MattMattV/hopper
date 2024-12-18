pub(crate) struct AtUri {
    pub(crate) identity: String,
    pub(crate) collection: Option<String>,
    pub(crate) rkey: Option<String>,
}

pub(crate) fn validate_aturi<S: Into<String>>(aturi: S) -> Option<AtUri> {
    let aturi = aturi.into();
    let aturi = aturi.trim();

    let stripped = aturi.strip_prefix("at://");
    if stripped.is_none() {
        return None;
    }

    let stripped = stripped.unwrap();

    let parts = stripped.split('/').collect::<Vec<&str>>();

    if !parts.is_empty() && !is_valid_identity(parts[0]) {
        return None;
    }
    if parts.len() > 1 && !is_valid_nsid(parts[1]) {
        return None;
    }
    if parts.len() > 3 {
        return None;
    }

    return Some(AtUri {
        identity: parts[0].to_string(),
        collection: parts.get(1).map(|s| s.to_string()),
        rkey: parts.get(2).map(|s| s.to_string()),
    });
}

pub(crate) fn is_valid_nsid(nsid: &str) -> bool {
    fn is_valid_char(byte: u8) -> bool {
        byte.is_ascii_lowercase()
            || byte.is_ascii_uppercase()
            || byte.is_ascii_digit()
            || byte == b'-'
            || byte == b'.'
    }
    !(nsid.bytes().any(|byte| !is_valid_char(byte))
        || nsid.split('.').count() < 3
        || nsid.split('.').any(|label| {
            label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-')
        })
        || nsid.is_empty()
        || nsid.len() > 253)
}

pub(crate) fn is_valid_hostname(hostname: &str) -> bool {
    fn is_valid_char(byte: u8) -> bool {
        byte.is_ascii_lowercase()
            || byte.is_ascii_uppercase()
            || byte.is_ascii_digit()
            || byte == b'-'
            || byte == b'.'
    }
    !(hostname.ends_with(".localhost")
        || hostname.ends_with(".internal")
        || hostname.ends_with(".arpa")
        || hostname.ends_with(".local")
        || hostname.bytes().any(|byte| !is_valid_char(byte))
        || hostname.split('.').any(|label| {
            label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-')
        })
        || hostname.is_empty()
        || hostname.len() > 253)
}

enum InputType {
    Handle(String),
    Plc(String),
    Web(String),
}

pub(crate) fn is_valid_identity(identity: &str) -> bool {
    let identity = if identity.starts_with("did:web:") {
        InputType::Web(identity.to_string())
    } else if identity.starts_with("did:plc:") {
        InputType::Plc(identity.to_string())
    } else {
        InputType::Handle(identity.to_string())
    };

    match identity {
        InputType::Handle(handle) => is_valid_hostname(&handle) && handle.chars().any(|c| c == '.'),
        InputType::Plc(did) => did
            .strip_prefix("did:plc:")
            .is_some_and(|remaining| remaining.len() == 24),
        InputType::Web(did) => {
            let parts = did
                .strip_prefix("did:web:")
                .map(|trimmed| trimmed.split(":").collect::<Vec<&str>>());

            parts.is_some_and(|inner_parts| {
                !inner_parts.is_empty()
                    && inner_parts.first().is_some_and(|hostname| {
                        is_valid_hostname(hostname) && hostname.chars().any(|c| c == '.')
                    })
            })
        }
    }
}
