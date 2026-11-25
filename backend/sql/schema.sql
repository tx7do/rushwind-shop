-- The golden DDL of the Shop database — dumped style matches the Go
-- reference's Ent-produced tables (mall_* prefix), applied statement by
-- statement on a fresh database by the store bootstrap. The bootstrap
-- splits on line-ending semicolons, so the file stays plain DDL — no
-- functions, no DO blocks (constraint guards ride the IF NOT EXISTS
-- forms). Domains accumulate here as they land (brand first).

CREATE TABLE IF NOT EXISTS public.mall_brands (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    sort_order integer NULL,
    logo_url character varying(500) NULL,
    CONSTRAINT mall_brands_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_brand_translations (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    brand_id bigint NOT NULL,
    language_code varchar(20) NOT NULL,
    name varchar(255) NOT NULL,
    slug varchar(255) NULL,
    description text NULL,
    CONSTRAINT mall_brand_translations_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_brand_translations_brand
        FOREIGN KEY (brand_id)
        REFERENCES public.mall_brands (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_brand_translations_brand_language
    ON public.mall_brand_translations (brand_id, language_code);

CREATE TABLE IF NOT EXISTS public.sys_users (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    username character varying(191) NOT NULL,
    nickname character varying(191) NULL,
    realname character varying(191) NULL,
    email character varying(191) NULL,
    mobile character varying(20) NULL,
    region character varying(50) NULL,
    password character varying(255) NULL,
    status character varying(20) NULL DEFAULT 'NORMAL',
    CONSTRAINT sys_users_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sys_users_username
    ON public.sys_users (username);

CREATE TABLE IF NOT EXISTS public.mall_categories (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    sort_order integer NULL,
    path character varying(191) NULL,
    depth integer NULL,
    image_url character varying(500) NULL,
    parent_id bigint NULL,
    CONSTRAINT mall_categories_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_category_translations (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    category_id bigint NOT NULL,
    language_code varchar(20) NOT NULL,
    name varchar(255) NOT NULL,
    slug varchar(255) NULL,
    description text NULL,
    full_path varchar(500) NULL,
    CONSTRAINT mall_category_translations_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_category_translations_category
        FOREIGN KEY (category_id)
        REFERENCES public.mall_categories (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_category_translations_cat_language
    ON public.mall_category_translations (category_id, language_code);

CREATE TABLE IF NOT EXISTS public.mall_carts (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NOT NULL,
    CONSTRAINT mall_carts_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_carts_user
    ON public.mall_carts (user_id);

CREATE TABLE IF NOT EXISTS public.mall_cart_items (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    cart_id bigint NOT NULL,
    user_id bigint NULL,
    sku_id bigint NOT NULL,
    quantity integer NOT NULL DEFAULT 1,
    CONSTRAINT mall_cart_items_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_cart_items_cart
        FOREIGN KEY (cart_id)
        REFERENCES public.mall_carts (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_cart_items_cart
    ON public.mall_cart_items (cart_id);

CREATE TABLE IF NOT EXISTS public.mall_products (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    sort_order integer NULL,
    status varchar(40) NULL DEFAULT 'PRODUCT_STATUS_ACTIVE',
    category_id bigint NULL,
    brand_id bigint NULL,
    image_url character varying(500) NULL,
    CONSTRAINT mall_products_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_product_translations (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    product_id bigint NOT NULL,
    language_code varchar(20) NOT NULL,
    name varchar(255) NOT NULL,
    slug varchar(255) NULL,
    short_description text NULL,
    long_description text NULL,
    CONSTRAINT mall_product_translations_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_product_translations_product
        FOREIGN KEY (product_id)
        REFERENCES public.mall_products (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_product_translations_product_language
    ON public.mall_product_translations (product_id, language_code);

CREATE TABLE IF NOT EXISTS public.mall_skus (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    product_id bigint NOT NULL,
    sku_code varchar(100) NOT NULL,
    stock_qty integer NOT NULL DEFAULT 0,
    CONSTRAINT mall_skus_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_sku_prices (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    sku_id bigint NOT NULL,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    amount varchar(20) NOT NULL,
    CONSTRAINT mall_sku_prices_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_sku_prices_sku
        FOREIGN KEY (sku_id)
        REFERENCES public.mall_skus (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS public.mall_sku_attribute_combinations (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    sku_id bigint NOT NULL,
    attribute_id bigint NOT NULL,
    attribute_value_id bigint NULL,
    CONSTRAINT mall_sku_attribute_combinations_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_sku_combinations_sku
        FOREIGN KEY (sku_id)
        REFERENCES public.mall_skus (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS public.mall_orders (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    business_ref_id varchar(100) NULL,
    idempotency_key varchar(100) NULL,
    user_id bigint NOT NULL,
    total_amount bigint NOT NULL DEFAULT 0,
    original_amount bigint NULL DEFAULT 0,
    discount_amount bigint NULL DEFAULT 0,
    shipping_fee bigint NULL DEFAULT 0,
    tax_amount bigint NULL DEFAULT 0,
    shipping_region varchar(100) NULL,
    status varchar(20) NOT NULL DEFAULT 'PENDING_PAYMENT',
    recipient_name varchar(191) NULL,
    recipient_phone varchar(30) NULL,
    shipping_address text NULL,
    groupon_activity_id bigint NULL,
    groupon_group_id bigint NULL,
    CONSTRAINT mall_orders_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_orders_user ON public.mall_orders (user_id);

CREATE TABLE IF NOT EXISTS public.mall_order_items (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    order_id bigint NOT NULL,
    user_id bigint NULL,
    sku_id bigint NOT NULL,
    sku_snapshot jsonb NULL,
    quantity integer NOT NULL DEFAULT 1,
    unit_price bigint NULL DEFAULT 0,
    subtotal bigint NULL DEFAULT 0,
    CONSTRAINT mall_order_items_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_order_items_order
        FOREIGN KEY (order_id)
        REFERENCES public.mall_orders (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_order_items_order ON public.mall_order_items (order_id);

CREATE TABLE IF NOT EXISTS public.mall_payment_transactions (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    business_ref_id varchar(100) NULL,
    idempotency_key varchar(100) NULL,
    payment_method varchar(50) NULL,
    business_type varchar(50) NULL,
    order_id bigint NOT NULL,
    user_id bigint NULL,
    amount bigint NOT NULL DEFAULT 0,
    status varchar(20) NOT NULL DEFAULT 'PENDING',
    gateway_trade_no varchar(100) NULL,
    CONSTRAINT mall_payment_transactions_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_payment_tx_order ON public.mall_payment_transactions (order_id);

CREATE TABLE IF NOT EXISTS public.mall_payment_refunds (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    transaction_id bigint NOT NULL,
    amount bigint NOT NULL DEFAULT 0,
    status varchar(20) NOT NULL DEFAULT 'PENDING',
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    business_ref_id varchar(100) NULL,
    idempotency_key varchar(100) NULL,
    user_id bigint NULL,
    CONSTRAINT mall_payment_refunds_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_payment_channel_configs (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(191) NOT NULL,
    channel varchar(50) NOT NULL,
    app_id varchar(191) NULL,
    mch_id varchar(191) NULL,
    sandbox boolean NULL DEFAULT true,
    enabled boolean NULL DEFAULT true,
    credentials text NULL,
    remark text NULL,
    CONSTRAINT mall_payment_channel_configs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_coupon_templates (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    discount_type varchar(30) NOT NULL DEFAULT 'FIXED_AMOUNT',
    discount_value bigint NULL DEFAULT 0,
    discount_percentage int NULL DEFAULT 0,
    valid_from timestamptz NULL,
    valid_until timestamptz NULL,
    max_redemptions int NULL DEFAULT 0,
    max_redemptions_per_user int NULL DEFAULT 1,
    redeemed_count int NULL DEFAULT 0,
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    claimable boolean NULL DEFAULT true,
    points_price bigint NULL DEFAULT 0,
    CONSTRAINT mall_coupon_templates_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_user_coupons (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NOT NULL,
    coupon_template_id bigint NOT NULL,
    status varchar(20) NOT NULL DEFAULT 'UNUSED',
    redeemed_at timestamptz NULL,
    redeemed_order_id bigint NULL,
    applied_discount_amount bigint NULL DEFAULT 0,
    CONSTRAINT mall_user_coupons_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_user_coupons_user ON public.mall_user_coupons (user_id);

CREATE TABLE IF NOT EXISTS public.mall_shipping_rates (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    region varchar(100) NOT NULL,
    base_fee bigint NOT NULL DEFAULT 0,
    per_unit_fee bigint NULL DEFAULT 0,
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    CONSTRAINT mall_shipping_rates_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_shipments (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    order_id bigint NOT NULL,
    user_id bigint NULL,
    carrier varchar(100) NULL,
    tracking_number varchar(100) NULL,
    status varchar(20) NOT NULL DEFAULT 'PENDING',
    tracking_events text NULL,
    CONSTRAINT mall_shipments_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_shipments_order ON public.mall_shipments (order_id);

CREATE TABLE IF NOT EXISTS public.mall_shipping_addresses (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NOT NULL,
    recipient_name varchar(191) NOT NULL,
    recipient_phone varchar(30) NOT NULL,
    region varchar(100) NULL,
    detail_address text NULL,
    postal_code varchar(20) NULL,
    tag varchar(50) NULL,
    is_default boolean NULL DEFAULT false,
    CONSTRAINT mall_shipping_addresses_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_shipping_addresses_user
    ON public.mall_shipping_addresses (user_id);

CREATE TABLE IF NOT EXISTS public.mall_groupon_activities (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(191) NOT NULL,
    sku_id bigint NOT NULL,
    group_price bigint NOT NULL DEFAULT 0,
    group_size int NOT NULL DEFAULT 2,
    group_valid_hours int NULL DEFAULT 24,
    per_user_limit int NULL DEFAULT 1,
    valid_from timestamptz NULL,
    valid_until timestamptz NULL,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    CONSTRAINT mall_groupon_activities_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_groupon_groups (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    activity_id bigint NOT NULL,
    leader_order_id bigint NULL,
    member_count int NULL DEFAULT 0,
    paid_count int NULL DEFAULT 0,
    target_size int NOT NULL DEFAULT 2,
    expire_at timestamptz NULL,
    status varchar(20) NOT NULL DEFAULT 'OPENING',
    CONSTRAINT mall_groupon_groups_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_groupon_groups_activity
        FOREIGN KEY (activity_id)
        REFERENCES public.mall_groupon_activities (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS public.mall_wishlist (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NOT NULL,
    product_id bigint NOT NULL,
    CONSTRAINT mall_wishlist_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_wishlist_user ON public.mall_wishlist (user_id);

CREATE TABLE IF NOT EXISTS public.mall_comments (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    content_type varchar(40) NOT NULL DEFAULT 'CONTENT_TYPE_PRODUCT',
    object_id bigint NOT NULL,
    content text NULL,
    status varchar(30) NOT NULL DEFAULT 'STATUS_APPROVED',
    rating int NULL DEFAULT 5,
    parent_id bigint NULL,
    CONSTRAINT mall_comments_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_comments_object
    ON public.mall_comments (content_type, object_id);

CREATE TABLE IF NOT EXISTS public.mall_interaction_likes (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NOT NULL,
    target_type varchar(40) NOT NULL DEFAULT 'TARGET_TYPE_COMMENT',
    target_id bigint NOT NULL,
    CONSTRAINT mall_interaction_likes_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_interaction_likes_unique
    ON public.mall_interaction_likes (user_id, target_type, target_id);

CREATE TABLE IF NOT EXISTS public.mall_interaction_counters (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    target_type varchar(40) NOT NULL,
    target_id bigint NOT NULL,
    metric varchar(40) NOT NULL DEFAULT 'COUNTER_METRIC_LIKE',
    count bigint NOT NULL DEFAULT 0,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    CONSTRAINT mall_interaction_counters_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_interaction_counters_unique
    ON public.mall_interaction_counters (target_type, target_id, metric);

CREATE TABLE IF NOT EXISTS public.mall_notification_channels (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(191) NOT NULL,
    type varchar(30) NOT NULL DEFAULT 'EMAIL',
    smtp_host varchar(191) NULL,
    smtp_port int NULL,
    smtp_username varchar(191) NULL,
    smtp_password text NULL,
    smtp_from varchar(191) NULL,
    smtp_tls varchar(20) NULL,
    smtp_from_name varchar(191) NULL,
    webhook_url text NULL,
    webhook_secret text NULL,
    CONSTRAINT mall_notification_channels_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_notification_rules (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    event_type varchar(50) NOT NULL,
    channel varchar(30) NOT NULL DEFAULT 'CHANNEL_UNSPECIFIED',
    is_async boolean NULL DEFAULT true,
    is_enabled boolean NULL DEFAULT true,
    remark text NULL,
    CONSTRAINT mall_notification_rules_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_notification_templates (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(191) NOT NULL,
    code varchar(100) NOT NULL,
    title_template text NULL,
    content_template text NULL,
    is_enabled boolean NULL DEFAULT true,
    remark text NULL,
    CONSTRAINT mall_notification_templates_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_notification_deliveries (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    event_type varchar(50) NOT NULL,
    channel varchar(30) NOT NULL,
    channel_id bigint NULL,
    recipient_user_id bigint NULL,
    target varchar(191) NULL,
    status varchar(20) NOT NULL DEFAULT 'SENDING',
    last_error text NULL,
    sent_at timestamptz NULL,
    related_id bigint NULL,
    request_id varchar(100) NULL,
    attempts int NULL DEFAULT 0,
    receipt_provider_id varchar(191) NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    CONSTRAINT mall_notification_deliveries_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_notification_deliveries_recipient
    ON public.mall_notification_deliveries (recipient_user_id);
