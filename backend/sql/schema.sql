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
