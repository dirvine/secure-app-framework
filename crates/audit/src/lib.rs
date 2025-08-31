#![forbid(unsafe_code)]

/// Minimal placeholder for audit log API.
pub struct AuditEvent {
    pub message: String,
}

pub fn record_event(_evt: &AuditEvent) -> Result<(), String> {
    // TODO: implement BLAKE3 hash-chained log (MVP placeholder)
    Ok(())
}
