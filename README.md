# Runtime Tracker - Cloudflare Workers (Rust)

使用 Rust 重构的设备应用使用时间追踪系统后端，部署在 Cloudflare Workers 上。

## 技术栈

- **语言**: Rust (编译为 WebAssembly)
- **运行时**: Cloudflare Workers
- **数据库**: Cloudflare D1 (SQLite)
- **缓存/状态**: Cloudflare Workers KV
- **定时任务**: Cloudflare Cron Triggers

## 与原始 Node.js 版本的对应关系

| 原始 (Node.js) | 重构 (Rust/CF Workers) |
|---|---|
| Express.js | `worker` crate Router |
| MongoDB (Mongoose) | Cloudflare D1 (SQL) |
| 内存 Map (电池/切换记录) | Workers KV |
| node-cron | Cron Triggers |
| JWT (jsonwebtoken) | KV Token 存储 |
| bcrypt 密码验证 | 明文比较 (CF 加密环境变量) |
| .env 文件配置 | wrangler.toml + KV 运行时覆盖 |
| PM2 进程管理 | Workers 自动伸缩 |

## 项目结构

```
src/
├── lib.rs              # 入口点 (fetch + scheduled 处理器)
├── models.rs           # 数据模型 (请求/响应/数据库行类型)
├── db.rs               # D1 数据库操作层
├── kv.rs               # KV 存储操作层
├── utils.rs            # 工具函数 (时区/日期范围/辅助)
├── routes/
│   ├── mod.rs
│   ├── api.rs          # 公开 API 路由 (/api/*)
│   ├── admin.rs        # 管理后台路由 (/admin/*)
│   └── eyetime.rs      # 用眼时长路由 (/api/eyetime/*)
└── services/
    ├── mod.rs
    ├── recorder.rs     # 统计记录服务
    ├── query.rs        # 统计查询服务
    ├── eyetime.rs      # 用眼时长记录 & 查询
    └── ai.rs           # AI 总结服务
```

## API 路由

### 公开 API (`/api`)

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/api` | 应用上报 (电池 + 应用状态) |
| GET | `/api/devices` | 获取设备列表 |
| GET | `/api/recent/:deviceId` | 获取设备最近切换记录 |
| GET | `/api/stats/:deviceId` | 当日统计 |
| GET | `/api/weekly/:deviceId` | 周统计 |
| GET | `/api/monthly/:deviceId` | 月统计 |
| GET | `/api/ai/summary/:deviceId` | 获取 AI 总结 |
| GET | `/api/ai/summaries` | 获取所有 AI 总结 |
| GET | `/api/ai/trigger/:deviceId` | 触发 AI 总结 |
| GET | `/api/ai/status` | AI 状态 |
| GET | `/api/pageConfig` | 页面配置 |
| GET | `/api/ip` | 客户端 IP |

### 用眼时长 (`/api/eyetime`)

| 方法 | 路径 | 说明 |
|---|---|---|
| GET | `/api/eyetime/daily` | 日用眼统计 |
| GET | `/api/eyetime/weekly` | 周用眼统计 |
| GET | `/api/eyetime/monthly` | 月用眼统计 |

### 管理后台 (`/admin`)

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/admin/login` | 管理员登录 |
| POST | `/admin/account/update` | 更新账户 |
| POST | `/admin/ai/trigger/:deviceId` | 触发 AI 总结 |
| GET | `/admin/config` | 获取配置 |
| POST | `/admin/config` | 更新配置 |

## 快速开始

详见 [一键部署 & 更新教程](DEPLOYMENT.md)，只需配置 GitHub Secrets 并点击一次按钮即可完成部署。

## 数据存储设计

### D1 表结构

**daily_stats** - 应用使用统计
```sql
CREATE TABLE daily_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT NOT NULL,
    date TEXT NOT NULL,              -- YYYY-MM-DD (本地时区)
    app_name TEXT NOT NULL,
    package_name TEXT NOT NULL DEFAULT '',
    hourly_usage TEXT NOT NULL,      -- JSON: [24个浮点数]
    UNIQUE(device_id, date, app_name, package_name)
);
```

**daily_eye_time** - 用眼时长
```sql
CREATE TABLE daily_eye_time (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,       -- YYYY-MM-DD (本地时区)
    hourly_usage TEXT NOT NULL       -- JSON: [24个浮点数]
);
```

### KV 键设计

| 键模式 | 说明 | TTL |
|---|---|---|
| `devices` | 设备ID列表 | 永久 |
| `battery:{deviceId}` | 电池信息 | 7天 |
| `recent:{deviceId}` | 最近切换记录 (max 20) | 7天 |
| `ai_summary:{deviceId}` | AI 总结缓存 | 30天 |
| `eye_device:{deviceId}` | 用眼设备状态 | 1天 |
| `eye_global` | 全局用眼状态 | 1天 |
| `auth_token:{token}` | 认证令牌 | 7天 |
| `runtime_config` | 运行时配置覆盖 | 永久 |

## 许可证

MIT
