# JusticeAI

## 本地启动（手动，不自动拉起后端）

1. 复制仓库根目录 `.env.example` 为 `.env`
2. 按你的本地环境修改 `.env`
3. 启动后端：

```bash
cargo run --manifest-path backend-rust/Cargo.toml
```

4. 启动前端：

```bash
npm --prefix frontend-vue run dev
```

5. 打开前端页面后，先进入 `/setup` 检查后端状态，再进入 `/imports`

## 配置入口

优先修改仓库根目录：

- `.env`：本地统一配置入口
- `.env.example`：根目录配置示例

后端还提供这些参考文件：

- `backend-rust/.env.example`
- `backend-rust/config/default.toml`
- `backend-rust/config/development.toml`
- `backend-rust/config/production.toml`

## 当前说明

- 前端不会自动拉起后端
- 如果 `/setup` 页面提示后端未连接，请先确认后端进程已启动
- 如果需要修改接口地址、端口、存储目录、数据库连接等，优先修改根目录 `.env`
