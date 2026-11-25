//! Generated thin-BFF proxies — a table of `passthrough_proxy!`
//! invocations, one per BFF face (see services/proxy_kit.rs for the
//! pass/drop/stub/hand method kinds; the BFF and domain methods share
//! their message types by contract, so no mapping rides here).
//! DO NOT EDIT; regenerate via scripts/gen-proxies.py when the
//! contract re-syncs. Hand-written faces (authentication — captcha/
//! cookie concerns; admin-portal aggregation; file-transfer multipart)
//! live in their own modules.
#![allow(clippy::all)]
#![allow(missing_docs)]

use crate::passthrough_proxy;
use shop_proto::proto::access_key::service::v1 as access_key_v1;
use shop_proto::proto::ai::service::v1 as ai_v1;
use shop_proto::proto::audit::service::v1 as audit_v1;
use shop_proto::proto::authentication::service::v1 as authentication_v1;
use shop_proto::proto::cart::service::v1 as cart_v1;
use shop_proto::proto::catalog::service::v1 as catalog_v1;
use shop_proto::proto::comment::service::v1 as comment_v1;
use shop_proto::proto::config::service::v1 as config_v1;
use shop_proto::proto::coupon::service::v1 as coupon_v1;
use shop_proto::proto::dict::service::v1 as dict_v1;
use shop_proto::proto::identity::service::v1 as identity_v1;
use shop_proto::proto::interaction::service::v1 as interaction_v1;
use shop_proto::proto::internal_message::service::v1 as internal_message_v1;
use shop_proto::proto::invoice::service::v1 as invoice_v1;
use shop_proto::proto::monitor_alert::service::v1 as monitor_alert_v1;
use shop_proto::proto::notification::service::v1 as notification_v1;
use shop_proto::proto::online_session::service::v1 as online_session_v1;
use shop_proto::proto::order::service::v1 as order_v1;
use shop_proto::proto::pagination;
use shop_proto::proto::payment::service::v1 as payment_v1;
use shop_proto::proto::permission::service::v1 as permission_v1;
use shop_proto::proto::redis_cache::service::v1 as redis_cache_v1;
use shop_proto::proto::script::service::v1 as script_v1;
use shop_proto::proto::shipping::service::v1 as shipping_v1;
use shop_proto::proto::storage::service::v1 as storage_v1;
use shop_proto::proto::task::service::v1 as task_v1;
use shop_proto::proto::tax::service::v1 as tax_v1;

passthrough_proxy! {
    /// The pass-through proxy of `AccessKeyServiceHandlers`.
    shop_proto::gen_admin::services::AccessKeyServiceHandlers for AccessKeyProxy {
        client: access_key_v1::access_key_service_client::AccessKeyServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> access_key_v1::ListAccessKeyResponse,
            pass get(access_key_v1::GetAccessKeyRequest) -> access_key_v1::AccessKey,
            pass create(access_key_v1::CreateAccessKeyRequest)
                -> access_key_v1::CreateAccessKeyResponse,
            pass update(access_key_v1::UpdateAccessKeyRequest) -> pbjson_types::Empty,
            pass delete(access_key_v1::DeleteAccessKeyRequest) -> pbjson_types::Empty,
            pass reset_secret(access_key_v1::ResetAccessKeySecretRequest)
                -> access_key_v1::CreateAccessKeyResponse,
            pass issue_token(access_key_v1::IssueTokenRequest) -> access_key_v1::IssueTokenResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiAgentServiceHandlers`.
    shop_proto::gen_admin::services::AiAgentServiceHandlers for AiAgentProxy {
        client: ai_v1::ai_agent_service_client::AiAgentServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> ai_v1::ListAiAgentAppsResponse,
            pass get(ai_v1::GetAiAgentAppRequest) -> ai_v1::AiAgentApp,
            pass create(ai_v1::CreateAiAgentAppRequest) -> ai_v1::AiAgentApp,
            pass update(ai_v1::UpdateAiAgentAppRequest) -> pbjson_types::Empty,
            pass delete(ai_v1::DeleteAiAgentAppRequest) -> pbjson_types::Empty,
            pass approve_run(ai_v1::ApproveAgentRunRequest) -> ai_v1::AiAgentRun,
            pass run(ai_v1::RunAiAgentAppRequest) -> ai_v1::AiAgentRun,
            pass list_runs(pagination::PagingRequest) -> ai_v1::ListAiAgentRunsResponse,
            pass get_run(ai_v1::GetAiAgentRunRequest) -> ai_v1::AiAgentRun,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiChatServiceHandlers`.
    shop_proto::gen_admin::services::AiChatServiceHandlers for AiChatProxy {
        client: ai_v1::ai_chat_service_client::AiChatServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            stub chat(ai_v1::ChatRequest) -> ai_v1::ChatResponse,
            pass generate(ai_v1::GenerateRequest) -> ai_v1::GenerateResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiContentServiceHandlers`.
    shop_proto::gen_admin::services::AiContentServiceHandlers for AiContentProxy {
        client: ai_v1::ai_content_service_client::AiContentServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass generate_content(ai_v1::GenerateContentRequest) -> ai_v1::GenerateContentResponse,
            pass semantic_search(ai_v1::SemanticSearchRequest) -> ai_v1::SemanticSearchResponse,
            stub rebuild_search_index(pbjson_types::Empty) -> ai_v1::RebuildSearchIndexResponse,
            pass get_safety_insights(ai_v1::SafetyInsightsRequest) -> ai_v1::SafetyInsightsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiConversationServiceHandlers`.
    shop_proto::gen_admin::services::AiConversationServiceHandlers for AiConversationProxy {
        client: ai_v1::ai_conversation_service_client::AiConversationServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> ai_v1::ListAiConversationResponse,
            pass get(ai_v1::GetAiConversationRequest) -> ai_v1::AiConversation,
            pass update(ai_v1::UpdateAiConversationRequest) -> pbjson_types::Empty,
            pass delete(ai_v1::DeleteAiConversationRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiKnowledgeBaseServiceHandlers`.
    shop_proto::gen_admin::services::AiKnowledgeBaseServiceHandlers for AiKnowledgeBaseProxy {
        client: ai_v1::ai_knowledge_base_service_client::AiKnowledgeBaseServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> ai_v1::ListAiKnowledgeBaseResponse,
            pass get(ai_v1::GetAiKnowledgeBaseRequest) -> ai_v1::AiKnowledgeBase,
            pass create(ai_v1::CreateAiKnowledgeBaseRequest) -> pbjson_types::Empty,
            pass update(ai_v1::UpdateAiKnowledgeBaseRequest) -> pbjson_types::Empty,
            pass delete(ai_v1::DeleteAiKnowledgeBaseRequest) -> pbjson_types::Empty,
            pass upload_doc(ai_v1::UploadAiDocRequest) -> ai_v1::UploadAiDocResponse,
            pass upload_doc_file(ai_v1::UploadAiDocFileRequest) -> ai_v1::UploadAiDocResponse,
            pass list_docs(ai_v1::ListAiDocsRequest) -> ai_v1::ListAiDocsResponse,
            pass delete_doc(ai_v1::DeleteAiDocRequest) -> pbjson_types::Empty,
            pass search(ai_v1::SearchAiKnowledgeRequest) -> ai_v1::SearchAiKnowledgeResponse,
            pass reindex_docs(ai_v1::ReindexDocsRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiMessageServiceHandlers`.
    shop_proto::gen_admin::services::AiMessageServiceHandlers for AiMessageProxy {
        client: ai_v1::ai_message_service_client::AiMessageServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> ai_v1::ListAiMessageResponse,
            pass delete(ai_v1::DeleteAiMessageRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiProviderServiceHandlers`.
    shop_proto::gen_admin::services::AiProviderServiceHandlers for AiProviderProxy {
        client: ai_v1::ai_provider_service_client::AiProviderServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> ai_v1::ListAiProviderResponse,
            pass get(ai_v1::GetAiProviderRequest) -> ai_v1::AiProvider,
            pass create(ai_v1::CreateAiProviderRequest) -> pbjson_types::Empty,
            pass update(ai_v1::UpdateAiProviderRequest) -> pbjson_types::Empty,
            pass delete(ai_v1::DeleteAiProviderRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiQueryServiceHandlers`.
    shop_proto::gen_admin::services::AiQueryServiceHandlers for AiQueryProxy {
        client: ai_v1::ai_query_service_client::AiQueryServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass ask(ai_v1::AskAiQueryRequest) -> ai_v1::AskAiQueryResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AiUsageLogServiceHandlers`.
    shop_proto::gen_admin::services::AiUsageLogServiceHandlers for AiUsageLogProxy {
        client: ai_v1::ai_usage_log_service_client::AiUsageLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> ai_v1::ListAiUsageLogResponse,
            stub get_usage_summary(pbjson_types::Empty) -> ai_v1::UsageSummaryResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ApiAuditLogServiceHandlers`.
    shop_proto::gen_admin::services::ApiAuditLogServiceHandlers for ApiAuditLogProxy {
        client: audit_v1::api_audit_log_service_client::ApiAuditLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> audit_v1::ListApiAuditLogResponse,
            pass get(audit_v1::GetApiAuditLogRequest) -> audit_v1::ApiAuditLog,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ApiServiceHandlers`.
    shop_proto::gen_admin::services::ApiServiceHandlers for ApiProxy {
        client: permission_v1::api_service_client::ApiServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> permission_v1::ListApiResponse,
            pass get(permission_v1::GetApiRequest) -> permission_v1::Api,
            pass create(permission_v1::CreateApiRequest) -> pbjson_types::Empty,
            pass update(permission_v1::UpdateApiRequest) -> pbjson_types::Empty,
            pass delete(permission_v1::DeleteApiRequest) -> pbjson_types::Empty,
            stub sync_apis(pbjson_types::Empty) -> pbjson_types::Empty,
            stub get_walk_route_data(pbjson_types::Empty) -> permission_v1::ListApiResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AuthenticationServiceHandlers`.
    shop_proto::gen_admin::services::AuthenticationServiceHandlers for AuthenticationProxy {
        client: authentication_v1::authentication_service_client::AuthenticationServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass login(authentication_v1::LoginRequest) -> authentication_v1::LoginResponse,
            stub logout(pbjson_types::Empty) -> pbjson_types::Empty,
            pass refresh_token(authentication_v1::LoginRequest) -> authentication_v1::LoginResponse,
            stub generate_captcha(pbjson_types::Empty)
                -> authentication_v1::GenerateCaptchaResponse,
            pass verify_captcha(authentication_v1::VerifyCaptchaRequest)
                -> authentication_v1::VerifyCaptchaResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `BrandServiceHandlers`.
    shop_proto::gen_admin::services::BrandServiceHandlers for BrandProxy {
        client: catalog_v1::brand_service_client::BrandServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListBrandResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountBrandResponse,
            pass get(catalog_v1::GetBrandRequest) -> catalog_v1::Brand,
            pass create(catalog_v1::CreateBrandRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateBrandsRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateBrandRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteBrandRequest) -> pbjson_types::Empty,
            pass translation_exists(catalog_v1::BrandTranslationExistsRequest)
                -> catalog_v1::BrandTranslationExistsResponse,
            pass get_translation(catalog_v1::GetBrandRequest) -> catalog_v1::BrandTranslation,
            pass create_translation(catalog_v1::CreateBrandTranslationRequest)
                -> catalog_v1::BrandTranslation,
            pass update_translation(catalog_v1::UpdateBrandTranslationRequest)
                -> catalog_v1::BrandTranslation,
            pass delete_translation(catalog_v1::DeleteBrandTranslationRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CartItemServiceHandlers`.
    shop_proto::gen_admin::services::CartItemServiceHandlers for CartItemProxy {
        client: cart_v1::cart_item_service_client::CartItemServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> cart_v1::ListCartItemResponse,
            pass count(pagination::PagingRequest) -> cart_v1::CountCartItemResponse,
            pass get(cart_v1::GetCartItemRequest) -> cart_v1::CartItem,
            pass create(cart_v1::CreateCartItemRequest) -> pbjson_types::Empty,
            pass batch_create(cart_v1::BatchCreateCartItemsRequest) -> pbjson_types::Empty,
            pass update(cart_v1::UpdateCartItemRequest) -> pbjson_types::Empty,
            pass delete(cart_v1::DeleteCartItemRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CartServiceHandlers`.
    shop_proto::gen_admin::services::CartServiceHandlers for CartProxy {
        client: cart_v1::cart_service_client::CartServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> cart_v1::ListCartResponse,
            pass count(pagination::PagingRequest) -> cart_v1::CountCartResponse,
            pass get(cart_v1::GetCartRequest) -> cart_v1::Cart,
            pass create(cart_v1::CreateCartRequest) -> pbjson_types::Empty,
            pass batch_create(cart_v1::BatchCreateCartsRequest) -> pbjson_types::Empty,
            pass update(cart_v1::UpdateCartRequest) -> pbjson_types::Empty,
            pass delete(cart_v1::DeleteCartRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CategoryServiceHandlers`.
    shop_proto::gen_admin::services::CategoryServiceHandlers for CategoryProxy {
        client: catalog_v1::category_service_client::CategoryServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListCategoryResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountCategoryResponse,
            pass get(catalog_v1::GetCategoryRequest) -> catalog_v1::Category,
            pass create(catalog_v1::CreateCategoryRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateCategoriesRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateCategoryRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteCategoryRequest) -> pbjson_types::Empty,
            pass translation_exists(catalog_v1::CategoryTranslationExistsRequest)
                -> catalog_v1::CategoryTranslationExistsResponse,
            pass get_translation(catalog_v1::GetCategoryRequest) -> catalog_v1::CategoryTranslation,
            pass create_translation(catalog_v1::CreateCategoryTranslationRequest)
                -> catalog_v1::CategoryTranslation,
            pass update_translation(catalog_v1::UpdateCategoryTranslationRequest)
                -> catalog_v1::CategoryTranslation,
            pass delete_translation(catalog_v1::DeleteCategoryTranslationRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CommentServiceHandlers`.
    shop_proto::gen_admin::services::CommentServiceHandlers for CommentProxy {
        client: comment_v1::comment_service_client::CommentServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> comment_v1::ListCommentResponse,
            pass count(pagination::PagingRequest) -> comment_v1::CountCommentResponse,
            pass get(comment_v1::GetCommentRequest) -> comment_v1::Comment,
            pass create(comment_v1::CreateCommentRequest) -> pbjson_types::Empty,
            pass update(comment_v1::UpdateCommentRequest) -> pbjson_types::Empty,
            pass delete(comment_v1::DeleteCommentRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ConfigServiceHandlers`.
    shop_proto::gen_admin::services::ConfigServiceHandlers for ConfigProxy {
        client: config_v1::config_service_client::ConfigServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> config_v1::ListConfigResponse,
            pass get(config_v1::GetConfigRequest) -> config_v1::Config,
            pass create(config_v1::CreateConfigRequest) -> pbjson_types::Empty,
            pass update(config_v1::UpdateConfigRequest) -> pbjson_types::Empty,
            pass delete(config_v1::DeleteConfigRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CouponTemplateServiceHandlers`.
    shop_proto::gen_admin::services::CouponTemplateServiceHandlers for CouponTemplateProxy {
        client: coupon_v1::coupon_template_service_client::CouponTemplateServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> coupon_v1::ListCouponTemplateResponse,
            pass count(pagination::PagingRequest) -> coupon_v1::CountCouponTemplateResponse,
            pass get(coupon_v1::GetCouponTemplateRequest) -> coupon_v1::CouponTemplate,
            pass create(coupon_v1::CreateCouponTemplateRequest) -> pbjson_types::Empty,
            pass update(coupon_v1::UpdateCouponTemplateRequest) -> pbjson_types::Empty,
            pass delete(coupon_v1::DeleteCouponTemplateRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `DataAccessAuditLogServiceHandlers`.
    shop_proto::gen_admin::services::DataAccessAuditLogServiceHandlers for DataAccessAuditLogProxy {
        client: audit_v1::data_access_audit_log_service_client::DataAccessAuditLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> audit_v1::ListDataAccessAuditLogResponse,
            pass get(audit_v1::GetDataAccessAuditLogRequest) -> audit_v1::DataAccessAuditLog,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `DictEntryServiceHandlers`.
    shop_proto::gen_admin::services::DictEntryServiceHandlers for DictEntryProxy {
        client: dict_v1::dict_entry_service_client::DictEntryServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> dict_v1::ListDictEntryResponse,
            pass create(dict_v1::CreateDictEntryRequest) -> pbjson_types::Empty,
            pass update(dict_v1::UpdateDictEntryRequest) -> pbjson_types::Empty,
            pass delete(dict_v1::DeleteDictEntryRequest) -> pbjson_types::Empty,
            pass list_by_type_code(dict_v1::ListDictEntryByTypeCodeRequest)
                -> dict_v1::ListDictEntryByTypeCodeResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `DictTypeServiceHandlers`.
    shop_proto::gen_admin::services::DictTypeServiceHandlers for DictTypeProxy {
        client: dict_v1::dict_type_service_client::DictTypeServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> dict_v1::ListDictTypeResponse,
            pass get(dict_v1::GetDictTypeRequest) -> dict_v1::DictType,
            pass create(dict_v1::CreateDictTypeRequest) -> pbjson_types::Empty,
            pass update(dict_v1::UpdateDictTypeRequest) -> pbjson_types::Empty,
            pass delete(dict_v1::DeleteDictTypeRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `FileServiceHandlers`.
    shop_proto::gen_admin::services::FileServiceHandlers for FileProxy {
        client: storage_v1::file_service_client::FileServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> storage_v1::ListFileResponse,
            pass get(storage_v1::GetFileRequest) -> storage_v1::File,
            drop create(storage_v1::CreateFileRequest) -> pbjson_types::Empty,
            drop update(storage_v1::UpdateFileRequest) -> pbjson_types::Empty,
            pass delete(storage_v1::DeleteFileRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `GrouponActivityServiceHandlers`.
    shop_proto::gen_admin::services::GrouponActivityServiceHandlers for GrouponActivityProxy {
        client: catalog_v1::groupon_activity_service_client::GrouponActivityServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListGrouponActivityResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountGrouponActivityResponse,
            pass get(catalog_v1::GetGrouponActivityRequest) -> catalog_v1::GrouponActivity,
            pass create(catalog_v1::CreateGrouponActivityRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateGrouponActivityRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteGrouponActivityRequest) -> pbjson_types::Empty,
            pass list_groups(catalog_v1::ListGrouponGroupsRequest)
                -> catalog_v1::ListGrouponGroupResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InteractionAdminServiceHandlers`.
    shop_proto::gen_admin::services::InteractionAdminServiceHandlers for InteractionAdminProxy {
        client: interaction_v1::interaction_admin_service_client::InteractionAdminServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass purge_target_interactions(interaction_v1::PurgeTargetInteractionsRequest)
                -> interaction_v1::PurgeTargetInteractionsResponse,
            pass purge_user_interactions(interaction_v1::PurgeUserInteractionsRequest)
                -> interaction_v1::PurgeUserInteractionsResponse,
            pass reset_counter(interaction_v1::ResetCounterRequest)
                -> interaction_v1::ResetCounterResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InternalMessageCategoryServiceHandlers`.
    shop_proto::gen_admin::services::InternalMessageCategoryServiceHandlers
        for InternalMessageCategoryProxy {
        client: internal_message_v1::internal_message_category_service_client::
            InternalMessageCategoryServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest)
                -> internal_message_v1::ListInternalMessageCategoryResponse,
            pass get(internal_message_v1::GetInternalMessageCategoryRequest)
                -> internal_message_v1::InternalMessageCategory,
            pass create(internal_message_v1::CreateInternalMessageCategoryRequest)
                -> pbjson_types::Empty,
            pass update(internal_message_v1::UpdateInternalMessageCategoryRequest)
                -> pbjson_types::Empty,
            pass delete(internal_message_v1::DeleteInternalMessageCategoryRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InternalMessageRecipientServiceHandlers`.
    shop_proto::gen_admin::services::InternalMessageRecipientServiceHandlers
        for InternalMessageRecipientProxy {
        client: internal_message_v1::internal_message_recipient_service_client::
            InternalMessageRecipientServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list_user_inbox(pagination::PagingRequest)
                -> internal_message_v1::ListUserInboxResponse,
            pass delete_notification_from_inbox(
                internal_message_v1::DeleteNotificationFromInboxRequest
            ) -> pbjson_types::Empty,
            pass mark_notification_as_read(internal_message_v1::MarkNotificationAsReadRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InternalMessageServiceHandlers`.
    shop_proto::gen_admin::services::InternalMessageServiceHandlers for InternalMessageProxy {
        client: internal_message_v1::internal_message_service_client::InternalMessageServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list_message(pagination::PagingRequest)
                -> internal_message_v1::ListInternalMessageResponse,
            pass get_message(internal_message_v1::GetInternalMessageRequest)
                -> internal_message_v1::InternalMessage,
            pass update_message(internal_message_v1::UpdateInternalMessageRequest)
                -> pbjson_types::Empty,
            pass delete_message(internal_message_v1::DeleteInternalMessageRequest)
                -> pbjson_types::Empty,
            pass send_message(internal_message_v1::SendMessageRequest)
                -> internal_message_v1::SendMessageResponse,
            pass revoke_message(internal_message_v1::RevokeMessageRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InvoiceServiceHandlers`.
    shop_proto::gen_admin::services::InvoiceServiceHandlers for InvoiceProxy {
        client: invoice_v1::invoice_service_client::InvoiceServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> invoice_v1::ListInvoiceResponse,
            pass count(pagination::PagingRequest) -> invoice_v1::CountInvoiceResponse,
            pass get(invoice_v1::GetInvoiceRequest) -> invoice_v1::Invoice,
            pass create(invoice_v1::CreateInvoiceRequest) -> pbjson_types::Empty,
            pass update(invoice_v1::UpdateInvoiceRequest) -> pbjson_types::Empty,
            pass delete(invoice_v1::DeleteInvoiceRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `LanguageServiceHandlers`.
    shop_proto::gen_admin::services::LanguageServiceHandlers for LanguageProxy {
        client: dict_v1::language_service_client::LanguageServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> dict_v1::ListLanguageResponse,
            pass get(dict_v1::GetLanguageRequest) -> dict_v1::Language,
            pass create(dict_v1::CreateLanguageRequest) -> pbjson_types::Empty,
            pass update(dict_v1::UpdateLanguageRequest) -> pbjson_types::Empty,
            pass delete(dict_v1::DeleteLanguageRequest) -> pbjson_types::Empty,
            pass batch_create(dict_v1::BatchCreateLanguagesRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `LoginAuditLogServiceHandlers`.
    shop_proto::gen_admin::services::LoginAuditLogServiceHandlers for LoginAuditLogProxy {
        client: audit_v1::login_audit_log_service_client::LoginAuditLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> audit_v1::ListLoginAuditLogResponse,
            pass get(audit_v1::GetLoginAuditLogRequest) -> audit_v1::LoginAuditLog,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `LoginPolicyServiceHandlers`.
    shop_proto::gen_admin::services::LoginPolicyServiceHandlers for LoginPolicyProxy {
        client: authentication_v1::login_policy_service_client::LoginPolicyServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> authentication_v1::ListLoginPolicyResponse,
            pass get(authentication_v1::GetLoginPolicyRequest) -> authentication_v1::LoginPolicy,
            pass create(authentication_v1::CreateLoginPolicyRequest) -> pbjson_types::Empty,
            pass update(authentication_v1::UpdateLoginPolicyRequest) -> pbjson_types::Empty,
            pass delete(authentication_v1::DeleteLoginPolicyRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `MemberTierServiceHandlers`.
    shop_proto::gen_admin::services::MemberTierServiceHandlers for MemberTierProxy {
        client: catalog_v1::member_tier_service_client::MemberTierServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListMemberTierResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountMemberTierResponse,
            pass get(catalog_v1::GetMemberTierRequest) -> catalog_v1::MemberTier,
            pass create(catalog_v1::CreateMemberTierRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateMemberTierRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteMemberTierRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `MenuServiceHandlers`.
    shop_proto::gen_admin::services::MenuServiceHandlers for MenuProxy {
        client: permission_v1::menu_service_client::MenuServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> permission_v1::ListMenuResponse,
            pass get(permission_v1::GetMenuRequest) -> permission_v1::Menu,
            pass sync_menus(permission_v1::SyncMenusRequest) -> pbjson_types::Empty,
            pass create(permission_v1::CreateMenuRequest) -> pbjson_types::Empty,
            pass update(permission_v1::UpdateMenuRequest) -> pbjson_types::Empty,
            pass delete(permission_v1::DeleteMenuRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `MfaServiceHandlers`.
    shop_proto::gen_admin::services::MfaServiceHandlers for MfaProxy {
        client: authentication_v1::mfa_service_client::MfaServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            renamed get_m_f_a_status(authentication_v1::GetMfaStatusRequest)
                -> authentication_v1::GetMfaStatusResponse => get_mfa_status,
            pass list_enrolled_methods(authentication_v1::ListEnrolledMethodsRequest)
                -> authentication_v1::ListEnrolledMethodsResponse,
            pass start_enroll_method(authentication_v1::StartEnrollMethodRequest)
                -> authentication_v1::StartEnrollMethodResponse,
            pass confirm_enroll_method(authentication_v1::ConfirmEnrollMethodRequest)
                -> authentication_v1::ConfirmEnrollMethodResponse,
            renamed disable_m_f_a(authentication_v1::DisableMfaRequest)
                -> pbjson_types::Empty => disable_mfa,
            renamed revoke_m_f_a_device(authentication_v1::RevokeMfaDeviceRequest)
                -> pbjson_types::Empty => revoke_mfa_device,
            stub verify_m_f_a_challenge(authentication_v1::VerifyMfaChallengeRequest)
                -> authentication_v1::LoginResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `MonitorAlertServiceHandlers`.
    shop_proto::gen_admin::services::MonitorAlertServiceHandlers for MonitorAlertProxy {
        client: monitor_alert_v1::monitor_alert_service_client::MonitorAlertServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list_monitor_alert_rule(pagination::PagingRequest)
                -> monitor_alert_v1::ListMonitorAlertRuleResponse,
            pass get_monitor_alert_rule(monitor_alert_v1::GetMonitorAlertRuleRequest)
                -> monitor_alert_v1::MonitorAlertRule,
            pass create_monitor_alert_rule(monitor_alert_v1::CreateMonitorAlertRuleRequest)
                -> monitor_alert_v1::MonitorAlertRule,
            pass update_monitor_alert_rule(monitor_alert_v1::UpdateMonitorAlertRuleRequest)
                -> pbjson_types::Empty,
            pass delete_monitor_alert_rule(monitor_alert_v1::DeleteMonitorAlertRuleRequest)
                -> pbjson_types::Empty,
            pass evaluate_monitor_alerts(monitor_alert_v1::EvaluateMonitorAlertsRequest)
                -> monitor_alert_v1::EvaluateMonitorAlertsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `NotificationChannelServiceHandlers`.
    shop_proto::gen_admin::services::NotificationChannelServiceHandlers
        for NotificationChannelProxy {
        client: notification_v1::notification_channel_service_client::
            NotificationChannelServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            stub list_notification_channel(pagination::PagingRequest)
                -> notification_v1::ListNotificationChannelResponse,
            stub get_notification_channel(notification_v1::GetNotificationChannelRequest)
                -> notification_v1::NotificationChannel,
            stub create_notification_channel(notification_v1::CreateNotificationChannelRequest)
                -> notification_v1::NotificationChannel,
            stub update_notification_channel(notification_v1::UpdateNotificationChannelRequest)
                -> notification_v1::NotificationChannel,
            stub delete_notification_channel(notification_v1::DeleteNotificationChannelRequest)
                -> pbjson_types::Empty,
            pass send_test_email(notification_v1::SendTestEmailRequest)
                -> notification_v1::SendTestEmailResponse,
            pass send_test(notification_v1::SendTestRequest) -> pbjson_types::Empty,
            stub list_channel_schemas(pbjson_types::Empty)
                -> notification_v1::ListChannelSchemasResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `NotificationPreferenceServiceHandlers`.
    shop_proto::gen_admin::services::NotificationPreferenceServiceHandlers
        for NotificationPreferenceProxy {
        client: notification_v1::notification_preference_service_client::
            NotificationPreferenceServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            stub get_my_notification_preference(pbjson_types::Empty)
                -> notification_v1::NotificationPreference,
            pass update_my_notification_preference(
                notification_v1::UpdateNotificationPreferenceRequest
            ) -> notification_v1::NotificationPreference,
            stub list_my_notification_categories(pbjson_types::Empty)
                -> notification_v1::ListMyNotificationCategoriesResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `NotificationRuleServiceHandlers`.
    shop_proto::gen_admin::services::NotificationRuleServiceHandlers for NotificationRuleProxy {
        client: notification_v1::notification_rule_service_client::NotificationRuleServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            stub list_notification_rule(pagination::PagingRequest)
                -> notification_v1::ListNotificationRuleResponse,
            stub get_notification_rule(notification_v1::GetNotificationRuleRequest)
                -> notification_v1::NotificationRule,
            stub create_notification_rule(notification_v1::CreateNotificationRuleRequest)
                -> notification_v1::NotificationRule,
            stub update_notification_rule(notification_v1::UpdateNotificationRuleRequest)
                -> notification_v1::NotificationRule,
            stub delete_notification_rule(notification_v1::DeleteNotificationRuleRequest)
                -> pbjson_types::Empty,
            stub test_dispatch_notification(notification_v1::TestDispatchNotificationRequest)
                -> notification_v1::TestDispatchNotificationResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `NotificationServiceHandlers`.
    shop_proto::gen_admin::services::NotificationServiceHandlers for NotificationProxy {
        client: notification_v1::notification_service_client::NotificationServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            stub list_notification_delivery(pagination::PagingRequest)
                -> notification_v1::ListNotificationDeliveryResponse,
            stub get_notification_delivery(notification_v1::GetNotificationDeliveryRequest)
                -> notification_v1::NotificationDelivery,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `NotificationTemplateServiceHandlers`.
    shop_proto::gen_admin::services::NotificationTemplateServiceHandlers
        for NotificationTemplateProxy {
        client: notification_v1::notification_template_service_client::
            NotificationTemplateServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            stub list_notification_template(pagination::PagingRequest)
                -> notification_v1::ListNotificationTemplateResponse,
            stub get_notification_template(notification_v1::GetNotificationTemplateRequest)
                -> notification_v1::NotificationTemplate,
            stub create_notification_template(notification_v1::CreateNotificationTemplateRequest)
                -> notification_v1::NotificationTemplate,
            stub update_notification_template(notification_v1::UpdateNotificationTemplateRequest)
                -> notification_v1::NotificationTemplate,
            stub delete_notification_template(notification_v1::DeleteNotificationTemplateRequest)
                -> pbjson_types::Empty,
            stub render_notification_template(notification_v1::RenderNotificationTemplateRequest)
                -> notification_v1::RenderNotificationTemplateResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `OnlineSessionServiceHandlers`.
    shop_proto::gen_admin::services::OnlineSessionServiceHandlers for OnlineSessionProxy {
        client: online_session_v1::online_session_service_client::OnlineSessionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            stub list_online_session(online_session_v1::ListOnlineSessionRequest)
                -> online_session_v1::ListOnlineSessionResponse,
            stub force_logout_session(online_session_v1::ForceLogoutSessionRequest)
                -> online_session_v1::ForceLogoutSessionResponse,
            stub list_my_online_session(online_session_v1::ListMyOnlineSessionRequest)
                -> online_session_v1::ListOnlineSessionResponse,
            stub revoke_my_online_session(online_session_v1::RevokeMyOnlineSessionRequest)
                -> online_session_v1::RevokeMyOnlineSessionResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `OperationAuditLogServiceHandlers`.
    shop_proto::gen_admin::services::OperationAuditLogServiceHandlers for OperationAuditLogProxy {
        client: audit_v1::operation_audit_log_service_client::OperationAuditLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> audit_v1::ListOperationAuditLogResponse,
            pass get(audit_v1::GetOperationAuditLogRequest) -> audit_v1::OperationAuditLog,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `OrderItemServiceHandlers`.
    shop_proto::gen_admin::services::OrderItemServiceHandlers for OrderItemProxy {
        client: order_v1::order_item_service_client::OrderItemServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> order_v1::ListOrderItemResponse,
            pass count(pagination::PagingRequest) -> order_v1::CountOrderItemResponse,
            pass get(order_v1::GetOrderItemRequest) -> order_v1::OrderItem,
            pass create(order_v1::CreateOrderItemRequest) -> pbjson_types::Empty,
            pass batch_create(order_v1::BatchCreateOrderItemsRequest) -> pbjson_types::Empty,
            pass update(order_v1::UpdateOrderItemRequest) -> pbjson_types::Empty,
            pass delete(order_v1::DeleteOrderItemRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `OrderServiceHandlers`.
    shop_proto::gen_admin::services::OrderServiceHandlers for OrderProxy {
        client: order_v1::order_service_client::OrderServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> order_v1::ListOrderResponse,
            pass count(pagination::PagingRequest) -> order_v1::CountOrderResponse,
            pass get(order_v1::GetOrderRequest) -> order_v1::Order,
            pass update(order_v1::UpdateOrderRequest) -> pbjson_types::Empty,
            pass delete(order_v1::DeleteOrderRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `OrgUnitServiceHandlers`.
    shop_proto::gen_admin::services::OrgUnitServiceHandlers for OrgUnitProxy {
        client: identity_v1::org_unit_service_client::OrgUnitServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> identity_v1::ListOrgUnitResponse,
            pass get(identity_v1::GetOrgUnitRequest) -> identity_v1::OrgUnit,
            pass create(identity_v1::CreateOrgUnitRequest) -> pbjson_types::Empty,
            pass update(identity_v1::UpdateOrgUnitRequest) -> pbjson_types::Empty,
            pass delete(identity_v1::DeleteOrgUnitRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PaymentChannelConfigServiceHandlers`.
    shop_proto::gen_admin::services::PaymentChannelConfigServiceHandlers
        for PaymentChannelConfigProxy {
        client: payment_v1::payment_channel_config_service_client::
            PaymentChannelConfigServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> payment_v1::ListPaymentChannelConfigResponse,
            pass count(pagination::PagingRequest) -> payment_v1::CountPaymentChannelConfigResponse,
            pass get(payment_v1::GetPaymentChannelConfigRequest)
                -> payment_v1::PaymentChannelConfig,
            pass create(payment_v1::CreatePaymentChannelConfigRequest) -> pbjson_types::Empty,
            pass update(payment_v1::UpdatePaymentChannelConfigRequest) -> pbjson_types::Empty,
            pass delete(payment_v1::DeletePaymentChannelConfigRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PaymentRefundServiceHandlers`.
    shop_proto::gen_admin::services::PaymentRefundServiceHandlers for PaymentRefundProxy {
        client: payment_v1::payment_refund_service_client::PaymentRefundServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> payment_v1::ListPaymentRefundResponse,
            pass count(pagination::PagingRequest) -> payment_v1::CountPaymentRefundResponse,
            pass get(payment_v1::GetPaymentRefundRequest) -> payment_v1::PaymentRefund,
            pass create(payment_v1::CreatePaymentRefundRequest) -> pbjson_types::Empty,
            pass update(payment_v1::UpdatePaymentRefundRequest) -> pbjson_types::Empty,
            pass delete(payment_v1::DeletePaymentRefundRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PaymentTransactionServiceHandlers`.
    shop_proto::gen_admin::services::PaymentTransactionServiceHandlers for PaymentTransactionProxy {
        client: payment_v1::payment_transaction_service_client::PaymentTransactionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> payment_v1::ListPaymentTransactionResponse,
            pass count(pagination::PagingRequest) -> payment_v1::CountPaymentTransactionResponse,
            pass get(payment_v1::GetPaymentTransactionRequest) -> payment_v1::PaymentTransaction,
            pass create(payment_v1::CreatePaymentTransactionRequest) -> pbjson_types::Empty,
            pass update(payment_v1::UpdatePaymentTransactionRequest) -> pbjson_types::Empty,
            pass delete(payment_v1::DeletePaymentTransactionRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PermissionAuditLogServiceHandlers`.
    shop_proto::gen_admin::services::PermissionAuditLogServiceHandlers for PermissionAuditLogProxy {
        client: audit_v1::permission_audit_log_service_client::PermissionAuditLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> audit_v1::ListPermissionAuditLogResponse,
            pass get(audit_v1::GetPermissionAuditLogRequest) -> audit_v1::PermissionAuditLog,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PermissionGroupServiceHandlers`.
    shop_proto::gen_admin::services::PermissionGroupServiceHandlers for PermissionGroupProxy {
        client: permission_v1::permission_group_service_client::PermissionGroupServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> permission_v1::ListPermissionGroupResponse,
            pass get(permission_v1::GetPermissionGroupRequest) -> permission_v1::PermissionGroup,
            pass create(permission_v1::CreatePermissionGroupRequest) -> pbjson_types::Empty,
            pass update(permission_v1::UpdatePermissionGroupRequest) -> pbjson_types::Empty,
            pass delete(permission_v1::DeletePermissionGroupRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PermissionServiceHandlers`.
    shop_proto::gen_admin::services::PermissionServiceHandlers for PermissionProxy {
        client: permission_v1::permission_service_client::PermissionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> permission_v1::ListPermissionResponse,
            pass get(permission_v1::GetPermissionRequest) -> permission_v1::Permission,
            pass create(permission_v1::CreatePermissionRequest) -> pbjson_types::Empty,
            pass update(permission_v1::UpdatePermissionRequest) -> pbjson_types::Empty,
            pass delete(permission_v1::DeletePermissionRequest) -> pbjson_types::Empty,
            stub sync_permissions(pbjson_types::Empty) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PlanModuleServiceHandlers`.
    shop_proto::gen_admin::services::PlanModuleServiceHandlers for PlanModuleProxy {
        client: identity_v1::plan_module_service_client::PlanModuleServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> identity_v1::ListPlanModuleResponse,
            pass get(identity_v1::GetPlanModuleRequest) -> identity_v1::PlanModule,
            pass create(identity_v1::CreatePlanModuleRequest) -> pbjson_types::Empty,
            pass update(identity_v1::UpdatePlanModuleRequest) -> pbjson_types::Empty,
            pass delete(identity_v1::DeletePlanModuleRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PlanQuotaServiceHandlers`.
    shop_proto::gen_admin::services::PlanQuotaServiceHandlers for PlanQuotaProxy {
        client: identity_v1::plan_quota_service_client::PlanQuotaServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> identity_v1::ListPlanQuotaResponse,
            pass create(identity_v1::CreatePlanQuotaRequest) -> pbjson_types::Empty,
            pass update(identity_v1::UpdatePlanQuotaRequest) -> pbjson_types::Empty,
            pass delete(identity_v1::DeletePlanQuotaRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PlanServiceHandlers`.
    shop_proto::gen_admin::services::PlanServiceHandlers for PlanProxy {
        client: identity_v1::plan_service_client::PlanServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> identity_v1::ListPlanResponse,
            pass get(identity_v1::GetPlanRequest) -> identity_v1::Plan,
            pass create(identity_v1::CreatePlanRequest) -> pbjson_types::Empty,
            pass update(identity_v1::UpdatePlanRequest) -> pbjson_types::Empty,
            pass delete(identity_v1::DeletePlanRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PointsLedgerServiceHandlers`.
    shop_proto::gen_admin::services::PointsLedgerServiceHandlers for PointsLedgerProxy {
        client: catalog_v1::points_ledger_service_client::PointsLedgerServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListPointsLedgerResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountPointsLedgerResponse,
            pass get(catalog_v1::GetPointsLedgerRequest) -> catalog_v1::PointsLedger,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PolicyEvaluationLogServiceHandlers`.
    shop_proto::gen_admin::services::PolicyEvaluationLogServiceHandlers
        for PolicyEvaluationLogProxy {
        client: permission_v1::policy_evaluation_log_service_client::
            PolicyEvaluationLogServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> permission_v1::ListPolicyEvaluationLogResponse,
            pass get(permission_v1::GetPolicyEvaluationLogRequest)
                -> permission_v1::PolicyEvaluationLog,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PositionServiceHandlers`.
    shop_proto::gen_admin::services::PositionServiceHandlers for PositionProxy {
        client: identity_v1::position_service_client::PositionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> identity_v1::ListPositionResponse,
            pass get(identity_v1::GetPositionRequest) -> identity_v1::Position,
            pass create(identity_v1::CreatePositionRequest) -> pbjson_types::Empty,
            pass update(identity_v1::UpdatePositionRequest) -> pbjson_types::Empty,
            pass delete(identity_v1::DeletePositionRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ProductAttributeServiceHandlers`.
    shop_proto::gen_admin::services::ProductAttributeServiceHandlers for ProductAttributeProxy {
        client: catalog_v1::product_attribute_service_client::ProductAttributeServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListProductAttributeResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountProductAttributeResponse,
            pass get(catalog_v1::GetProductAttributeRequest) -> catalog_v1::ProductAttribute,
            pass create(catalog_v1::CreateProductAttributeRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateProductAttributesRequest)
                -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateProductAttributeRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteProductAttributeRequest) -> pbjson_types::Empty,
            pass translation_exists(catalog_v1::ProductAttributeTranslationExistsRequest)
                -> catalog_v1::ProductAttributeTranslationExistsResponse,
            pass get_translation(catalog_v1::GetProductAttributeRequest)
                -> catalog_v1::ProductAttributeTranslation,
            pass create_translation(catalog_v1::CreateProductAttributeTranslationRequest)
                -> catalog_v1::ProductAttributeTranslation,
            pass update_translation(catalog_v1::UpdateProductAttributeTranslationRequest)
                -> catalog_v1::ProductAttributeTranslation,
            pass delete_translation(catalog_v1::DeleteProductAttributeTranslationRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ProductAttributeValueServiceHandlers`.
    shop_proto::gen_admin::services::ProductAttributeValueServiceHandlers
        for ProductAttributeValueProxy {
        client: catalog_v1::product_attribute_value_service_client::
            ProductAttributeValueServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListProductAttributeValueResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountProductAttributeValueResponse,
            pass get(catalog_v1::GetProductAttributeValueRequest)
                -> catalog_v1::ProductAttributeValue,
            pass create(catalog_v1::CreateProductAttributeValueRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateProductAttributeValuesRequest)
                -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateProductAttributeValueRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteProductAttributeValueRequest) -> pbjson_types::Empty,
            pass translation_exists(catalog_v1::ProductAttributeValueTranslationExistsRequest)
                -> catalog_v1::ProductAttributeValueTranslationExistsResponse,
            pass get_translation(catalog_v1::GetProductAttributeValueRequest)
                -> catalog_v1::ProductAttributeValueTranslation,
            pass create_translation(catalog_v1::CreateProductAttributeValueTranslationRequest)
                -> catalog_v1::ProductAttributeValueTranslation,
            pass update_translation(catalog_v1::UpdateProductAttributeValueTranslationRequest)
                -> catalog_v1::ProductAttributeValueTranslation,
            pass delete_translation(catalog_v1::DeleteProductAttributeValueTranslationRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ProductServiceHandlers`.
    shop_proto::gen_admin::services::ProductServiceHandlers for ProductProxy {
        client: catalog_v1::product_service_client::ProductServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListProductResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountProductResponse,
            pass get(catalog_v1::GetProductRequest) -> catalog_v1::Product,
            pass create(catalog_v1::CreateProductRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateProductsRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateProductRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteProductRequest) -> pbjson_types::Empty,
            pass translation_exists(catalog_v1::ProductTranslationExistsRequest)
                -> catalog_v1::ProductTranslationExistsResponse,
            pass get_translation(catalog_v1::GetProductRequest) -> catalog_v1::ProductTranslation,
            pass create_translation(catalog_v1::CreateProductTranslationRequest)
                -> catalog_v1::ProductTranslation,
            pass update_translation(catalog_v1::UpdateProductTranslationRequest)
                -> catalog_v1::ProductTranslation,
            pass delete_translation(catalog_v1::DeleteProductTranslationRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PromotionServiceHandlers`.
    shop_proto::gen_admin::services::PromotionServiceHandlers for PromotionProxy {
        client: catalog_v1::promotion_service_client::PromotionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListPromotionResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountPromotionResponse,
            pass get(catalog_v1::GetPromotionRequest) -> catalog_v1::Promotion,
            pass create(catalog_v1::CreatePromotionRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdatePromotionRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeletePromotionRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `RedisCacheMonitorServiceHandlers`.
    shop_proto::gen_admin::services::RedisCacheMonitorServiceHandlers for RedisCacheMonitorProxy {
        client: redis_cache_v1::redis_cache_monitor_service_client::RedisCacheMonitorServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass get(redis_cache_v1::GetRedisCacheMonitorRequest)
                -> redis_cache_v1::RedisCacheMonitorInfo,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `RoleServiceHandlers`.
    shop_proto::gen_admin::services::RoleServiceHandlers for RoleProxy {
        client: permission_v1::role_service_client::RoleServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> permission_v1::ListRoleResponse,
            pass get(permission_v1::GetRoleRequest) -> permission_v1::Role,
            pass create(permission_v1::CreateRoleRequest) -> pbjson_types::Empty,
            pass update(permission_v1::UpdateRoleRequest) -> pbjson_types::Empty,
            pass delete(permission_v1::DeleteRoleRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ScriptLogServiceHandlers`.
    shop_proto::gen_admin::services::ScriptLogServiceHandlers for ScriptLogProxy {
        client: script_v1::script_log_service_client::ScriptLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> script_v1::ListScriptLogsResponse,
            pass count(pagination::PagingRequest) -> script_v1::CountScriptLogsResponse,
            pass purge(script_v1::PurgeScriptLogsRequest) -> script_v1::PurgeScriptLogsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ScriptServiceHandlers`.
    shop_proto::gen_admin::services::ScriptServiceHandlers for ScriptProxy {
        client: script_v1::script_service_client::ScriptServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> script_v1::ListScriptsResponse,
            pass count(pagination::PagingRequest) -> script_v1::CountScriptsResponse,
            pass get(script_v1::GetScriptRequest) -> script_v1::Script,
            pass create(script_v1::CreateScriptRequest) -> pbjson_types::Empty,
            pass update(script_v1::UpdateScriptRequest) -> pbjson_types::Empty,
            pass delete(script_v1::DeleteScriptRequest) -> pbjson_types::Empty,
            pass test_run(script_v1::TestRunScriptRequest) -> script_v1::TestRunScriptResponse,
            stub list_hook_points(pbjson_types::Empty) -> script_v1::ListHookPointsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SeckillActivityServiceHandlers`.
    shop_proto::gen_admin::services::SeckillActivityServiceHandlers for SeckillActivityProxy {
        client: catalog_v1::seckill_activity_service_client::SeckillActivityServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListSeckillActivityResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountSeckillActivityResponse,
            pass get(catalog_v1::GetSeckillActivityRequest) -> catalog_v1::SeckillActivity,
            pass create(catalog_v1::CreateSeckillActivityRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateSeckillActivityRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteSeckillActivityRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ServerMonitorServiceHandlers`.
    shop_proto::gen_admin::services::ServerMonitorServiceHandlers for ServerMonitorProxy {
        client: redis_cache_v1::server_monitor_service_client::ServerMonitorServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            stub get(shop_proto::proto::server_monitor::service::v1::GetServerMonitorRequest)
                -> shop_proto::proto::server_monitor::service::v1::ServerMonitorInfo,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ShipmentServiceHandlers`.
    shop_proto::gen_admin::services::ShipmentServiceHandlers for ShipmentProxy {
        client: shipping_v1::shipment_service_client::ShipmentServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> shipping_v1::ListShipmentResponse,
            pass count(pagination::PagingRequest) -> shipping_v1::CountShipmentResponse,
            pass get(shipping_v1::GetShipmentRequest) -> shipping_v1::Shipment,
            pass create(shipping_v1::CreateShipmentRequest) -> pbjson_types::Empty,
            pass update(shipping_v1::UpdateShipmentRequest) -> pbjson_types::Empty,
            pass delete(shipping_v1::DeleteShipmentRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ShippingRateServiceHandlers`.
    shop_proto::gen_admin::services::ShippingRateServiceHandlers for ShippingRateProxy {
        client: shipping_v1::shipping_rate_service_client::ShippingRateServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> shipping_v1::ListShippingRateResponse,
            pass count(pagination::PagingRequest) -> shipping_v1::CountShippingRateResponse,
            pass get(shipping_v1::GetShippingRateRequest) -> shipping_v1::ShippingRate,
            pass create(shipping_v1::CreateShippingRateRequest) -> pbjson_types::Empty,
            pass update(shipping_v1::UpdateShippingRateRequest) -> pbjson_types::Empty,
            pass delete(shipping_v1::DeleteShippingRateRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SkuAttributeCombinationServiceHandlers`.
    shop_proto::gen_admin::services::SkuAttributeCombinationServiceHandlers
        for SkuAttributeCombinationProxy {
        client: catalog_v1::sku_attribute_combination_service_client::
            SkuAttributeCombinationServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListSkuAttributeCombinationResponse,
            pass count(pagination::PagingRequest)
                -> catalog_v1::CountSkuAttributeCombinationResponse,
            pass get(catalog_v1::GetSkuAttributeCombinationRequest)
                -> catalog_v1::SkuAttributeCombination,
            pass create(catalog_v1::CreateSkuAttributeCombinationRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateSkuAttributeCombinationsRequest)
                -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateSkuAttributeCombinationRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteSkuAttributeCombinationRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SkuPriceServiceHandlers`.
    shop_proto::gen_admin::services::SkuPriceServiceHandlers for SkuPriceProxy {
        client: catalog_v1::sku_price_service_client::SkuPriceServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListSkuPriceResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountSkuResponse,
            pass get(catalog_v1::GetSkuPriceRequest) -> catalog_v1::SkuPrice,
            pass create(catalog_v1::CreateSkuPriceRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateSkuPricesRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateSkuPriceRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteSkuPriceRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SkuServiceHandlers`.
    shop_proto::gen_admin::services::SkuServiceHandlers for SkuProxy {
        client: catalog_v1::sku_service_client::SkuServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListSkuResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountSkuResponse,
            pass get(catalog_v1::GetSkuRequest) -> catalog_v1::Sku,
            pass create(catalog_v1::CreateSkuRequest) -> pbjson_types::Empty,
            pass batch_create(catalog_v1::BatchCreateSkusRequest) -> pbjson_types::Empty,
            pass update(catalog_v1::UpdateSkuRequest) -> pbjson_types::Empty,
            pass delete(catalog_v1::DeleteSkuRequest) -> pbjson_types::Empty,
            pass adjust_stock(catalog_v1::AdjustSkuStockRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `StockAlertServiceHandlers`.
    shop_proto::gen_admin::services::StockAlertServiceHandlers for StockAlertProxy {
        client: catalog_v1::stock_alert_service_client::StockAlertServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListStockAlertResponse,
            pass get(catalog_v1::GetStockAlertRequest) -> catalog_v1::StockAlert,
            pass update(catalog_v1::UpdateStockAlertRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `StockOpLogServiceHandlers`.
    shop_proto::gen_admin::services::StockOpLogServiceHandlers for StockOpLogProxy {
        client: catalog_v1::stock_op_log_service_client::StockOpLogServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListStockOpLogResponse,
            pass count(pagination::PagingRequest) -> catalog_v1::CountStockOpLogResponse,
            pass get(catalog_v1::GetStockOpLogRequest) -> catalog_v1::StockOpLog,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `TaskServiceHandlers`.
    shop_proto::gen_admin::services::TaskServiceHandlers for TaskProxy {
        client: task_v1::task_service_client::TaskServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> task_v1::ListTaskResponse,
            pass get(task_v1::GetTaskRequest) -> task_v1::Task,
            pass create(task_v1::CreateTaskRequest) -> pbjson_types::Empty,
            pass update(task_v1::UpdateTaskRequest) -> pbjson_types::Empty,
            pass delete(task_v1::DeleteTaskRequest) -> pbjson_types::Empty,
            stub list_task_type_name(pbjson_types::Empty) -> task_v1::ListTaskTypeNameResponse,
            stub restart_all_task(pbjson_types::Empty) -> task_v1::RestartAllTaskResponse,
            stub start_all_task(pbjson_types::Empty) -> pbjson_types::Empty,
            stub stop_all_task(pbjson_types::Empty) -> pbjson_types::Empty,
            pass control_task(task_v1::ControlTaskRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `TaxRateServiceHandlers`.
    shop_proto::gen_admin::services::TaxRateServiceHandlers for TaxRateProxy {
        client: tax_v1::tax_rate_service_client::TaxRateServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> tax_v1::ListTaxRateResponse,
            pass count(pagination::PagingRequest) -> tax_v1::CountTaxRateResponse,
            pass get(tax_v1::GetTaxRateRequest) -> tax_v1::TaxRate,
            pass create(tax_v1::CreateTaxRateRequest) -> pbjson_types::Empty,
            pass update(tax_v1::UpdateTaxRateRequest) -> pbjson_types::Empty,
            pass delete(tax_v1::DeleteTaxRateRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `TenantServiceHandlers`.
    shop_proto::gen_admin::services::TenantServiceHandlers for TenantProxy {
        client: identity_v1::tenant_service_client::TenantServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> identity_v1::ListTenantResponse,
            pass get(identity_v1::GetTenantRequest) -> identity_v1::Tenant,
            drop create(identity_v1::CreateTenantRequest) -> pbjson_types::Empty,
            pass update(identity_v1::UpdateTenantRequest) -> pbjson_types::Empty,
            pass delete(identity_v1::DeleteTenantRequest) -> pbjson_types::Empty,
            pass create_tenant_with_admin_user(identity_v1::CreateTenantWithAdminUserRequest)
                -> pbjson_types::Empty,
            pass tenant_exists(identity_v1::TenantExistsRequest)
                -> identity_v1::TenantExistsResponse,
            pass get_usage(identity_v1::GetTenantUsageRequest) -> identity_v1::TenantUsage,
            pass cleanup_data(identity_v1::CleanupTenantDataRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `UserCouponServiceHandlers`.
    shop_proto::gen_admin::services::UserCouponServiceHandlers for UserCouponProxy {
        client: coupon_v1::user_coupon_service_client::UserCouponServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> coupon_v1::ListUserCouponResponse,
            pass count(pagination::PagingRequest) -> coupon_v1::CountUserCouponResponse,
            pass get(coupon_v1::GetUserCouponRequest) -> coupon_v1::UserCoupon,
            pass create(coupon_v1::CreateUserCouponRequest) -> pbjson_types::Empty,
            pass delete(coupon_v1::DeleteUserCouponRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `UserProfileServiceHandlers`.
    shop_proto::gen_admin::services::UserProfileServiceHandlers for UserProfileProxy {
        client: identity_v1::user_profile_service_client::UserProfileServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: behaviors,
        methods: [
            stub get_user(pbjson_types::Empty) -> identity_v1::User,
            pass update_user(identity_v1::UpdateUserRequest) -> pbjson_types::Empty,
            stub change_password(identity_v1::ChangePasswordRequest) -> pbjson_types::Empty,
            stub upload_avatar(identity_v1::UploadAvatarRequest)
                -> identity_v1::UploadAvatarResponse,
            stub delete_avatar(pbjson_types::Empty) -> pbjson_types::Empty,
            stub bind_contact(identity_v1::BindContactRequest) -> pbjson_types::Empty,
            stub verify_contact(identity_v1::VerifyContactRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `UserServiceHandlers`.
    shop_proto::gen_admin::services::UserServiceHandlers for UserProxy {
        client: identity_v1::user_service_client::UserServiceClient<tonic::transport::Channel>,
        behaviors: behaviors,
        methods: [
            pass list(pagination::PagingRequest) -> identity_v1::ListUserResponse,
            pass get(identity_v1::GetUserRequest) -> identity_v1::User,
            pass create(identity_v1::CreateUserRequest) -> identity_v1::User,
            pass update(identity_v1::UpdateUserRequest) -> pbjson_types::Empty,
            pass delete(identity_v1::DeleteUserRequest) -> pbjson_types::Empty,
            pass user_exists(identity_v1::UserExistsRequest) -> identity_v1::UserExistsResponse,
            stub edit_user_password(identity_v1::EditUserPasswordRequest) -> pbjson_types::Empty,
        ]
    }
}
