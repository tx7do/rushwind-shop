//! The admin-portal hand face — the three post-login aggregates the
//! shell needs: the navigation routes, the permission codes (the
//! caller's role codes through the core's role domain), and the
//! initial context. The menu ledger this deployment carries is empty
//! (the shell renders its static route table), so the routes answer
//! empty; the permission codes resolve the caller's real roles.

use crate::state::StatusError;

use crate::state::AppState;
use shop_proto::proto::admin::service::v1::{
    InitialContextResponse, ListPermissionCodeResponse, ListRouteResponse,
};
use shop_proto::proto::permission::service::v1::{MenuMeta, MenuRouteItem};

/// The caller's operator id off the forwarded claims (the BFF's auth
/// gate stamps it before this face runs).
fn operator_user_id(ctx: &rushwind_http_binding::ctx::RequestContext) -> i64 {
    ctx.claims
        .as_ref()
        .and_then(|c| c.get("uid"))
        .and_then(|v| v.as_u64())
        .map(|v| v as i64)
        .unwrap_or(0)
}

async fn role_codes_of(
    state: &std::sync::Arc<AppState>,
    user_id: i64,
) -> Result<Vec<String>, StatusError> {
    use shop_proto::proto::identity::service::v1::get_user_request::QueryBy;
    use shop_proto::proto::identity::service::v1::user_service_client::UserServiceClient;
    use shop_proto::proto::identity::service::v1::GetUserRequest;
    use shop_proto::proto::permission::service::v1::role_service_client::RoleServiceClient;
    use shop_proto::proto::permission::service::v1::ListRoleCodesByIdsRequest;
    // The caller's roles off the user row, then those roles' codes.
    let mut users = UserServiceClient::new(state.core_channel.clone());
    let user = users
        .get(tonic::Request::new(GetUserRequest {
            query_by: Some(QueryBy::Id(user_id)),
            ..Default::default()
        }))
        .await
        .map_err(crate::services::map_status)?
        .into_inner();
    let role_ids = user.role_ids;
    let mut codes = Vec::new();
    if !role_ids.is_empty() {
        let mut roles = RoleServiceClient::new(state.core_channel.clone());
        let resp = roles
            .list_role_codes_by_ids(tonic::Request::new(ListRoleCodesByIdsRequest {
                role_ids,
                ..Default::default()
            }))
            .await
            .map_err(crate::services::map_status)?
            .into_inner();
        codes = resp.role_codes;
    }
    Ok(codes)
}

/// The navigation routes — the menu ledger this deployment carries is
/// reads the sys_menus ledger (status ON, tenant 0) and rebuilds the
/// two-level route tree by parent_id: group rows become parents with
/// their child routes nested; the meta JSON (title/icon/order/
/// authority) maps onto MenuMeta.
async fn navigation_items(
    state: &std::sync::Arc<AppState>,
) -> Result<Vec<MenuRouteItem>, StatusError> {
    use shop_proto::proto::pagination::PagingRequest;
    use shop_proto::proto::permission::service::v1::menu_service_client::MenuServiceClient;
    let mut core = MenuServiceClient::new(state.core_channel.clone());
    let resp = core
        .list(tonic::Request::new(PagingRequest {
            page: Some(1),
            page_size: Some(200),
            no_paging: Some(true),
            ..Default::default()
        }))
        .await
        .map_err(crate::services::map_status)?
        .into_inner();
    // Two-level rebuild by parent_id: group rows (no parent) become
    // parents, leaf rows nest under their group.
    let mut parents: Vec<MenuRouteItem> = Vec::new();
    let mut child_map: std::collections::HashMap<i64, Vec<MenuRouteItem>> = Default::default();
    for m in &resp.items {
        let item = MenuRouteItem {
            path: m.path.clone(),
            redirect: m.redirect.clone(),
            name: m.name.clone(),
            component: m.component.clone(),
            meta: m.meta.clone(),
            ..Default::default()
        };
        match m.parent_id {
            Some(pid) => child_map.entry(pid).or_default().push(item),
            None => parents.push(item),
        }
    }
    for parent in &mut parents {
        if let Some(path) = parent.path.as_deref() {
            if let Some(children) = resp
                .items
                .iter()
                .find(|m| m.path.as_deref() == Some(path))
                .and_then(|m| m.id)
                .and_then(|pid| child_map.get(&pid))
            {
                parent.children = children.clone();
            }
        }
    }
    Ok(parents)
}

/// The navigation routes.
pub async fn get_navigation(
    state: &std::sync::Arc<AppState>,
    _ctx: &rushwind_http_binding::ctx::RequestContext,
    _msg: pbjson_types::Empty,
) -> Result<ListRouteResponse, StatusError> {
    Ok(ListRouteResponse {
        items: navigation_items(state).await?,
    })
}

/// The caller's permission codes.
pub async fn get_my_permission_code(
    state: &std::sync::Arc<AppState>,
    ctx: &rushwind_http_binding::ctx::RequestContext,
    _msg: pbjson_types::Empty,
) -> Result<ListPermissionCodeResponse, StatusError> {
    let codes = role_codes_of(state, operator_user_id(ctx)).await?;
    Ok(ListPermissionCodeResponse {
        codes,
        hidden_fields: Vec::new(),
    })
}

/// The shell's first fetch after login: menus + codes in one round.
pub async fn get_initial_context(
    state: &std::sync::Arc<AppState>,
    ctx: &rushwind_http_binding::ctx::RequestContext,
    _msg: pbjson_types::Empty,
) -> Result<InitialContextResponse, StatusError> {
    let menus = navigation_items(state).await?;
    let permissions = role_codes_of(state, operator_user_id(ctx)).await?;
    Ok(InitialContextResponse {
        menus,
        permissions,
        hidden_fields: Vec::new(),
        ..Default::default()
    })
}

/// The mounted face — the trait impl delegating to the free functions.
pub struct AdminPortalService {
    pub state: std::sync::Arc<AppState>,
}

#[async_trait::async_trait]
impl shop_proto::gen_admin::services::AdminPortalServiceHandlers for AdminPortalService {
    async fn get_navigation(
        &self,
        ctx: rushwind_http_binding::ctx::RequestContext,
        req: pbjson_types::Empty,
    ) -> Result<ListRouteResponse, rushwind_http_binding::envelope::StatusError> {
        get_navigation(&self.state, &ctx, req).await
    }

    async fn get_my_permission_code(
        &self,
        ctx: rushwind_http_binding::ctx::RequestContext,
        req: pbjson_types::Empty,
    ) -> Result<ListPermissionCodeResponse, rushwind_http_binding::envelope::StatusError> {
        get_my_permission_code(&self.state, &ctx, req).await
    }

    async fn get_initial_context(
        &self,
        ctx: rushwind_http_binding::ctx::RequestContext,
        req: pbjson_types::Empty,
    ) -> Result<InitialContextResponse, rushwind_http_binding::envelope::StatusError> {
        get_initial_context(&self.state, &ctx, req).await
    }
}
