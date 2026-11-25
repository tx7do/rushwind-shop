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
use shop_proto::proto::address::service::v1 as address_v1;
use shop_proto::proto::ai::service::v1 as ai_v1;
use shop_proto::proto::authentication::service::v1 as authentication_v1;
use shop_proto::proto::cart::service::v1 as cart_v1;
use shop_proto::proto::catalog::service::v1 as catalog_v1;
use shop_proto::proto::comment::service::v1 as comment_v1;
use shop_proto::proto::coupon::service::v1 as coupon_v1;
use shop_proto::proto::identity::service::v1 as identity_v1;
use shop_proto::proto::interaction::service::v1 as interaction_v1;
use shop_proto::proto::internal_message::service::v1 as internal_message_v1;
use shop_proto::proto::invoice::service::v1 as invoice_v1;
use shop_proto::proto::order::service::v1 as order_v1;
use shop_proto::proto::pagination;
use shop_proto::proto::payment::service::v1 as payment_v1;
use shop_proto::proto::shipping::service::v1 as shipping_v1;
use shop_proto::proto::wishlist::service::v1 as wishlist_v1;

passthrough_proxy! {
    /// The pass-through proxy of `AiChatServiceHandlers`.
    shop_proto::gen_app::services::AiChatServiceHandlers for AiChatProxy {
        client: ai_v1::ai_chat_service_client::AiChatServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            stub chat(ai_v1::ChatRequest) -> ai_v1::ChatResponse,
            stub rewrite_search_query(
                shop_proto::proto::app::service::v1::RewriteSearchQueryRequest
            ) -> shop_proto::proto::app::service::v1::RewriteSearchQueryResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `AuthenticationServiceHandlers`.
    shop_proto::gen_app::services::AuthenticationServiceHandlers for AuthenticationProxy {
        client: authentication_v1::authentication_service_client::AuthenticationServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass login(authentication_v1::LoginRequest) -> authentication_v1::LoginResponse,
            stub register(authentication_v1::RegisterUserRequest)
                -> authentication_v1::RegisterUserResponse,
            stub logout(pbjson_types::Empty) -> pbjson_types::Empty,
            pass refresh_token(authentication_v1::LoginRequest) -> authentication_v1::LoginResponse,
            stub send_reset_code(shop_proto::proto::app::service::v1::SendResetCodeRequest)
                -> shop_proto::proto::app::service::v1::SendResetCodeResponse,
            stub reset_password(shop_proto::proto::app::service::v1::ResetPasswordRequest)
                -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `BrandServiceHandlers`.
    shop_proto::gen_app::services::BrandServiceHandlers for BrandProxy {
        client: catalog_v1::brand_service_client::BrandServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListBrandResponse,
            pass get(catalog_v1::GetBrandRequest) -> catalog_v1::Brand,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CartItemServiceHandlers`.
    shop_proto::gen_app::services::CartItemServiceHandlers for CartItemProxy {
        client: cart_v1::cart_item_service_client::CartItemServiceClient<tonic::transport::Channel>,
        behaviors: public,
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
    shop_proto::gen_app::services::CartServiceHandlers for CartProxy {
        client: cart_v1::cart_service_client::CartServiceClient<tonic::transport::Channel>,
        behaviors: public,
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
    shop_proto::gen_app::services::CategoryServiceHandlers for CategoryProxy {
        client: catalog_v1::category_service_client::CategoryServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListCategoryResponse,
            pass get(catalog_v1::GetCategoryRequest) -> catalog_v1::Category,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CommentServiceHandlers`.
    shop_proto::gen_app::services::CommentServiceHandlers for CommentProxy {
        client: comment_v1::comment_service_client::CommentServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> comment_v1::ListCommentResponse,
            pass get(comment_v1::GetCommentRequest) -> comment_v1::Comment,
            pass create(comment_v1::CreateCommentRequest) -> pbjson_types::Empty,
            pass update(comment_v1::UpdateCommentRequest) -> pbjson_types::Empty,
            pass delete(comment_v1::DeleteCommentRequest) -> pbjson_types::Empty,
            pass get_product_rating(comment_v1::GetProductRatingRequest)
                -> comment_v1::ProductRatingSummary,
            stub get_product_review_summary(
                shop_proto::proto::app::service::v1::GetProductReviewSummaryRequest
            ) -> shop_proto::proto::app::service::v1::ProductReviewSummaryResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `CouponTemplateServiceHandlers`.
    shop_proto::gen_app::services::CouponTemplateServiceHandlers for CouponTemplateProxy {
        client: coupon_v1::coupon_template_service_client::CouponTemplateServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> coupon_v1::ListCouponTemplateResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `GrouponServiceHandlers`.
    shop_proto::gen_app::services::GrouponServiceHandlers for GrouponProxy {
        client: catalog_v1::groupon_activity_service_client::GrouponActivityServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list_active_groupons(catalog_v1::ListActiveGrouponsRequest)
                -> catalog_v1::ListGrouponActivityResponse,
            pass get_group(catalog_v1::GetGrouponGroupRequest) -> catalog_v1::GrouponGroup,
            pass list_groups(catalog_v1::ListGrouponGroupsRequest)
                -> catalog_v1::ListGrouponGroupResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InteractionServiceHandlers`.
    shop_proto::gen_app::services::InteractionServiceHandlers for InteractionProxy {
        client: interaction_v1::interaction_service_client::InteractionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass like(interaction_v1::LikeRequest) -> interaction_v1::LikeResponse,
            pass unlike(interaction_v1::LikeRequest) -> interaction_v1::LikeResponse,
            pass get_interaction_status(interaction_v1::GetInteractionStatusRequest)
                -> interaction_v1::GetInteractionStatusResponse,
            pass get_counts(interaction_v1::GetCountsRequest) -> interaction_v1::GetCountsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InternalMessageRecipientServiceHandlers`.
    shop_proto::gen_app::services::InternalMessageRecipientServiceHandlers
        for InternalMessageRecipientProxy {
        client: internal_message_v1::internal_message_recipient_service_client::
            InternalMessageRecipientServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list_user_inbox(pagination::PagingRequest)
                -> internal_message_v1::ListUserInboxResponse,
            pass mark_notification_as_read(internal_message_v1::MarkNotificationAsReadRequest)
                -> pbjson_types::Empty,
            pass delete_notification_from_inbox(
                internal_message_v1::DeleteNotificationFromInboxRequest
            ) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `InvoiceServiceHandlers`.
    shop_proto::gen_app::services::InvoiceServiceHandlers for InvoiceProxy {
        client: invoice_v1::invoice_service_client::InvoiceServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> invoice_v1::ListInvoiceResponse,
            pass get(invoice_v1::GetInvoiceRequest) -> invoice_v1::Invoice,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `MemberTierServiceHandlers`.
    shop_proto::gen_app::services::MemberTierServiceHandlers for MemberTierProxy {
        client: catalog_v1::member_tier_service_client::MemberTierServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            hand get_my_tier_info(catalog_v1::GetUserTierInfoRequest)
                -> catalog_v1::GetUserTierInfoResponse => get_my_tier_info,
            hand list_tiers(catalog_v1::ListActiveMemberTiersRequest)
                -> catalog_v1::ListMemberTierResponse => list_tiers,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `OrderItemServiceHandlers`.
    shop_proto::gen_app::services::OrderItemServiceHandlers for OrderItemProxy {
        client: order_v1::order_item_service_client::OrderItemServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> order_v1::ListOrderItemResponse,
            pass count(pagination::PagingRequest) -> order_v1::CountOrderItemResponse,
            pass get(order_v1::GetOrderItemRequest) -> order_v1::OrderItem,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `OrderServiceHandlers`.
    shop_proto::gen_app::services::OrderServiceHandlers for OrderProxy {
        client: order_v1::order_service_client::OrderServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> order_v1::ListOrderResponse,
            pass count(pagination::PagingRequest) -> order_v1::CountOrderResponse,
            pass get(order_v1::GetOrderRequest) -> order_v1::Order,
            pass create(order_v1::CreateOrderRequest) -> pbjson_types::Empty,
            pass update(order_v1::UpdateOrderRequest) -> pbjson_types::Empty,
            pass cancel_order(order_v1::CancelOrderRequest) -> pbjson_types::Empty,
            pass confirm_receipt(order_v1::ConfirmReceiptRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PaymentChannelServiceHandlers`.
    shop_proto::gen_app::services::PaymentChannelServiceHandlers for PaymentChannelProxy {
        client: payment_v1::payment_channel_config_service_client::
            PaymentChannelConfigServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list_enabled_channels(payment_v1::ListEnabledChannelsRequest)
                -> payment_v1::ListEnabledChannelsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PaymentRefundServiceHandlers`.
    shop_proto::gen_app::services::PaymentRefundServiceHandlers for PaymentRefundProxy {
        client: payment_v1::payment_refund_service_client::PaymentRefundServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> payment_v1::ListPaymentRefundResponse,
            pass count(pagination::PagingRequest) -> payment_v1::CountPaymentRefundResponse,
            pass get(payment_v1::GetPaymentRefundRequest) -> payment_v1::PaymentRefund,
            pass create(payment_v1::CreatePaymentRefundRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PaymentTransactionServiceHandlers`.
    shop_proto::gen_app::services::PaymentTransactionServiceHandlers for PaymentTransactionProxy {
        client: payment_v1::payment_transaction_service_client::PaymentTransactionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> payment_v1::ListPaymentTransactionResponse,
            pass count(pagination::PagingRequest) -> payment_v1::CountPaymentTransactionResponse,
            pass get(payment_v1::GetPaymentTransactionRequest) -> payment_v1::PaymentTransaction,
            pass create(payment_v1::CreatePaymentTransactionRequest) -> pbjson_types::Empty,
            hand create_payment_intent(payment_v1::CreatePaymentIntentRequest)
                -> payment_v1::CreatePaymentIntentAppResponse => create_payment_intent,
            hand mock_confirm_payment(payment_v1::MockConfirmPaymentRequest)
                -> payment_v1::MockConfirmPaymentResponse => mock_confirm_payment,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PointsServiceHandlers`.
    shop_proto::gen_app::services::PointsServiceHandlers for PointsProxy {
        client: catalog_v1::points_ledger_service_client::PointsLedgerServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            renamed get_my_points_balance(catalog_v1::GetUserPointsBalanceRequest)
                -> catalog_v1::GetUserPointsBalanceResponse => get_user_points_balance,
            renamed list_my_points_ledgers(pagination::PagingRequest)
                -> catalog_v1::ListPointsLedgerResponse => list,
            pass check_in(catalog_v1::CheckInRequest) -> catalog_v1::CheckInResponse,
            pass get_check_in_status(catalog_v1::GetCheckInStatusRequest)
                -> catalog_v1::GetCheckInStatusResponse,
            pass exchange_points_for_coupon(catalog_v1::ExchangePointsForCouponRequest)
                -> catalog_v1::ExchangePointsForCouponResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ProductAttributeServiceHandlers`.
    shop_proto::gen_app::services::ProductAttributeServiceHandlers for ProductAttributeProxy {
        client: catalog_v1::product_attribute_service_client::ProductAttributeServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListProductAttributeResponse,
            pass get(catalog_v1::GetProductAttributeRequest) -> catalog_v1::ProductAttribute,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ProductAttributeValueServiceHandlers`.
    shop_proto::gen_app::services::ProductAttributeValueServiceHandlers
        for ProductAttributeValueProxy {
        client: catalog_v1::product_attribute_value_service_client::
            ProductAttributeValueServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListProductAttributeValueResponse,
            pass get(catalog_v1::GetProductAttributeValueRequest)
                -> catalog_v1::ProductAttributeValue,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ProductServiceHandlers`.
    shop_proto::gen_app::services::ProductServiceHandlers for ProductProxy {
        client: catalog_v1::product_service_client::ProductServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListProductResponse,
            pass get(catalog_v1::GetProductRequest) -> catalog_v1::Product,
            pass search_products(catalog_v1::SearchProductsRequest)
                -> catalog_v1::SearchProductsResponse,
            pass search_product_suggestions(catalog_v1::SearchProductSuggestionsRequest)
                -> catalog_v1::SearchProductSuggestionsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `PromotionServiceHandlers`.
    shop_proto::gen_app::services::PromotionServiceHandlers for PromotionProxy {
        client: catalog_v1::promotion_service_client::PromotionServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list_active_promotions(catalog_v1::ListActivePromotionsRequest)
                -> catalog_v1::ListPromotionResponse,
            pass quote_order_promotions(catalog_v1::QuoteOrderPromotionsRequest)
                -> catalog_v1::QuoteOrderPromotionsResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SeckillServiceHandlers`.
    shop_proto::gen_app::services::SeckillServiceHandlers for SeckillProxy {
        client: catalog_v1::seckill_activity_service_client::SeckillActivityServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list_active_seckills(catalog_v1::ListActiveSeckillsRequest)
                -> catalog_v1::ListSeckillActivityResponse,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ShipmentServiceHandlers`.
    shop_proto::gen_app::services::ShipmentServiceHandlers for ShipmentProxy {
        client: shipping_v1::shipment_service_client::ShipmentServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> shipping_v1::ListShipmentResponse,
            pass get(shipping_v1::GetShipmentRequest) -> shipping_v1::Shipment,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `ShippingAddressServiceHandlers`.
    shop_proto::gen_app::services::ShippingAddressServiceHandlers for ShippingAddressProxy {
        client: address_v1::shipping_address_service_client::ShippingAddressServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> address_v1::ListShippingAddressResponse,
            pass count(pagination::PagingRequest) -> address_v1::CountShippingAddressResponse,
            pass get(address_v1::GetShippingAddressRequest) -> address_v1::ShippingAddress,
            pass create(address_v1::CreateShippingAddressRequest) -> pbjson_types::Empty,
            pass update(address_v1::UpdateShippingAddressRequest) -> pbjson_types::Empty,
            pass delete(address_v1::DeleteShippingAddressRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SkuAttributeCombinationServiceHandlers`.
    shop_proto::gen_app::services::SkuAttributeCombinationServiceHandlers
        for SkuAttributeCombinationProxy {
        client: catalog_v1::sku_attribute_combination_service_client::
            SkuAttributeCombinationServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListSkuAttributeCombinationResponse,
            pass get(catalog_v1::GetSkuAttributeCombinationRequest)
                -> catalog_v1::SkuAttributeCombination,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SkuPriceServiceHandlers`.
    shop_proto::gen_app::services::SkuPriceServiceHandlers for SkuPriceProxy {
        client: catalog_v1::sku_price_service_client::SkuPriceServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListSkuPriceResponse,
            pass get(catalog_v1::GetSkuPriceRequest) -> catalog_v1::SkuPrice,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `SkuServiceHandlers`.
    shop_proto::gen_app::services::SkuServiceHandlers for SkuProxy {
        client: catalog_v1::sku_service_client::SkuServiceClient<tonic::transport::Channel>,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> catalog_v1::ListSkuResponse,
            pass get(catalog_v1::GetSkuRequest) -> catalog_v1::Sku,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `UserCouponServiceHandlers`.
    shop_proto::gen_app::services::UserCouponServiceHandlers for UserCouponProxy {
        client: coupon_v1::user_coupon_service_client::UserCouponServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> coupon_v1::ListUserCouponResponse,
            pass get(coupon_v1::GetUserCouponRequest) -> coupon_v1::UserCoupon,
            pass quote(coupon_v1::QuoteRequest) -> coupon_v1::QuoteResponse,
            pass claim(coupon_v1::ClaimCouponRequest) -> pbjson_types::Empty,
        ]
    }
}

passthrough_proxy! {
    /// The pass-through proxy of `UserProfileServiceHandlers`.
    shop_proto::gen_app::services::UserProfileServiceHandlers for UserProfileProxy {
        client: identity_v1::user_profile_service_client::UserProfileServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass get_user(pbjson_types::Empty) -> identity_v1::User,
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
    /// The pass-through proxy of `WishlistServiceHandlers`.
    shop_proto::gen_app::services::WishlistServiceHandlers for WishlistProxy {
        client: wishlist_v1::wishlist_service_client::WishlistServiceClient<
            tonic::transport::Channel,
        >,
        behaviors: public,
        methods: [
            pass list(pagination::PagingRequest) -> wishlist_v1::ListWishlistResponse,
            pass create(wishlist_v1::CreateWishlistRequest) -> pbjson_types::Empty,
            pass delete(wishlist_v1::DeleteWishlistRequest) -> pbjson_types::Empty,
        ]
    }
}
