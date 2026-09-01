use serde::Serialize;

#[derive(Serialize)]
pub struct Ack {
    pub ok: bool,
}

pub fn ack() -> Ack {
    Ack { ok: true }
}
