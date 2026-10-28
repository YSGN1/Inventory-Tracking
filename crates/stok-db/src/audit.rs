use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Row, SqliteConnection};
use stok_core::{Ctx, Result, UserId, now_ms};
use ts_rs::TS;
use uuid::Uuid;

use crate::store::{DbExt, Store, get_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    Restore,
    Purge,
}

impl AuditAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Restore => "restore",
            Self::Purge => "purge",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "create" => Self::Create,
            "update" => Self::Update,
            "delete" => Self::Delete,
            "restore" => Self::Restore,
            _ => Self::Purge,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AuditEntry {
    pub id: String,
    pub entity: String,
    pub entity_id: String,
    pub action: AuditAction,
    #[ts(type = "unknown")]
    pub before: Option<Value>,
    #[ts(type = "unknown")]
    pub after: Option<Value>,
    pub user_id: UserId,
    pub user_name: Option<String>,
    pub created_at: i64,
}

pub(crate) async fn write_audit(
    conn: &mut SqliteConnection,
    ctx: &Ctx,
    entity: &str,
    entity_id: &str,
    action: AuditAction,
    before: Option<Value>,
    after: Option<Value>,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO audit_log (id, tenant_id, entity, entity_id, action, before_json, after_json, user_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)",
    )
    .bind(Uuid::now_v7().to_string())
    .bind(ctx.tenant_id.to_string())
    .bind(entity)
    .bind(entity_id)
    .bind(action.as_str())
    .bind(before.map(|v| v.to_string()))
    .bind(after.map(|v| v.to_string()))
    .bind(ctx.user_id.to_string())
    .bind(now_ms())
    .execute(conn)
    .await
    .db()?;
    Ok(())
}

impl Store {
    /// Bir varlığın değişiklik geçmişi, yeniden eskiye.
    pub async fn audit_history(&self, ctx: &Ctx, entity: &str, entity_id: &str) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query(
            "SELECT a.*, u.display_name AS user_name FROM audit_log a LEFT JOIN users u ON u.id = a.user_id
             WHERE a.tenant_id = $1 AND a.entity = $2 AND a.entity_id = $3 ORDER BY a.id DESC",
        )
        .bind(ctx.tenant_id.to_string())
        .bind(entity)
        .bind(entity_id)
        .fetch_all(&self.pool)
        .await
        .db()?;
        rows.iter()
            .map(|r| {
                let json = |c: &str| -> Result<Option<Value>> {
                    let s: Option<String> = r.try_get(c).db()?;
                    Ok(s.and_then(|s| serde_json::from_str(&s).ok()))
                };
                Ok(AuditEntry {
                    id: r.try_get("id").db()?,
                    entity: r.try_get("entity").db()?,
                    entity_id: r.try_get("entity_id").db()?,
                    action: AuditAction::parse(&r.try_get::<String, _>("action").db()?),
                    before: json("before_json")?,
                    after: json("after_json")?,
                    user_id: get_id(r, "user_id")?,
                    user_name: r.try_get("user_name").db()?,
                    created_at: r.try_get("created_at").db()?,
                })
            })
            .collect()
    }
}
