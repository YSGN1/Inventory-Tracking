use serde::Serialize;
use stok_core::UserId;
use stok_core::user::{NewUser, Permission, Role, User};
use tauri::State;
use ts_rs::TS;

use crate::error::ApiResult;
use crate::state::{AppState, is_single_user};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AppStatus {
    pub initialized: bool,
    pub single_user_mode: bool,
    pub user: Option<User>,
}

pub(crate) async fn status(state: &AppState) -> ApiResult<AppStatus> {
    state.auto_login_if_single_user().await?;
    let store = state.store().await;
    // Oturumdaki kullanıcı tazelenir (rol/aktiflik değişmiş olabilir).
    let session = state.session.lock().await.clone();
    let user = match session {
        Some((ctx, _)) => match store.get_user(&ctx, ctx.user_id).await {
            Ok(u) if u.active => Some(u),
            _ => {
                *state.session.lock().await = None;
                None
            }
        },
        None => None,
    };
    Ok(AppStatus { initialized: store.is_initialized().await?, single_user_mode: is_single_user(&store).await?, user })
}

#[tauri::command]
pub async fn app_status(state: State<'_, AppState>) -> ApiResult<AppStatus> {
    status(&state).await
}

#[tauri::command]
pub async fn setup_first_admin(state: State<'_, AppState>, username: String, display_name: String, password: String) -> ApiResult<AppStatus> {
    let s = state.store().await.create_first_admin(&username, &display_name, &password).await?;
    *state.session.lock().await = Some(s);
    status(&state).await
}

#[tauri::command]
pub async fn login(state: State<'_, AppState>, username: String, password: String) -> ApiResult<AppStatus> {
    let s = state.store().await.authenticate(&username, &password).await?;
    *state.session.lock().await = Some(s);
    status(&state).await
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> ApiResult<()> {
    *state.session.lock().await = None;
    Ok(())
}

#[tauri::command]
pub async fn list_users(state: State<'_, AppState>) -> ApiResult<Vec<User>> {
    let (ctx, _) = state.require(Permission::ManageUsers).await?;
    Ok(state.store().await.list_users(&ctx).await?)
}

#[tauri::command]
pub async fn create_user(state: State<'_, AppState>, user: NewUser) -> ApiResult<User> {
    let (ctx, _) = state.require(Permission::ManageUsers).await?;
    Ok(state.store().await.create_user(&ctx, user).await?)
}

#[tauri::command]
pub async fn set_user_role(state: State<'_, AppState>, id: UserId, role: Role, expected_version: i64) -> ApiResult<User> {
    let (ctx, _) = state.require(Permission::ManageUsers).await?;
    Ok(state.store().await.set_role(&ctx, id, role, expected_version).await?)
}

#[tauri::command]
pub async fn set_user_active(state: State<'_, AppState>, id: UserId, active: bool, expected_version: i64) -> ApiResult<User> {
    let (ctx, _) = state.require(Permission::ManageUsers).await?;
    Ok(state.store().await.set_active(&ctx, id, active, expected_version).await?)
}

/// Yönetici herkesin, diğer kullanıcılar yalnız kendi parolasını değiştirir.
#[tauri::command]
pub async fn set_password(state: State<'_, AppState>, id: UserId, password: String, expected_version: i64) -> ApiResult<User> {
    let (ctx, role) = state.require(Permission::ViewAll).await?;
    if id != ctx.user_id && !stok_core::user::can(role, Permission::ManageUsers) {
        return Err(stok_core::DomainError::Forbidden.into());
    }
    Ok(state.store().await.set_password(&ctx, id, &password, expected_version).await?)
}
