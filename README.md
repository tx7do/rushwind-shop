<div align="center">

<img src="docs/brand/rushwind-tile.svg" width="120" alt="RushWind Shop｜风行商城" />

# RushWind Shop｜风行商城

[![Rust](https://img.shields.io/badge/Rust-1.81+-DEA584?logo=rust)](https://www.rust-lang.org/)
[![Vue](https://img.shields.io/badge/Vue-3.x-4FC08D?logo=vuedotjs)](https://vuejs.org/)
[![Nuxt](https://img.shields.io/badge/Nuxt-3.x-00DC82?logo=nuxt)](https://nuxt.com/)
[![Docker](https://img.shields.io/badge/Docker-Ready-2463EB?logo=docker)](https://www.docker.com/)

</div>

---

## 项目简介

RushWind Shop（风行商城）是一套面向出海场景的企业级多语言电商平台，后端基于
[RushWind](https://github.com/tx7do/rushwind) Rust 微服务框架，采用
**Protobuf-first（契约驱动）** 工作流；前端后台基于 Vue 3 + TypeScript，店铺前台基于
Nuxt + Vue 3。

本仓库是 [go-wind-shop](https://github.com/tx7do/go-wind-shop)（go-kratos 版风行商城）
的 Rust 复刻：**复用同一套 proto 契约、同一套前端**，后端从 Go/go-kratos 换为
Rust/RushWind——前端零改动即可在两个后端之间切换。

## 项目亮点

- **契约驱动（Protobuf-first）**：接口、数据模型、错误码均以 proto 为单一事实源；
  Rust 侧由 `rushwind-proto-build`（buf 编译）+ `rushwind-gen-http`（路由/绑定/trait/
  mount 生成）直接从契约生成 HTTP 面，由 tonic 生成域服务的 gRPC 面
- **三服务 BFF 架构**：`admin` / `app` 瘦网关 + `core` 核心实现，业务逻辑与传输层解耦
- **RushWind 全家桶装配**：authn-jwt + authn-gate（会话校验）、authz-rbac、
  SeaORM 存储（软删/缓存/观测装饰器）、Redis 缓存、apalis 异步任务、
  registry 服务发现、OpenTelemetry 链路追踪、SSE 推送
- **broker 引擎抽象**：订单/库存事件走 RushWind 的 14 引擎消息契约，
  开发环境 memory，生产环境一行配置切换 kafka/rabbitmq/…
- **前端零改动复用**：店铺前台与管理后台直接复用 go-wind-shop 的前端
- **生产就绪**：JWT 鉴权、RBAC 授权、Docker 一键部署

## 系统架构

```
HTTP/REST 请求
  └─ admin-service  (后台网关 BFF，REST:10600 / SSE:10601)
       └─ gRPC → core-service  (核心业务 + 数据落点)

app-service (前台网关 BFF，REST:10700) ──gRPC──→ core-service
core-service (gRPC，业务实现与数据访问)
```

| 服务 | 角色 | 是否访问 DB |
|------|------|:---:|
| `app/admin/service` | 后台网关 BFF（REST → gRPC 转发） | ❌ |
| `app/app/service` | 前台网关 BFF | ❌ |
| `app/core/service` | 核心业务实现（持久化、真实逻辑） | ✅ |

## 技术栈

| 层级 | 技术 |
|------|------|
| **后端框架** | `Rust` · `RushWind` · `Protobuf / Buf` · `tonic (gRPC)` |
| **ORM** | `SeaORM` · `PostgreSQL` · `MySQL` |
| **中间件** | `Redis` · `SeaweedFS`（S3 兼容对象存储）· `ElasticSearch / OpenSearch` · `etcd`（服务发现） |
| **认证授权** | `rushwind-authn-jwt`（JWT）· `rushwind-authn-gate`（会话白/黑名单）· `rushwind-authz-rbac` |
| **实时通信** | `SSE`（服务端推送）· `apalis-postgres`（异步任务） |
| **消息队列** | RushWind broker 契约（memory / kafka / rabbitmq / … 14 引擎） |
| **分布式** | `OpenTelemetry + Jaeger`（链路追踪） |
| **后台前端** | `Vue 3` · `TypeScript` · `Vite` · `Element Plus` |
| **店铺前端** | `Nuxt` · `Vue 3` · `TypeScript` · `Tailwind CSS` · `i18n` |
| **部署运维** | `Docker` · `Docker Compose` |

## 快速开始

### 环境要求

| 工具 | 版本 |
|------|------|
| Rust | 1.81+（stable 工具链） |
| buf | 1.50+（契约编译，PATH 可达） |
| Node.js | >= 20.10.0 |
| pnpm | >= 10.0.0 |
| Docker | 20.0+ |

> RushWind 框架以本地路径依赖接入（`../rushwind/crates/*`），构建前请确保
> rushwind 仓库与本仓库同级（或调整 `backend/Cargo.toml` 中的路径）。

### 启动中间件（推荐开发模式）

```bash
cd backend
docker compose -f docker-compose.libs.yaml up -d
```

### 运行后端

```bash
cd backend
cargo run -p core-service    # 核心业务（gRPC）
cargo run -p admin-service   # 后台网关（REST :10600）
cargo run -p app-service     # 前台网关（REST :10700）
```

### 启动前端

```bash
cd frontend/admin && pnpm install && pnpm dev    # 管理后台
cd frontend/app && pnpm install && pnpm dev      # 店铺前台
```

## 仓库结构

```
rushwind-shop/
├── backend/               # Rust 工作区
│   ├── api/               # proto 契约（buf workspace，与 go-wind-shop 同源同构）
│   ├── crates/proto/      # 契约 crate：buf → prost/pbjson 类型 + gen-http 路由面
│   ├── app/
│   │   ├── admin/service/ # 后台网关 BFF
│   │   ├── app/service/   # 前台网关 BFF
│   │   └── core/service/  # 核心业务（gRPC + SeaORM）
│   ├── pkg/               # 共享库（中间件装配、错误映射、配置）
│   └── sql/               # 数据库初始化与演示数据
├── frontend/              # 前端（复用 go-wind-shop）
│   ├── admin/             # 管理后台（Vue 3 + Element Plus）
│   └── app/               # 店铺前台（Nuxt 3）
├── docs/                  # 文档与品牌资产
└── frontend/*/deploy/     # 各前端的 nginx 部署配置
```

## License

[MIT](./LICENSE)
