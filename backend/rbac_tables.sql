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