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

CREATE TABLE IF NOT EXISTS public.sys_api_audit_logs (
    id bigserial,
    created_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NULL,
    username varchar(191) NULL,
    ip_address varchar(64) NULL,
    http_method varchar(10) NULL,
    path varchar(500) NULL,
    request_uri varchar(500) NULL,
    api_module varchar(100) NULL,
    status_code int NULL,
    payload jsonb NULL,
    CONSTRAINT sys_api_audit_logs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_login_audit_logs (
    id bigserial,
    created_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NULL,
    username varchar(191) NULL,
    ip_address varchar(64) NULL,
    session_id varchar(191) NULL,
    request_id varchar(100) NULL,
    trace_id varchar(100) NULL,
    action_type varchar(30) NULL,
    status varchar(20) NULL,
    failure_reason text NULL,
    payload jsonb NULL,
    CONSTRAINT sys_login_audit_logs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_operation_audit_logs (
    id bigserial,
    created_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NULL,
    username varchar(191) NULL,
    resource_type varchar(100) NULL,
    resource_id varchar(100) NULL,
    action varchar(30) NULL,
    before_data jsonb NULL,
    after_data jsonb NULL,
    sensitive_level varchar(30) NULL,
    request_id varchar(100) NULL,
    payload jsonb NULL,
    CONSTRAINT sys_operation_audit_logs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_data_access_audit_logs (
    id bigserial,
    created_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NULL,
    username varchar(191) NULL,
    ip_address varchar(64) NULL,
    request_id varchar(100) NULL,
    data_source varchar(100) NULL,
    table_name varchar(191) NULL,
    data_id varchar(100) NULL,
    access_type varchar(30) NULL,
    sql_digest varchar(191) NULL,
    payload jsonb NULL,
    CONSTRAINT sys_data_access_audit_logs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_permission_audit_logs (
    id bigserial,
    created_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    operator_id bigint NULL,
    operator_name varchar(191) NULL,
    target_type varchar(100) NULL,
    target_id varchar(100) NULL,
    target_name varchar(191) NULL,
    action varchar(30) NULL,
    old_value jsonb NULL,
    new_value jsonb NULL,
    ip_address varchar(64) NULL,
    request_id varchar(100) NULL,
    payload jsonb NULL,
    CONSTRAINT sys_permission_audit_logs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_internal_messages (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    title varchar(255) NOT NULL,
    content text NULL,
    status varchar(20) NOT NULL DEFAULT 'DRAFT',
    sender_id bigint NULL,
    sender_name varchar(191) NULL,
    category_id bigint NULL,
    CONSTRAINT mall_internal_messages_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_internal_message_recipients (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    message_id bigint NOT NULL,
    recipient_user_id bigint NOT NULL,
    status varchar(20) NOT NULL DEFAULT 'SENT',
    received_at timestamptz NULL,
    read_at timestamptz NULL,
    title varchar(255) NULL,
    content text NULL,
    CONSTRAINT mall_internal_message_recipients_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_im_recipients_user
    ON public.mall_internal_message_recipients (recipient_user_id);

CREATE TABLE IF NOT EXISTS public.mall_internal_message_categories (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(100) NOT NULL,
    code varchar(100) NOT NULL,
    icon_url varchar(500) NULL,
    sort_order int NULL DEFAULT 0,
    is_enabled boolean NULL DEFAULT true,
    CONSTRAINT mall_internal_message_categories_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_tasks (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    type varchar(40) NOT NULL DEFAULT 'PERIODIC',
    type_name varchar(100) NOT NULL,
    payload text NULL,
    cron_spec varchar(100) NULL,
    options jsonb NULL,
    enable boolean NULL DEFAULT true,
    remark text NULL,
    CONSTRAINT sys_tasks_pkey PRIMARY KEY (id)
);



CREATE TABLE IF NOT EXISTS public.sys_tenants (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  remark character varying,
  name character varying,
  code character varying,
  logo_url character varying,
  domain character varying,
  industry character varying,
  admin_user_id bigint,
  status character varying DEFAULT 'ON'::character varying,
  type character varying DEFAULT 'PAID'::character varying,
  audit_status character varying,
  subscription_at timestamp with time zone,
  unsubscribe_at timestamp with time zone,
  subscription_plan character varying,
  expired_at timestamp with time zone
);

CREATE TABLE IF NOT EXISTS public.sys_org_units (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  sort_order bigint DEFAULT 0,
  tenant_id bigint DEFAULT 0,
  remark character varying,
  description character varying,
  path character varying,
  name character varying NOT NULL,
  code character varying,
  leader_id bigint,
  type character varying NOT NULL DEFAULT 'DEPARTMENT'::character varying,
  business_scopes jsonb,
  external_id character varying,
  is_legal_entity boolean DEFAULT false,
  registration_number character varying,
  tax_id character varying,
  legal_entity_org_id bigint,
  address character varying,
  phone character varying,
  email character varying,
  timezone character varying,
  country character varying,
  latitude double precision,
  longitude double precision,
  start_at timestamp with time zone,
  end_at timestamp with time zone,
  contact_user_id bigint,
  permission_tags jsonb,
  parent_id bigint
);

CREATE TABLE IF NOT EXISTS public.sys_positions (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  sort_order bigint DEFAULT 0,
  remark character varying,
  tenant_id bigint DEFAULT 0,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  name character varying NOT NULL,
  code character varying NOT NULL,
  org_unit_id bigint NOT NULL,
  reports_to_position_id bigint,
  description character varying,
  job_family character varying,
  job_grade character varying,
  level integer,
  headcount bigint NOT NULL DEFAULT 0,
  is_key_position boolean NOT NULL DEFAULT false,
  type character varying NOT NULL DEFAULT 'REGULAR'::character varying,
  start_at timestamp with time zone,
  end_at timestamp with time zone
);

CREATE TABLE IF NOT EXISTS public.sys_apis (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  tenant_id bigint DEFAULT 0,
  description character varying,
  module character varying,
  module_description character varying,
  operation character varying,
  path character varying,
  method character varying,
  scope character varying DEFAULT 'ADMIN'::character varying
);

CREATE TABLE IF NOT EXISTS public.sys_menus (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  remark character varying,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  tenant_id bigint DEFAULT 0,
  type character varying DEFAULT 'MENU'::character varying,
  path character varying DEFAULT ''::character varying,
  redirect character varying,
  alias character varying,
  name character varying,
  component character varying DEFAULT ''::character varying,
  meta jsonb,
  parent_id bigint
);

CREATE TABLE IF NOT EXISTS public.sys_permissions (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  description character varying,
  tenant_id bigint DEFAULT 0,
  name character varying NOT NULL,
  code character varying NOT NULL,
  group_id bigint
);

CREATE TABLE IF NOT EXISTS public.sys_role_metadata (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  tenant_id bigint DEFAULT 0,
  role_id bigint,
  is_template boolean DEFAULT false,
  template_for character varying,
  template_version integer DEFAULT 1,
  last_synced_version integer,
  last_synced_at timestamp with time zone,
  sync_policy character varying DEFAULT 'AUTO'::character varying,
  scope character varying DEFAULT 'TENANT'::character varying,
  custom_overrides jsonb NOT NULL
);

CREATE TABLE IF NOT EXISTS public.sys_role_permissions (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  tenant_id bigint DEFAULT 0,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  role_id bigint NOT NULL,
  permission_id bigint NOT NULL,
  effect character varying DEFAULT 'ALLOW'::character varying,
  priority integer DEFAULT 0
);

CREATE TABLE IF NOT EXISTS public.sys_roles (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  remark character varying,
  description character varying,
  sort_order bigint DEFAULT 0,
  tenant_id bigint DEFAULT 0,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  name character varying,
  code character varying,
  is_protected boolean NOT NULL DEFAULT false,
  type character varying NOT NULL DEFAULT 'TENANT'::character varying,
  parent_id bigint
);

CREATE TABLE IF NOT EXISTS public.sys_user_roles (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  tenant_id bigint DEFAULT 0,
  user_id bigint NOT NULL,
  role_id bigint NOT NULL,
  start_at timestamp with time zone,
  end_at timestamp with time zone,
  assigned_at timestamp with time zone,
  assigned_by bigint,
  is_primary boolean NOT NULL DEFAULT false,
  status character varying NOT NULL DEFAULT 'ACTIVE'::character varying
);

CREATE TABLE IF NOT EXISTS public.sys_permission_apis (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  tenant_id bigint DEFAULT 0,
  permission_id bigint NOT NULL,
  api_id bigint NOT NULL
);

CREATE TABLE IF NOT EXISTS public.sys_permission_menus (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  tenant_id bigint DEFAULT 0,
  permission_id bigint NOT NULL,
  menu_id bigint NOT NULL
);

CREATE TABLE IF NOT EXISTS public.sys_policy_evaluation_logs (
  id bigserial,
  created_at timestamp with time zone,
  tenant_id bigint DEFAULT 0,
  user_id bigint NOT NULL,
  membership_id bigint NOT NULL,
  permission_id bigint NOT NULL,
  policy_id bigint,
  request_path character varying,
  request_method character varying,
  result boolean NOT NULL DEFAULT false,
  effect_details character varying,
  scope_sql character varying,
  ip_address character varying,
  trace_id character varying,
  evaluation_context character varying,
  log_hash character varying,
  signature bytea
);

CREATE TABLE IF NOT EXISTS public.sys_permission_groups (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  description character varying,
  status character varying NOT NULL DEFAULT 'ON'::character varying,
  sort_order bigint DEFAULT 0,
  path character varying,
  tenant_id bigint DEFAULT 0,
  name character varying NOT NULL,
  module character varying,
  parent_id bigint
);

CREATE TABLE IF NOT EXISTS public.sys_monitor_alert_rules (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(191) NOT NULL,
    metric varchar(40) NOT NULL DEFAULT 'GO_GOROUTINES',
    op varchar(10) NOT NULL DEFAULT 'GE',
    threshold double precision NOT NULL DEFAULT 0,
    cooldown_minutes int NULL DEFAULT 30,
    channel varchar(30) NULL DEFAULT 'EMAIL',
    target varchar(255) NULL,
    is_enabled boolean NULL DEFAULT true,
    remark text NULL,
    last_firing boolean NULL DEFAULT false,
    last_value double precision NULL DEFAULT 0,
    last_alerted_at timestamptz NULL,
    CONSTRAINT sys_monitor_alert_rules_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_tax_rates (
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
    tax_rate integer NOT NULL DEFAULT 0,
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    CONSTRAINT mall_tax_rates_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_invoices (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    order_id bigint NOT NULL,
    user_id bigint NULL,
    invoice_number varchar(100) NULL,
    invoice_type varchar(30) NOT NULL DEFAULT 'VAT_GENERAL',
    status varchar(20) NOT NULL DEFAULT 'PENDING',
    buyer_name varchar(191) NULL,
    buyer_tax_id varchar(100) NULL,
    buyer_address text NULL,
    buyer_phone varchar(30) NULL,
    amount bigint NOT NULL DEFAULT 0,
    CONSTRAINT mall_invoices_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_invoices_order ON public.mall_invoices (order_id);

CREATE TABLE IF NOT EXISTS public.sys_dict_types (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  is_enabled boolean DEFAULT true,
  sort_order bigint DEFAULT 0,
  tenant_id bigint DEFAULT 0,
  type_code character varying,
  type_name character varying
);

CREATE TABLE IF NOT EXISTS public.sys_dict_entries (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  sort_order bigint DEFAULT 0,
  is_enabled boolean DEFAULT true,
  tenant_id bigint DEFAULT 0,
  entry_value character varying NOT NULL,
  numeric_value integer,
  type_id bigint
);

CREATE TABLE IF NOT EXISTS public.sys_languages (
  id bigserial,
  created_at timestamp with time zone,
  updated_at timestamp with time zone,
  deleted_at timestamp with time zone,
  created_by bigint,
  updated_by bigint,
  deleted_by bigint,
  sort_order bigint DEFAULT 0,
  is_enabled boolean DEFAULT true,
  language_code character varying,
  language_name character varying,
  native_name character varying,
  is_default boolean DEFAULT false
);

CREATE TABLE IF NOT EXISTS public.mall_promotions (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(191) NOT NULL,
    promo_type varchar(30) NOT NULL DEFAULT 'SPECIAL_PRICE',
    product_id bigint NULL,
    special_price bigint NULL DEFAULT 0,
    threshold_amount bigint NULL DEFAULT 0,
    reduction_amount bigint NULL DEFAULT 0,
    priority int NULL DEFAULT 0,
    valid_from timestamptz NULL,
    valid_until timestamptz NULL,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    CONSTRAINT mall_promotions_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_stock_alerts (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    sku_id bigint NOT NULL,
    stock_qty_at_trigger int NOT NULL DEFAULT 0,
    threshold int NOT NULL DEFAULT 0,
    status varchar(20) NOT NULL DEFAULT 'OPEN',
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    CONSTRAINT mall_stock_alerts_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_stock_op_logs (
    id bigserial,
    created_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    sku_id bigint NOT NULL,
    product_id bigint NULL,
    delta int NOT NULL DEFAULT 0,
    stock_before int NULL DEFAULT 0,
    stock_after int NULL DEFAULT 0,
    reason varchar(40) NULL,
    ref_id bigint NULL,
    remark text NULL,
    CONSTRAINT mall_stock_op_logs_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_product_attributes (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    sort_order integer NULL DEFAULT 0,
    CONSTRAINT mall_product_attributes_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_product_attribute_translations (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    attribute_id bigint NOT NULL,
    language_code varchar(20) NOT NULL,
    name varchar(255) NOT NULL,
    CONSTRAINT mall_product_attribute_translations_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_pa_translations_attribute
        FOREIGN KEY (attribute_id)
        REFERENCES public.mall_product_attributes (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_pa_translations_attr_language
    ON public.mall_product_attribute_translations (attribute_id, language_code);

CREATE TABLE IF NOT EXISTS public.mall_product_attribute_values (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    sort_order integer NULL DEFAULT 0,
    attribute_id bigint NOT NULL,
    CONSTRAINT mall_product_attribute_values_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_pa_values_attribute
        FOREIGN KEY (attribute_id)
        REFERENCES public.mall_product_attributes (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS public.mall_product_attribute_value_translations (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    value_id bigint NOT NULL,
    language_code varchar(20) NOT NULL,
    display_name varchar(255) NOT NULL,
    CONSTRAINT mall_product_attribute_value_translations_pkey PRIMARY KEY (id),
    CONSTRAINT fk_mall_pav_translations_value
        FOREIGN KEY (value_id)
        REFERENCES public.mall_product_attribute_values (id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_pav_translations_value_language
    ON public.mall_product_attribute_value_translations (value_id, language_code);

CREATE TABLE IF NOT EXISTS public.mall_seckill_activities (
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
    seckill_price bigint NOT NULL DEFAULT 0,
    total_stock int NOT NULL DEFAULT 0,
    sold_count int NOT NULL DEFAULT 0,
    per_user_limit int NULL DEFAULT 1,
    priority int NULL DEFAULT 0,
    valid_from timestamptz NULL,
    valid_until timestamptz NULL,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    CONSTRAINT mall_seckill_activities_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.mall_points_ledgers (
    id bigserial,
    created_at timestamptz NULL,
    deleted_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NOT NULL,
    delta bigint NOT NULL DEFAULT 0,
    balance_after bigint NOT NULL DEFAULT 0,
    type varchar(40) NOT NULL DEFAULT 'POINTS_TYPE_EARN',
    reason varchar(255) NULL,
    ref_order_id bigint NULL,
    updated_by bigint NULL,
    created_by bigint NULL,
    deleted_by bigint NULL,
    updated_at timestamptz NULL,
    CONSTRAINT mall_points_ledgers_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_points_ledgers_user ON public.mall_points_ledgers (user_id);

CREATE TABLE IF NOT EXISTS public.mall_member_tiers (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(100) NOT NULL,
    level int NOT NULL DEFAULT 1,
    min_spend bigint NOT NULL DEFAULT 0,
    points_multiplier int NULL DEFAULT 1,
    badge_icon varchar(255) NULL,
    currency varchar(10) NOT NULL DEFAULT 'CNY',
    status varchar(20) NOT NULL DEFAULT 'ACTIVE',
    CONSTRAINT mall_member_tiers_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_user_credentials (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NOT NULL,
    identity_type varchar(30) NULL,
    identifier varchar(255) NOT NULL,
    credential_type varchar(30) NULL,
    credential text NOT NULL,
    is_primary boolean NULL DEFAULT false,
    status varchar(20) NULL DEFAULT 'ACTIVE',
    extra_info jsonb NULL,
    provider varchar(50) NULL,
    provider_account_id varchar(255) NULL,
    activate_token_hash varchar(255) NULL,
    activate_token_expires_at timestamptz NULL,
    activate_token_used_at timestamptz NULL,
    reset_token_hash varchar(255) NULL,
    reset_token_expires_at timestamptz NULL,
    reset_token_used_at timestamptz NULL,
    CONSTRAINT sys_user_credentials_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_user_credentials_identifier
    ON public.sys_user_credentials (identifier);

CREATE TABLE IF NOT EXISTS public.sys_user_mfa_factors (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    tenant_id bigint NULL DEFAULT 0,
    user_id bigint NULL,
    method varchar(30) NULL DEFAULT 'TOTP',
    secret_hash varchar(512) NULL,
    display_name varchar(128) NULL,
    status varchar(30) NULL DEFAULT 'ENABLED',
    last_used_at timestamptz NULL,
    CONSTRAINT sys_user_mfa_factors_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_sys_user_mfa_tenant_user_id
    ON public.sys_user_mfa_factors (tenant_id, user_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sys_user_mfa_tenant_uid_method
    ON public.sys_user_mfa_factors (tenant_id, user_id, method);

CREATE TABLE IF NOT EXISTS public.sys_login_policies (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    target_id bigint NULL,
    value varchar(255) NULL,
    reason varchar(255) NULL,
    type varchar(30) NULL DEFAULT 'BLACKLIST',
    method varchar(30) NULL DEFAULT 'IP',
    CONSTRAINT sys_login_policies_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.files (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    remark varchar(255) NULL,
    tenant_id bigint NULL DEFAULT 0,
    provider varchar(30) NULL DEFAULT 'MINIO',
    bucket_name varchar(255) NULL,
    file_directory varchar(500) NULL,
    file_guid varchar(255) NULL,
    save_file_name varchar(255) NULL,
    file_name varchar(255) NULL,
    extension varchar(50) NULL,
    size bigint NULL,
    size_format varchar(30) NULL,
    link_url varchar(1000) NULL,
    content_hash varchar(255) NULL,
    CONSTRAINT files_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_plans (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    remark varchar(255) NULL,
    name varchar(100) NULL,
    version varchar(30) NULL DEFAULT 'FREE',
    expiry_policy varchar(30) NULL DEFAULT 'READONLY',
    data_retention_days bigint NULL,
    description text NULL,
    CONSTRAINT sys_plans_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uniq_sys_plans_name ON public.sys_plans (name);

CREATE TABLE IF NOT EXISTS public.sys_plan_quotas (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    plan_id bigint NOT NULL,
    quota_type varchar(30) NULL,
    quota_value bigint NULL,
    CONSTRAINT sys_plan_quotas_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_plan_quotas_plan ON public.sys_plan_quotas (plan_id);

CREATE TABLE IF NOT EXISTS public.sys_plan_modules (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    plan_id bigint NOT NULL,
    module varchar(30) NULL,
    CONSTRAINT sys_plan_modules_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_plan_modules_plan ON public.sys_plan_modules (plan_id);

CREATE TABLE IF NOT EXISTS public.sys_configs (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    name varchar(100) NULL,
    key varchar(100) NULL,
    value text NULL,
    value_type varchar(20) NULL DEFAULT 'STRING',
    is_built_in boolean NULL DEFAULT false,
    CONSTRAINT sys_configs_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uniq_sys_configs_key ON public.sys_configs (key);

CREATE TABLE IF NOT EXISTS public.sys_access_keys (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(100) NULL,
    access_key varchar(100) NULL,
    secret_hash varchar(128) NULL,
    status varchar(10) NULL DEFAULT 'ON',
    expires_at timestamptz NULL,
    last_used_at timestamptz NULL,
    CONSTRAINT sys_access_keys_pkey PRIMARY KEY (id)
);

CREATE UNIQUE INDEX IF NOT EXISTS uniq_sys_access_keys_ak ON public.sys_access_keys (access_key);

CREATE TABLE IF NOT EXISTS public.sys_scripts (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    name varchar(200) NULL,
    language varchar(30) NULL DEFAULT 'LUA',
    hook_point varchar(200) NULL,
    source text NULL,
    priority int NULL DEFAULT 0,
    description varchar(500) NULL,
    critical boolean NULL DEFAULT false,
    version int NULL DEFAULT 1,
    is_enabled boolean NULL DEFAULT false,
    CONSTRAINT sys_scripts_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_script_logs (
    id bigserial,
    created_at timestamptz NULL,
    script_id bigint NULL,
    script_name varchar(200) NULL,
    language varchar(30) NULL,
    trigger_type varchar(50) NULL,
    hook_point varchar(200) NULL,
    version int NULL DEFAULT 0,
    success boolean NULL,
    duration_ms bigint NULL,
    error text NULL,
    context jsonb NULL,
    CONSTRAINT sys_script_logs_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_script_logs_script ON public.sys_script_logs (script_id);

CREATE TABLE IF NOT EXISTS public.sys_ai_providers (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    remark varchar(500) NULL,
    tenant_id bigint NULL DEFAULT 0,
    name varchar(100) NULL,
    model_type varchar(20) NULL DEFAULT 'LOCAL',
    model_name varchar(200) NULL,
    base_url varchar(500) NULL,
    organization varchar(200) NULL,
    api_key text NULL,
    api_key_hint varchar(100) NULL,
    local_host varchar(200) NULL,
    local_port int NULL,
    timeout_seconds int NULL,
    system_prompt text NULL,
    is_default boolean NULL DEFAULT false,
    is_enabled boolean NULL DEFAULT true,
    CONSTRAINT sys_ai_providers_pkey PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS public.sys_ai_conversations (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    title varchar(200) NULL,
    provider_id bigint NULL DEFAULT 0,
    user_id bigint NULL,
    last_message_at timestamptz NULL,
    CONSTRAINT sys_ai_conversations_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_ai_conversations_user ON public.sys_ai_conversations (tenant_id, user_id);

CREATE TABLE IF NOT EXISTS public.sys_ai_messages (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    conversation_id bigint NULL,
    role varchar(20) NULL DEFAULT 'USER',
    content text NULL,
    model_name varchar(200) NULL,
    prompt_tokens bigint NULL,
    completion_tokens bigint NULL,
    duration_ms bigint NULL,
    error_message text NULL,
    user_id bigint NULL,
    CONSTRAINT sys_ai_messages_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_ai_messages_conversation ON public.sys_ai_messages (conversation_id);

CREATE TABLE IF NOT EXISTS public.sys_ai_usage_logs (
    id bigserial,
    created_at timestamptz NULL,
    updated_at timestamptz NULL,
    deleted_at timestamptz NULL,
    created_by bigint NULL,
    updated_by bigint NULL,
    deleted_by bigint NULL,
    tenant_id bigint NULL DEFAULT 0,
    provider_id bigint NULL,
    conversation_id bigint NULL DEFAULT 0,
    user_id bigint NULL,
    model_name varchar(200) NULL,
    prompt_tokens bigint NULL,
    completion_tokens bigint NULL,
    total_tokens bigint NULL,
    duration_ms bigint NULL,
    CONSTRAINT sys_ai_usage_logs_pkey PRIMARY KEY (id)
);

CREATE INDEX IF NOT EXISTS idx_ai_usage_logs_tenant_time ON public.sys_ai_usage_logs (tenant_id, created_at);
