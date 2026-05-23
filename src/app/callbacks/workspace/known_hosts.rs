use crate::model::Message;

pub(super) fn trust_known_host_message(host: &str, port: i32) -> Option<Message> {
    let port = known_host_port(port)?;

    Some(Message::TrustKnownHost {
        host: host.to_owned(),
        port,
    })
}

pub(super) fn remove_known_host_message(host: &str, port: i32) -> Option<Message> {
    let port = known_host_port(port)?;

    Some(Message::RemoveKnownHost {
        host: host.to_owned(),
        port,
    })
}

fn known_host_port(port: i32) -> Option<u16> {
    u16::try_from(port).ok().filter(|port| *port > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_port_rejects_invalid_values() {
        assert_eq!(known_host_port(22), Some(22));
        assert_eq!(known_host_port(0), None);
        assert_eq!(known_host_port(-1), None);
        assert_eq!(known_host_port(70_000), None);
    }
}
