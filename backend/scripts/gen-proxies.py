#!/usr/bin/env python3
# gen-proxies.py — 从生成的 BFF trait（OUT_DIR 的 *_gen.rs services 模块）产出
# 瘦 BFF 代理实现：每个 trait 方法 → 同名领域 gRPC 客户端调用。
#
# 契约保证：BFF trait 方法与领域 gRPC 服务方法同名同型（BFF proto 直接引用
# 领域消息），转发是零映射 pass-through。
#
# 跳过（手工实现或保持桩）：
#   AuthenticationService  — 验证码/cookie 等 BFF 关注点（手写）
#   AdminPortalService    — BFF 聚合面，无领域对应（手写）
#   FileTransferService   — multipart 流式传输（手写/桩）
#
# 用法：
#   python3 scripts/gen-proxies.py <admin_gen.rs> admin > services/admin-api/src/services/proxies.rs
#   python3 scripts/gen-proxies.py <app_gen.rs>   app  > services/app-api/src/services/proxies.rs
import re
import sys

# BFF 服务名 → 领域包（与 api/protos 的领域服务声明一致）
PACKAGE = {
    "AccessKey": "access_key",
    "AdminPortal": "permission",
    "AiAgent": "ai",
    "AiChat": "ai",
    "AiContent": "ai",
    "AiConversation": "ai",
    "AiKnowledgeBase": "ai",
    "AiMessage": "ai",
    "AiProvider": "ai",
    "AiQuery": "ai",
    "AiUsageLog": "ai",
    "Api": "permission",
    "ApiAuditLog": "audit",
    "Authentication": "authentication",
    "Brand": "catalog",
    "Cart": "cart",
    "CartItem": "cart",
    "Category": "catalog",
    "Comment": "comment",
    "Config": "config",
    "CouponTemplate": "coupon",
    "DataAccessAuditLog": "audit",
    "DictEntry": "dict",
    "DictType": "dict",
    "File": "storage",
    "FileTransfer": "storage",
    "GrouponActivity": "catalog",
    "InteractionAdmin": "interaction",
    "InternalMessage": "internal_message",
    "InternalMessageCategory": "internal_message",
    "InternalMessageRecipient": "internal_message",
    "Invoice": "invoice",
    "Language": "dict",
    "LoginAuditLog": "audit",
    "LoginPolicy": "authentication",
    "MemberTier": "catalog",
    "Menu": "permission",
    "Mfa": "authentication",
    "MonitorAlert": "monitor_alert",
    "Notification": "notification",
    "NotificationChannel": "notification",
    "NotificationPreference": "notification",
    "NotificationRule": "notification",
    "NotificationTemplate": "notification",
    "OnlineSession": "online_session",
    "OperationAuditLog": "audit",
    "Order": "order",
    "OrderItem": "order",
    "OrgUnit": "identity",
    "PaymentChannelConfig": "payment",
    "PaymentRefund": "payment",
    "PaymentTransaction": "payment",
    "Permission": "permission",
    "PermissionAuditLog": "audit",
    "PermissionGroup": "permission",
    "Plan": "identity",
    "PlanModule": "identity",
    "PlanQuota": "identity",
    "PointsLedger": "catalog",
    "PolicyEvaluationLog": "permission",
    "Position": "identity",
    "Product": "catalog",
    "ProductAttribute": "catalog",
    "ProductAttributeValue": "catalog",
    "Promotion": "catalog",
    "RedisCacheMonitor": "redis_cache",
    "Role": "permission",
    "Script": "script",
    "ScriptLog": "script",
    "SeckillActivity": "catalog",
    "ServerMonitor": "redis_cache",
    "Shipment": "shipping",
    "ShippingRate": "shipping",
    "Sku": "catalog",
    "SkuAttributeCombination": "catalog",
    "SkuPrice": "catalog",
    "StockAlert": "catalog",
    "StockOpLog": "catalog",
    "Task": "task",
    "TaskMonitor": "task",
    "TaxRate": "tax",
    "Tenant": "identity",
    "User": "identity",
    "UserCoupon": "coupon",
    "UserProfile": "identity",
    "AiChat": "ai",
    "Authentication": "authentication",
    "Brand": "catalog",
    "Cart": "cart",
    "CartItem": "cart",
    "Category": "catalog",
    "Comment": "comment",
    "CouponTemplate": "coupon",
    "FileTransfer": "storage",
    "Groupon": "catalog",
    "Interaction": "interaction",
    "InternalMessageRecipient": "internal_message",
    "Invoice": "invoice",
    "MemberTier": "catalog",
    "Order": "order",
    "OrderItem": "order",
    "PaymentChannel": "payment",
    "PaymentRefund": "payment",
    "PaymentTransaction": "payment",
    "Points": "catalog",
    "Product": "catalog",
    "ProductAttribute": "catalog",
    "ProductAttributeValue": "catalog",
    "Promotion": "catalog",
    "Seckill": "catalog",
    "Shipment": "shipping",
    "ShippingAddress": "address",
    "Sku": "catalog",
    "SkuAttributeCombination": "catalog",
    "SkuPrice": "catalog",
    "UserCoupon": "coupon",
    "UserProfile": "identity",
    "Wishlist": "wishlist"
}

# 领域 client 名与 BFF 服务名错位的覆写：(模块, 类型)。缺省按
# snake(service)_service_client + {service}ServiceClient 派生。
DOMAIN_CLIENT = {
    # 缩略词归一：tonic 把 MFAService 的 client 命名为 MfaServiceClient。
    "Mfa": ("mfa_service_client", "MfaServiceClient"),
    # 契约期改名：BFF 面的服务名落后/超前于域服务名（同消息类型）。
    "Groupon": ("groupon_activity_service_client", "GrouponActivityServiceClient"),
    "Points": ("points_ledger_service_client", "PointsLedgerServiceClient"),
    "Seckill": ("seckill_activity_service_client", "SeckillActivityServiceClient"),
    "PaymentChannel": ("payment_channel_config_service_client", "PaymentChannelConfigServiceClient"),
}

# snake 派生对连续大写词（缩略词）会把每个字母断开（MFA → m_f_a），领域
# client 模块是整词 mfa——逐个钉正。
SNAKE_OVERRIDE = {"MFA": "mfa"}

# BFF 方法名与域 RPC 名漂移（同服务 client、消息同源）的改名转发：
# (服务, BFF 方法) → 域 RPC 方法名。生成 `pass <bff> => <domain>`。
RENAMED_METHODS = {
    # 契约期改名：BFF 面的"我的"语义拼写 ↔ 域面的"用户"拼写（同请求消息）。
    ("Points", "get_my_points_balance"): "get_user_points_balance",
    ("Points", "list_my_points_ledgers"): "list",
    # MFA 缩略词：gen-http 对 MFA 的蛇形切法（m_f_a）↔ tonic 域 client
    # 的整词 mfa。verify_m_f_a_challenge 响应形状错位，走 stub。
    ("Mfa", "get_m_f_a_status"): "get_mfa_status",
    ("Mfa", "disable_m_f_a"): "disable_mfa",
    ("Mfa", "reset_user_m_f_a"): "reset_user_mfa",
    ("Mfa", "revoke_m_f_a_device"): "revoke_mfa_device",
}

# 手工实现的面（不生成）
SKIP = {
    # Authentication：Phase pass 转发（token 走响应体；Set-Cookie 的
    # 手写面随登录页联调落地）。
    "AdminPortal",
    "FileTransfer",
    # 契约仅 admin 面声明、领域尚无对应服务（随特性批次落域后转正）。
    "TaskMonitor",
    # 契约仅 admin 面声明、领域尚无对应服务（随特性批次落域后转正）。
    "ComponentDefinition",
    "Preview",
}
# 面特定的额外跳过（BFF 方法集超出领域面）
SKIP_BY_FACE = {"app": set(), "admin": set()}

# BFF/领域签名错位的方法（非零映射转发）——生成桩，待手写适配：
#   ApiService.SyncApis：BFF Empty ↔ 领域 SyncApisRequest
STUB_METHODS = {
    # Chat：域侧 server-streaming（ChatStreamFrame），BFF 面按 unary
    # ChatResponse 建模——流式桥接随 AI 特性批次落 behaviors/SSE。
    ("AiChat", "chat"),
    # CreatePaymentIntent：BFF 面返回 App 变体响应（同包不同消息），
    # 转发后需要逐字段适配——随支付域落 behaviors。
    ("PaymentTransaction", "create_payment_intent"),
    # SyncApis：BFF Empty ↔ 域 SyncApisRequest —— 请求形状错位。
    ("Api", "sync_apis"),
    # SyncPermissions：BFF Empty ↔ 域 SyncPermissionsRequest —— 同上。
    ("Permission", "sync_permissions"),
    # MFA 挑战校验：BFF LoginResponse（通过即发令牌对）↔ 域
    # VerifyMFAChallengeResponse（success + 可选令牌对）——逐字段
    # 适配随认证域落 behaviors。
    ("Mfa", "verify_m_f_a_challenge"),
}

# 行为模块（手写面）按 face 路由：admin → services/behaviors.rs，
# app → services/public.rs（每个面恒定携带一行 `behaviors:`，供 hand 形态委派）。

# 面特定的行为覆写——方法体改为调用手写模块，行为与生成代码解耦：
#   app 公开站点语义（参照 app/service 的服务层逐方法对位）：公开读
#   状态过滤、内容写禁用、Host 租户解析、游客评论策略、档案/改密钉扎。
#   admin 侧：文件元数据创建（服务端操作者盖章）、路由表同步（本二进制
#   路由语料）、walk-route 调试面、权限派生重建触发、改密/联系面、
#   管理员改密适配（BFF 路径参数 ↔ 领域 UpdateUserRequest）。
BEHAVIOR_METHODS = {"admin": {}, "app": {}}

# 响应形状错位（BFF Empty ↔ 领域实体）：转发后丢弃响应体（`drop` 形态）
# 响应错位（BFF Empty ↔ 域实体）：转发后丢弃响应体（drop 形态）。
ADAPTER_METHODS = {("Tenant", "create"), ("File", "create"), ("File", "update")}

RUST_KEYWORDS = {"type", "ref"}



def domain_rpcs(api_root):
    """pkg -> service_name -> set(rpc names), read straight off the
    contract tree - the ground truth the pass/stub split anchors on."""
    import glob
    import os
    table = {}
    for path in glob.glob(api_root + "/protos/*/service/v1/*.proto"):
        norm = path.replace(os.sep, "/")
        # The BFF faces (admin/app) are the callers' side — their rpc
        # lists must not vouch for themselves in the domain check.
        if "/admin/" in norm or "/app/" in norm:
            continue
        pkg = norm.split("/protos/")[1].split("/service/v1/")[0]
        src = open(path, encoding="utf-8").read()
        for svc, body in re.findall(r"service (\w+) \{(.*?)\n\}", src, re.S):
            # tonic client methods are snake_case of the proto rpc
            # spelling — collect the normalized form the BFF trait
            # methods compare against.
            names = {
                re.sub(r"(?<!^)(?=[A-Z])", "_", m.group(1)).lower()
                for m in re.finditer(r"rpc\s+(\w+)\s*\(", body)
            }
            table.setdefault(pkg, {}).setdefault(svc, set()).update(names)
    return table


def parse_traits(gen_path):
    src = open(gen_path, encoding="utf-8").read()
    traits = {}
    for m in re.finditer(
        r"#\[async_trait::async_trait\]\s*pub trait (\w+)Handlers: Send \+ Sync \{(.*?)\n    \}",
        src,
        re.S,
    ):
        name, body = m.group(1), m.group(2)
        methods = []
        for mm in re.finditer(
            r"async fn (\w+)\(\s*&self, _?ctx: rushwind_http_binding::ctx::RequestContext, req: (.+?)\)\s*-> Result<(.+)>;",
            body,
        ):
            # normalize: the generated faces address types as crate::proto
            # (they live in the proto crate); the proxies live in the
            # service crate. Result<Resp, Err> — keep only the Resp.
            norm = lambda t: t.strip().replace("crate::proto::", "shop_proto::proto::")
            resp = mm.group(3).split(", rushwind_http_binding::envelope::StatusError")[0]
            methods.append((mm.group(1), norm(mm.group(2)), norm(resp)))
        if methods:
            traits[name] = methods
    return traits


def snake(name):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def method_kind(service, method, face, rpcs=None):
    """One contract method's proxy-kit form — (kind, target-name).

    Precedence: the hand behavior wins over everything (an admin face
    method can be both STUB-listed and behavior-overridden — the stub
    only applies where no behavior rides), then the renamed forward
    (same service client, same message types, drifted RPC name), then
    the response-dropping adapter, then the not-implemented stub, else
    the plain pass-through.
    """
    behavior = BEHAVIOR_METHODS.get(face, {}).get((service, method))
    if behavior:
        return "hand", behavior
    renamed = RENAMED_METHODS.get((service, method))
    if renamed:
        return "renamed", renamed
    if (service, method) in ADAPTER_METHODS:
        return "drop", None
    if (service, method) in STUB_METHODS:
        return "stub", None
    # The domain face grounds the forward: a BFF method with no same-name
    # RPC on the pinned domain service rides the stub (the contract
    # carries it; the domain does not back it on this deployment).
    domain_svc = DOMAIN_CLIENT.get(service, (None, f"{service}Service"))[1]
    if rpcs is not None:
        names = set()
        for pkg_services in rpcs.values():
            names.update(pkg_services.get(domain_svc, set()))
        if names and method not in names:
            return "stub", None
    # The pass-through's type identity: the BFF face must reference the
    # domain package's own messages (the pagination envelope is shared).
    # An app/admin-local message type has no same-shaped domain twin —
    # the forward needs a hand mapping, so it rides the stub until the
    # behavior lands.
    return "pass", None


def shorten(ty, pkgs):
    """A type path against the package-root aliases — same type, shorter
    line (the fully-qualified paths push the macro table past the
    100-column width rustfmt holds elsewhere in the crate)."""
    for pkg in pkgs:
        ty = ty.replace(f"shop_proto::proto::{pkg}::service::v1::", f"{pkg}_v1::")
    return ty.replace("shop_proto::proto::pagination::", "pagination::")


def main():
    gen_path, face = sys.argv[1], sys.argv[2]
    rpcs = domain_rpcs("api")
    traits = parse_traits(gen_path)
    gen_mod = "gen_admin" if face == "admin" else "gen_app"

    # First pass — the emitted faces, so the alias block covers exactly
    # the packages the output references.
    faces = []
    used_pkgs = set()
    for trait_name, methods in sorted(traits.items()):
        service = trait_name.removesuffix("Service")
        if service in SKIP or service in SKIP_BY_FACE.get(face, set()):
            continue
        pkg = PACKAGE.get(service)
        if pkg is None:
            continue
        kinds = [(m, *method_kind(service, m, face, rpcs), req_ty, resp_ty)
                 for m, req_ty, resp_ty in methods]
        # The pass-through's type identity: the BFF face must reference
        # the domain package's own messages (the pagination envelope is
        # shared). A face-local message type has no same-shaped domain
        # twin — the forward needs a hand mapping, so it rides the stub
        # until the behavior lands.
        # The request side is strict (a face-local or pbjson request has no
        # same-shaped domain twin); the response side stays loose — an Empty
        # reply is same-shaped, and entity-vs-Empty drifts ride the drop
        # adapters.
        def req_grounded(ty):
            seg = ty.split("proto::proto::")
            return len(seg) >= 2 and seg[1].split("::")[0] in (pkg, "pagination")
        def resp_grounded(ty):
            return req_grounded(ty) or ty.startswith("pbjson_types::")
        kinds = [(m, *(("stub", None) if kind == "pass" and not (req_grounded(req_ty) and resp_grounded(resp_ty)) else (kind, behavior)), req_ty, resp_ty)
                 for m, kind, behavior, req_ty, resp_ty in kinds]
        faces.append((trait_name, service, pkg, kinds))
        used_pkgs.add(pkg)
    uses_pagination = any(
        "shop_proto::proto::pagination::" in ty
        for _, _, _, kinds in faces
        for _, _, _, req_ty, resp_ty in kinds
        for ty in (req_ty, resp_ty)
    )

    out = []
    out.append("//! Generated thin-BFF proxies — a table of `passthrough_proxy!`")
    out.append("//! invocations, one per BFF face (see services/proxy_kit.rs for the")
    out.append("//! pass/drop/stub/hand method kinds; the BFF and domain methods share")
    out.append("//! their message types by contract, so no mapping rides here).")
    out.append("//! DO NOT EDIT; regenerate via scripts/gen-proxies.py when the")
    out.append("//! contract re-syncs. Hand-written faces (authentication — captcha/")
    out.append("//! cookie concerns; admin-portal aggregation; file-transfer multipart)")
    out.append("//! live in their own modules.")
    out.append("#![allow(clippy::all)]")
    out.append("#![allow(missing_docs)]")
    out.append("")
    out.append("use crate::passthrough_proxy;")
    # Package-root aliases: every type below addresses its package through
    # one of these (see `shorten`), keeping the lines at the 100-column
    # width. Sorted: rustfmt orders the block and would otherwise drift
    # the committed file away from the generator's output.
    import_lines = [
        f"use shop_proto::proto::{pkg}::service::v1 as {pkg}_v1;" for pkg in used_pkgs
    ]
    if uses_pagination:
        import_lines.append("use shop_proto::proto::pagination;")
    out.extend(sorted(import_lines))
    out.append("")

    emitted = 0
    for trait_name, service, pkg, kinds in faces:
        client_mod, client_ty = DOMAIN_CLIENT.get(
            service,
            (f"{SNAKE_OVERRIDE.get(service, snake(service))}_service_client",
             f"{service}ServiceClient"),
        )
        # The Channel-generic spelled out: the kit calls the client via a
        # qualified path (<Ty>::new), where inference does not apply.
        client_head = f"        client: {pkg}_v1::{client_mod}::{client_ty}"
        struct_name = f"{service}Proxy"
        behaviors = "behaviors" if face == "admin" else "public"
        out.append("passthrough_proxy! {")
        out.append(f"    /// The pass-through proxy of `{trait_name}Handlers`.")
        for_line = (
            f"    shop_proto::{gen_mod}::services::{trait_name}Handlers for {struct_name} {{"
        )
        if len(for_line) <= 100:
            out.append(for_line)
        else:
            out.append(f"    shop_proto::{gen_mod}::services::{trait_name}Handlers")
            out.append(f"        for {struct_name} {{")
        client_tail = "<tonic::transport::Channel>,"
        if len(client_head) + len(client_tail) <= 100:
            out.append(client_head + client_tail)
        elif len(client_head) + 1 <= 100:
            out.append(client_head + "<")
            out.append("            tonic::transport::Channel,")
            out.append("        >,")
        else:
            out.append(f"        client: {pkg}_v1::{client_mod}::")
            out.append(f"            {client_ty}<tonic::transport::Channel>,")
        out.append(f"        behaviors: {behaviors},")
        out.append("        methods: [")
        for method, kind, behavior, req_ty, resp_ty in kinds:
            mname = f"r#{method}" if method in RUST_KEYWORDS else method
            req = shorten(req_ty, used_pkgs)
            resp = shorten(resp_ty, used_pkgs)
            suffix = f" => {behavior}," if kind in ("hand", "renamed") else ","
            line = f"            {kind} {mname}({req}) -> {resp}{suffix}"
            if len(line) <= 100:
                out.append(line)
            elif len(f"            {kind} {mname}({req})") <= 100:
                out.append(f"            {kind} {mname}({req})")
                out.append(f"                -> {resp}{suffix}")
            else:
                out.append(f"            {kind} {mname}(")
                out.append(f"                {req}")
                out.append(f"            ) -> {resp}{suffix}")
        out.append("        ]")
        out.append("    }")
        out.append("}")
        out.append("")
        emitted += 1

    print("\n".join(out))
    print(f"// emitted {emitted} proxy impls", file=sys.stderr)


if __name__ == "__main__":
    main()
