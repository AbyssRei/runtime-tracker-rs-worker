# 一键部署 & 更新教程

本文档指导你通过 GitHub Actions 将 Runtime Tracker Worker 一键部署到 Cloudflare Workers，并支持后续代码推送自动更新。

---

## 目录

- [前置条件](#前置条件)
- [第一步：获取并配置 GitHub Secrets](#第一步获取并配置-github-secrets)
- [第二步：一键初始化 & 首次部署](#第二步一键初始化--首次部署)
- [后续更新](#后续更新)
- [前端静态资源](#前端静态资源)
- [FAQ](#faq)

---

## 前置条件

- 一个 [Cloudflare](https://dash.cloudflare.com/sign-up) 账号（免费计划即可）
- 本仓库已 Fork 或推送到你的 GitHub 账号下

> 不需要在本地安装任何工具，所有操作均通过 GitHub Actions 自动完成。

---

## 第一步：获取并配置 GitHub Secrets

进入你的 GitHub 仓库页面：**Settings** → **Secrets and variables** → **Actions** → **New repository secret**

需要配置以下 5 个 Secrets：

### 1. `CLOUDFLARE_API_TOKEN`

Cloudflare API Token，用于 GitHub Actions 操作你的 Cloudflare 账号。

**获取步骤：**

1. 打开 [Cloudflare API Tokens 页面](https://dash.cloudflare.com/profile/api-tokens)
2. 点击 **Create Token**
3. 选择 **Create Custom Token**
4. 配置以下权限：

   | 权限项 | 权限级别 |
   |---|---|
   | **Account** → Workers Scripts | Edit |
   | **Account** → Workers KV Storage | Edit |
   | **Account** → D1 | Edit |
   | **Account** → Workers Tail | Read |

5. **Account Resources**：选择 `Include → 你的账号`
6. 点击 **Continue to summary** → **Create Token**
7. 复制生成的 Token（**仅显示一次**）

> 将此 Token 作为 `CLOUDFLARE_API_TOKEN` 添加到 GitHub Secrets。

### 2. `CLOUDFLARE_ACCOUNT_ID`

Cloudflare 账户 ID，用于标识你的账号。

**获取步骤：**

1. 登录 [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. 点击左侧菜单 **Workers & Pages**
3. 右侧面板 **Account ID** 即是（32 位十六进制字符串）

或者也可以在任意域名的 **Overview** 页面右下角的 **API** 区域找到。

> 将此 ID 作为 `CLOUDFLARE_ACCOUNT_ID` 添加到 GitHub Secrets。

### 3. `SECRET`

应用鉴权密钥，客户端上报数据时携带此密钥进行身份验证。

**设置方式：**

自行生成一个强随机字符串，例如：

```bash
# Linux/macOS
openssl rand -hex 32

# 或使用 Python
python3 -c "import secrets; print(secrets.token_hex(32))"
```

> 将生成的字符串作为 `SECRET` 添加到 GitHub Secrets。
> 同时确保你的客户端（上报设备）也使用相同的密钥。

### 4. `ADMIN_PASSWD`

管理后台登录密码。

**设置方式：**

自行设定一个强密码即可。该密码用于 `/admin/login` 接口进行管理员身份验证。

> 将密码作为 `ADMIN_PASSWD` 添加到 GitHub Secrets。

### 5. `AI_API_KEY`

AI 总结功能所使用的 API Key（调用 OpenAI 兼容接口）。

**获取步骤：**

- **OpenAI**：前往 [OpenAI API Keys](https://platform.openai.com/api-keys) → 创建新 Key
- **其他 OpenAI 兼容服务**（如 DeepSeek、Moonshot 等）：在对应平台获取 API Key

> 将 API Key 作为 `AI_API_KEY` 添加到 GitHub Secrets。
> 如不需要 AI 总结功能，可将 `wrangler.toml` 中 `AI_SUMMARY_ENABLED` 设为 `"false"`，此 Secret 填任意值即可。

---

### Secrets 配置汇总

| Secret 名称 | 说明 | 来源 |
|---|---|---|
| `CLOUDFLARE_API_TOKEN` | CF API Token | [Cloudflare Dashboard](https://dash.cloudflare.com/profile/api-tokens) 创建 |
| `CLOUDFLARE_ACCOUNT_ID` | CF 账户 ID | Cloudflare Dashboard → Workers & Pages 页面 |
| `SECRET` | 应用鉴权密钥 | 自行生成随机字符串 |
| `ADMIN_PASSWD` | 管理后台密码 | 自行设定 |
| `AI_API_KEY` | AI 服务 API Key | OpenAI / 兼容服务平台获取 |

配置完成后，在 GitHub 仓库的 **Settings → Secrets → Actions** 页面应能看到 5 个 Secrets：

```
CLOUDFLARE_API_TOKEN    ••••••
CLOUDFLARE_ACCOUNT_ID   ••••••
SECRET                  ••••••
ADMIN_PASSWD            ••••••
AI_API_KEY              ••••••
```

---

## 第二步：一键初始化 & 首次部署

所有操作集中在同一个 **Deploy to Cloudflare Workers** Workflow 中。首次部署时开启 `init_resources` 即可自动完成：

- 创建 D1 数据库（已存在则跳过）
- 创建 KV 命名空间（已存在则跳过）
- 将资源 ID 自动写入 `wrangler.toml` 并提交到仓库
- 执行数据库迁移 & 部署 Worker

**操作步骤：**

1. 进入 GitHub 仓库页面，点击顶部 **Actions** 标签
2. 左侧选择 **Deploy to Cloudflare Workers**
3. 点击右侧 **Run workflow** 按钮
4. 设置参数：
   - **初始化 Cloudflare 资源**：选 `true`（⚠️ 首次部署必须选 true）
   - **执行数据库迁移**：保持 `false` 即可（`init_resources = true` 时会自动迁移）
5. 点击绿色 **Run workflow** 按钮

等待 Workflow 完成（全程约 5-15 分钟，首次构建较慢）。

部署成功后，你的 Worker 将运行在：`https://runtime-tracker.<your-subdomain>.workers.dev`

> 💡 **整个过程只需配置 Secrets + 点一次按钮**，无需在本地安装任何工具。

### 手动初始化（可选）

如果你更喜欢在本地手动创建资源，也可以这样做：

```bash
# 安装 wrangler CLI
npm install -g wrangler

# 登录 Cloudflare
wrangler login

# 创建 D1 数据库
wrangler d1 create runtime-tracker-db
# ✅ 记录输出中的 database_id

# 创建 KV 命名空间
wrangler kv namespace create runtime-tracker-kv
# ✅ 记录输出中的 id
```

然后编辑 `wrangler.toml`，将 `database_id` 和 KV `id` 替换为实际值，提交到仓库后手动触发 Workflow（`run_migrations = true`）。

---

## 后续更新

### 方式一：推送代码自动部署（推荐）

向 `main` 分支推送代码时，如果修改了以下路径的文件，会自动触发部署：

- `crates/**`（Worker 源码）
- `Cargo.toml` / `Cargo.lock`（依赖变更）

```bash
git add .
git commit -m "feat: 新功能"
git push origin main
# ✅ 自动触发部署，无需额外操作
```

### 方式二：手动触发部署

在 Actions 页面手动运行 **Deploy to Cloudflare Workers**：

- **仅更新代码**：两个选项都保持 `false`（默认）
- **代码 + 数据库迁移**：`run_migrations` 选 `true`（当有新的迁移文件时）
- **重新初始化资源**：`init_resources` 选 `true`（一般不需要，幂等安全）

---

## 前端静态资源

前端 Vue 3 SPA 位于 `frontend/` 子目录，与 Rust Worker 集成在同一个 Cloudflare Workers 部署中。

### 架构说明

| 组件 | 技术栈 | 说明 |
|---|---|---|
| 后端 API | Rust + Workers | 处理 `/api/*` 和 `/admin/*` 路由 |
| 前端 SPA | Vue 3 + Vite + Tailwind CSS | 其余所有路径由 Static Assets 服务 |

**工作原理**：
1. 所有请求优先进入 Rust Worker（`run_worker_first = true`）
2. 路径以 `/api` 或 `/admin` 开头 → Rust 路由处理
3. 其他路径 → 转发给 `ASSETS` fetcher 服务静态文件
4. 未匹配的路径（如 `/about`）→ 返回 `index.html`，由 Vue Router 处理（SPA 路由）

### 构建流程

CI 中的构建顺序：
```
frontend/ npm ci + npm run build  →  dist/（静态文件）
worker-build --release            →  build/worker/（WASM）
wrangler deploy --no-build        →  部署 dist/ + build/ 到 Cloudflare
```

本地构建（等同于 `wrangler dev` / `wrangler deploy` 时自动执行）：
```bash
cd frontend && npm ci && npm run build
cd .. && worker-build --release
```

### 自定义前端配置

修改 `frontend/src/config.js` 可配置：
- `SITE_*`：站点标题、描述等显示信息
- Giscus 评论配置（由后端 `pageConfig` API 动态下发，也可在 `wrangler.toml` 的 `[vars]` 中设置）

> **注意**：`API_BASE` 已设置为 `/api`（相对路径），前端与 Worker 同域部署无需修改。

---

## FAQ

### Q: 如何判断是否需要执行数据库迁移？

如果 `migrations/` 目录下有新增的 `.sql` 文件，部署时需要选择 `run_migrations = true`。如果只是修改了 Rust 代码，不需要迁移。

### Q: 部署失败怎么办？

1. 在 Actions 页面查看失败的 step 日志
2. 常见原因：
   - **Secrets 未配置**：检查 5 个 Secrets 是否都已正确配置
   - **API Token 权限不足**：确认 Token 包含 Workers Scripts (Edit)、KV Storage (Edit)、D1 (Edit) 权限
   - **资源未创建**：手动触发 Workflow 并将 `init_resources` 设为 `true`
   - **构建错误**：检查 Rust 代码是否有编译错误

### Q: init_resources 可以重复开启吗？

可以。资源初始化是幂等的——如果 D1 数据库和 KV 命名空间已存在，会自动跳过创建步骤，仅确保 `wrangler.toml` 中的资源 ID 是正确的。

### Q: 如何更新 Secrets？

进入 **Settings → Secrets → Actions**，点击对应 Secret 旁的更新按钮，输入新值即可。下次部署会自动使用新值。

### Q: 如何修改非敏感配置？

直接编辑 `wrangler.toml` 中的 `[vars]` 区域，提交推送后会自动部署生效。

### Q: 构建太慢怎么办？

首次构建需要编译所有依赖（约 5-10 分钟），后续构建会利用 GitHub Actions 的 Cargo 缓存，通常 2-4 分钟即可完成。

### Q: 如何查看部署后的 Worker URL？

部署成功后，在 Actions 日志的 Deploy 步骤输出中可以看到 Worker URL。也可以在 [Cloudflare Dashboard → Workers & Pages](https://dash.cloudflare.com/) 中查看。
