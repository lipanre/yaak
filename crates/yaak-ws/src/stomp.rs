use url::Url;

const NUL: char = '\0';

pub fn frame_command(frame: &str) -> &str {
    frame.lines().next().unwrap_or_default().trim()
}

pub fn connect_frame(url: &Url) -> String {
    frame(
        "CONNECT",
        &[
            ("accept-version", "1.2,1.1,1.0"),
            ("host", url.host_str().unwrap_or("localhost")),
            ("heart-beat", "0,0"),
        ],
        "",
    )
}

pub fn subscribe_frames(subscriptions: &[String]) -> Vec<String> {
    subscriptions
        .iter()
        .map(|destination| destination.trim())
        .filter(|destination| !destination.is_empty())
        .enumerate()
        .map(|(index, destination)| {
            frame(
                "SUBSCRIBE",
                &[
                    ("id", &format!("sub-{index}")),
                    ("destination", destination),
                ],
                "",
            )
        })
        .collect()
}

pub fn send_frame(destination: &str, body: &str) -> String {
    frame(
        "SEND",
        &[
            ("destination", destination.trim()),
            ("content-length", &body.len().to_string()),
        ],
        body,
    )
}

fn frame(command: &str, headers: &[(&str, &str)], body: &str) -> String {
    let mut output = String::new();
    output.push_str(command);
    output.push('\n');
    for (name, value) in headers {
        output.push_str(&escape_header(name));
        output.push(':');
        output.push_str(&escape_header(value));
        output.push('\n');
    }
    output.push('\n');
    output.push_str(body);
    output.push(NUL);
    output
}

fn escape_header(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\r', "\\r").replace('\n', "\\n").replace(':', "\\c")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_connect_frame() {
        let url = Url::parse("wss://example.com/ws").unwrap();
        assert_eq!(
            connect_frame(&url),
            "CONNECT\naccept-version:1.2,1.1,1.0\nhost:example.com\nheart-beat:0,0\n\n\0"
        );
    }

    #[test]
    fn builds_subscription_frames() {
        let frames = subscribe_frames(&["/topic/a".into(), "".into(), "/queue/b".into()]);
        assert_eq!(frames.len(), 2);
        assert_eq!(frame_command(&frames[0]), "SUBSCRIBE");
        assert!(frames[0].contains("id:sub-0\ndestination:/topic/a"));
        assert!(frames[1].contains("id:sub-1\ndestination:/queue/b"));
    }

    #[test]
    fn builds_send_frame_with_byte_length() {
        let frame = send_frame("/topic/a", "hello");
        assert_eq!(frame, "SEND\ndestination:/topic/a\ncontent-length:5\n\nhello\0");
    }

    #[test]
    fn escapes_header_separators() {
        let frame = send_frame("/topic/a:b", "body");
        assert!(frame.contains("destination:/topic/a\\cb"));
    }
}
