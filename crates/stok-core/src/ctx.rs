use crate::ids::{TenantId, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ctx {
    pub tenant_id: TenantId,
    pub user_id: UserId,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
