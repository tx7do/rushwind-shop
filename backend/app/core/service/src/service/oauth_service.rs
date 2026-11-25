//! The OAuth face (the reference's o_auth_service.go) — the linked
//! account local face rides the credential ledger: the OAUTH rows of
//! `sys_user_credentials` (the provider spellings its
//! provider/provider_account_id columns carry). The linking/exchange
//! faces (Start/Confirm/Link/Refresh/Exchange) need the actual
//! third-party OAuth providers — they answer Unimplemented this round;
//! ListProviders answers the catalog the contract enumerates.

use std::sync::Arc;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tonic::{Request, Response, Status};

use crate::state::{bad, db_status, ts_to_proto, AppState};
use store::entities::sys_user_credentials;

use shop_proto::proto::authentication::service::v1 as authv1;

fn provider_spelling(v: i32) -> Option<&'static str> {
    Some(match authv1::OAuthProvider::try_from(v).ok()? {
        authv1::OAuthProvider::Wechat => "WECHAT",
        authv1::OAuthProvider::Qq => "QQ",
        authv1::OAuthProvider::Weibo => "WEIBO",
        authv1::OAuthProvider::Douyin => "DOUYIN",
        authv1::OAuthProvider::Kuaishou => "KUAISHOU",
        authv1::OAuthProvider::Baidu => "BAIDU",
        authv1::OAuthProvider::Alipay => "ALIPAY",
        authv1::OAuthProvider::Taobao => "TAOBAO",
        authv1::OAuthProvider::Jd => "JD",
        _ => return None,
    })
}

fn credential_proto(r: sys_user_credentials::Model) -> authv1::UserCredential {
    authv1::UserCredential {
        id: r.id,
        user_id: r.user_id,
        tenant_id: r.tenant_id,
        identity_type: r
            .identity_type
            .as_deref()
            .and_then(authv1::user_credential::IdentityType::from_str_name)
            .map(|e| e as i32),
        identifier: r.identifier,
        credential_type: r
            .credential_type
            .as_deref()
            .and_then(authv1::user_credential::CredentialType::from_str_name)
            .map(|e| e as i32),
        credential: None, // the secret never rides the management wire
        is_primary: r.is_primary,
        status: r
            .status
            .as_deref()
            .and_then(authv1::user_credential::Status::from_str_name)
            .map(|e| e as i32),
        provider: r.provider,
        provider_account_id: r.provider_account_id,
        created_at: r.created_at.and_then(ts_to_proto),
        updated_at: r.updated_at.and_then(ts_to_proto),
        ..Default::default()
    }
}

pub struct OAuthService {
    pub state: Arc<AppState>,
}

#[async_trait::async_trait]
impl authv1::o_auth_service_server::OAuthService for OAuthService {
    /// The operator's OAUTH rows off the credential ledger.
    async fn list_linked_accounts(
        &self,
        request: Request<authv1::ListLinkedAccountsRequest>,
    ) -> Result<Response<authv1::ListLinkedAccountsResponse>, Status> {
        let uid = crate::service::context::operator_of(&request)?;
        let rows = sys_user_credentials::Entity::find()
            .filter(sys_user_credentials::Column::UserId.eq(uid))
            .filter(sys_user_credentials::Column::IdentityType.eq("OAUTH"))
            .all(&self.state.db)
            .await
            .map_err(db_status)?;
        let total = rows.len() as i64;
        Ok(Response::new(authv1::ListLinkedAccountsResponse {
            items: rows.into_iter().map(credential_proto).collect(),
            total,
            next_page_token: String::new(),
        }))
    }

    async fn get_linked_account(
        &self,
        request: Request<authv1::GetLinkedAccountRequest>,
    ) -> Result<Response<authv1::GetLinkedAccountResponse>, Status> {
        let req = request.into_inner();
        let id: i64 = req
            .credential_id
            .parse()
            .map_err(|_| bad("invalid credential id"))?;
        let row = sys_user_credentials::Entity::find_by_id(id)
            .filter(sys_user_credentials::Column::IdentityType.eq("OAUTH"))
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("linked account not found"))?;
        Ok(Response::new(authv1::GetLinkedAccountResponse {
            account: Some(credential_proto(row)),
        }))
    }

    /// The unbinding face: the credential row goes, the provider
    /// linkage with it.
    async fn unlink_o_auth(
        &self,
        request: Request<authv1::UnlinkOAuthRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let uid = crate::service::context::operator_of(&request)?;
        let req = request.into_inner();
        let id: i64 = req
            .credential_id
            .filter(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| bad("credential_id required"))?;
        let row = sys_user_credentials::Entity::find_by_id(id)
            .filter(sys_user_credentials::Column::UserId.eq(uid))
            .filter(sys_user_credentials::Column::IdentityType.eq("OAUTH"))
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("linked account not found"))?;
        use sea_orm::ModelTrait;
        row.delete(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    async fn revoke_linked_account(
        &self,
        request: Request<authv1::RevokeLinkedAccountRequest>,
    ) -> Result<Response<pbjson_types::Empty>, Status> {
        let uid = crate::service::context::operator_of(&request)?;
        let req = request.into_inner();
        let id: i64 = req
            .credential_id
            .parse()
            .map_err(|_| bad("invalid credential id"))?;
        let row = sys_user_credentials::Entity::find_by_id(id)
            .filter(sys_user_credentials::Column::UserId.eq(uid))
            .filter(sys_user_credentials::Column::IdentityType.eq("OAUTH"))
            .one(&self.state.db)
            .await
            .map_err(db_status)?
            .ok_or_else(|| Status::not_found("linked account not found"))?;
        use sea_orm::ModelTrait;
        row.delete(&self.state.db).await.map_err(db_status)?;
        Ok(Response::new(pbjson_types::Empty {}))
    }

    /// The provider catalog the contract enumerates (the static
    /// metadata; the live callback URLs ride the provider configs).
    async fn list_providers(
        &self,
        _request: Request<authv1::ListProvidersRequest>,
    ) -> Result<Response<authv1::ListProvidersResponse>, Status> {
        let items = [
            (authv1::OAuthProvider::Wechat, "微信"),
            (authv1::OAuthProvider::Qq, "QQ"),
            (authv1::OAuthProvider::Weibo, "微博"),
            (authv1::OAuthProvider::Douyin, "抖音"),
            (authv1::OAuthProvider::Alipay, "支付宝"),
        ]
        .into_iter()
        .map(|(provider, display)| authv1::ProviderMetadata {
            provider: provider as i32,
            display_name: display.to_string(),
            ..Default::default()
        })
        .collect();
        Ok(Response::new(authv1::ListProvidersResponse { items }))
    }

    async fn get_provider_metadata(
        &self,
        request: Request<authv1::GetProviderMetadataRequest>,
    ) -> Result<Response<authv1::ProviderMetadata>, Status> {
        let req = request.into_inner();
        let provider = req.provider;
        let spelling = provider_spelling(provider).ok_or_else(|| bad("unknown provider"))?;
        Ok(Response::new(authv1::ProviderMetadata {
            provider,
            display_name: spelling.to_string(),
            ..Default::default()
        }))
    }

    async fn start_link_o_auth(
        &self,
        _request: Request<authv1::StartLinkOAuthRequest>,
    ) -> Result<Response<authv1::StartLinkOAuthResponse>, Status> {
        Err(Status::unimplemented(
            "live OAuth provider callbacks not yet implemented",
        ))
    }

    async fn confirm_link_o_auth(
        &self,
        _request: Request<authv1::ConfirmLinkOAuthRequest>,
    ) -> Result<Response<authv1::ConfirmLinkOAuthResponse>, Status> {
        Err(Status::unimplemented(
            "live OAuth provider callbacks not yet implemented",
        ))
    }

    async fn link_o_auth(
        &self,
        _request: Request<authv1::LinkOAuthRequest>,
    ) -> Result<Response<authv1::LinkOAuthResponse>, Status> {
        Err(Status::unimplemented(
            "live OAuth provider callbacks not yet implemented",
        ))
    }

    async fn refresh_o_auth_token(
        &self,
        _request: Request<authv1::RefreshOAuthTokenRequest>,
    ) -> Result<Response<authv1::RefreshOAuthTokenResponse>, Status> {
        Err(Status::unimplemented(
            "live OAuth provider callbacks not yet implemented",
        ))
    }

    async fn exchange_o_auth_code(
        &self,
        _request: Request<authv1::ExchangeOAuthCodeRequest>,
    ) -> Result<Response<authv1::ExchangeOAuthCodeResponse>, Status> {
        Err(Status::unimplemented(
            "live OAuth provider callbacks not yet implemented",
        ))
    }
}
