# JusticeAI

## Docker 启动（推荐）

项目已补充容器化文件：

- `backend-rust/Dockerfile`
- `frontend-vue/Dockerfile`
- `frontend-vue/nginx.conf`
- `docker-compose.yml`
- `.env.docker.example`

### Docker 持久化目录

`docker-compose.yml` 默认把数据持久化挂载到仓库根目录下的本地文件夹：

- `./docker-data/postgres`
- `./docker-data/runtime/uploads`
- `./docker-data/runtime/reports`
- `./docker-data/runtime/training`

这些目录中的数据保存在宿主机本地，不会只留在容器内部匿名卷中。

### 启动方式

先确保 Docker Desktop 已启动，然后在仓库根目录执行：

```bash
docker compose up --build -d
```

启动后访问：

- 前端：`http://127.0.0.1:18100`
- 农民工移动端：`http://127.0.0.1:18101`
- 后端：`http://127.0.0.1:8088`
- 健康检查：`http://127.0.0.1:8088/api/health`

停止：

```bash
docker compose down
```

查看日志：

```bash
docker compose logs -f
```

### 当前容器方案说明

- PostgreSQL 运行在容器内，并把数据挂载到 `./docker-data/postgres`
- 后端上传/报告/训练目录挂载到 `./docker-data/runtime/*`
- 前端由 Nginx 提供静态文件，并将 `/api/*` 反向代理到后端容器
- HugeGraph、Milvus、vLLM 当前默认通过 `host.docker.internal` 访问宿主机上的现有服务
- JusticeAI 后续接入大模型时，统一按 vLLM 的 OpenAI 兼容 ChatCompletion 接口对接：`POST ${VLLM__BASE_URL}/chat/completions`
- `VLLM__BASE_URL` 约定为 `/v1` 根路径，例如 `http://127.0.0.1:8000/v1`
- 如果宿主机没有启动 HugeGraph、Milvus、vLLM，后端仍可启动，但 `/api/health` 会显示降级

## 本地启动（手动，不自动拉起后端）

1. 复制仓库根目录 `.env.example` 为 `.env`
2. 按你的本地环境修改 `.env`
3. 启动后端：

```bash
cargo run --manifest-path backend-rust/Cargo.toml
```

4. 安装前端依赖：

```bash
npm --prefix frontend-vue install
```

5. 启动前端：

```bash
npm --prefix frontend-vue run dev
```

6. 如需单独启动农民工移动端：

```bash
$env:VITE_APP_ENTRY='mobile'; $env:VITE_DEV_PORT='18101'; npm --prefix frontend-vue run dev:mobile
```

7. 打开前端页面后，先进入 `/setup` 检查后端状态，再进入 `/imports`

## 配置入口

优先修改仓库根目录：

- `.env`：本地统一配置入口
- `.env.example`：本地开发配置示例
- `.env.docker.example`：Docker 运行配置参考

后端还提供这些参考文件：

- `backend-rust/.env.example`
- `backend-rust/config/default.toml`
- `backend-rust/config/development.toml`
- `backend-rust/config/production.toml`

## 当前说明

- 前端不会自动拉起后端
- 当前后端启动硬依赖 PostgreSQL
- 如果 `/setup` 页面提示后端未连接，请先确认后端进程或 Docker 容器已启动
- 如果需要修改接口地址、端口、存储目录、数据库连接等，优先修改根目录 `.env` 或 `docker-compose.yml`
- 后续接入模型对话时，默认兼容 vLLM 的 OpenAI ChatCompletion 接口，而不是自定义私有协议
