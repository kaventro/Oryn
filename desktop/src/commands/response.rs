use serde::Serialize;

#[derive(Serialize)]
pub struct Ack {
    pub ok: bool,
}

pub fn ack() -> Ack {
    Ack { ok: true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack() {
        assert!(ack().ok);
    }
}
