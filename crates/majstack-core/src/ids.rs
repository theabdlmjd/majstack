use uuid::Uuid;

pub fn new_id(prefix: &str) -> String {
    let raw = Uuid::new_v4().simple().to_string();
    format!("{prefix}-{}", &raw[..12])
}

pub fn agent_id() -> String {
    let raw = Uuid::new_v4().simple().to_string();
    format!("agent-{}", &raw[..8])
}

pub fn short_hex() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

pub fn event_id() -> String {
    new_id("evt")
}
