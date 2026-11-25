/// The auth-free whitelist — the operations that bypass the authn gate,
/// one table shared by both gateway faces (build-script scope and the
/// crate's own re-export).
///
/// Mirrors the Go deployment's `rpc.AddWhiteList(...)` calls
/// (admin/app `rest_server.go`): the refresh endpoint must stay free —
/// an expired access token is the very scenario that triggers the
/// refresh, demanding a valid AT here would deadlock the session's
/// renewal chain. The storefront's browse surface (catalog, active
/// promotions/seckills/groupons, comment ratings, AI search rewrite) is
/// anonymous by design.

pub const AUTH_FREE: &[(&str, &str)] = &[
    // ---- admin gateway: the login chain ----
    ("admin.service.v1.AuthenticationService", "Login"),
    ("admin.service.v1.AuthenticationService", "GenerateCaptcha"),
    ("admin.service.v1.AuthenticationService", "VerifyCaptcha"),
    ("admin.service.v1.AuthenticationService", "RefreshToken"),
    // ---- app gateway: the login chain ----
    ("app.service.v1.AuthenticationService", "Login"),
    ("app.service.v1.AuthenticationService", "Register"),
    ("app.service.v1.AuthenticationService", "SendResetCode"),
    ("app.service.v1.AuthenticationService", "ResetPassword"),
    // ---- app gateway: the anonymous browse surface ----
    ("app.service.v1.CategoryService", "List"),
    ("app.service.v1.CategoryService", "Get"),
    ("app.service.v1.BrandService", "List"),
    ("app.service.v1.BrandService", "Get"),
    ("app.service.v1.ProductService", "List"),
    ("app.service.v1.ProductService", "Get"),
    ("app.service.v1.ProductService", "SearchProducts"),
    ("app.service.v1.ProductService", "SearchProductSuggestions"),
    ("app.service.v1.PromotionService", "ListActivePromotions"),
    ("app.service.v1.SeckillService", "ListActiveSeckills"),
    ("app.service.v1.GrouponService", "ListActiveGroupons"),
    ("app.service.v1.ProductAttributeService", "List"),
    ("app.service.v1.ProductAttributeService", "Get"),
    ("app.service.v1.ProductAttributeValueService", "List"),
    ("app.service.v1.ProductAttributeValueService", "Get"),
    ("app.service.v1.SkuService", "List"),
    ("app.service.v1.SkuService", "Get"),
    ("app.service.v1.SkuPriceService", "List"),
    ("app.service.v1.SkuPriceService", "Get"),
    ("app.service.v1.SkuAttributeCombinationService", "List"),
    ("app.service.v1.SkuAttributeCombinationService", "Get"),
    ("app.service.v1.InteractionService", "GetCounts"),
    ("app.service.v1.CommentService", "GetProductRating"),
    ("app.service.v1.CommentService", "GetProductReviewSummary"),
    ("app.service.v1.AiChatService", "RewriteSearchQuery"),
    ("app.service.v1.CouponTemplateService", "List"),
];
