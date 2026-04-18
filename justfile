# 本地开发工作流
# 依赖：cargo install just, cargo install cargo-watch

# 启动开发基础设施 + 迁移 + 启动后端（热重载）
dev:
    docker compose --env-file .env up -d
    docker compose --env-file .env exec postgres bash -c 'until pg_isready -U "$POSTGRES_USER" -d "$POSTGRES_DB"; do sleep 0.5; done'
    cargo run -p migration -- up
    cargo watch -w src -w Cargo.toml -w Cargo.lock -x "run -F swagger"

# 停止开发基础设施
dev-down:
    docker compose --env-file .env down

# 本地构建全栈（在 dev 基础上加 app 容器）
build:
    docker compose --env-file .env -f docker-compose.yml -f docker-compose.build.yml up -d --build

# 停止构建全栈
build-down:
    docker compose --env-file .env -f docker-compose.yml -f docker-compose.build.yml down

# 运行数据库迁移（up）
migrate-up:
    cargo run -p migration -- up

# 运行数据库迁移（down）
migrate-down:
    cargo run -p migration -- down



# 启动测试数据库
test-db:
    docker compose --env-file .env.test up -d
    docker compose --env-file .env.test exec postgres bash -c 'until pg_isready -U $${POSTGRES_USER} -d $${POSTGRES_DB}; do sleep 0.5; done'

# 停止测试数据库
test-db-down:
    docker compose --env-file .env.test down

# 准备测试环境（启动测试数据库并应用 migration/seed）
test-prepare: test-db
    just --dotenv-path .env.test --command cargo run -p migration -- up

# 使用 nextest 运行测试
test-nextest: test-prepare
    bash -lc 'set -euo pipefail; trap "just test-db-down" EXIT; just --dotenv-path .env.test --command cargo nextest run --all-features'

# 运行文档测试
test-doc: test-prepare
    just --dotenv-path .env.test --command cargo test --quiet --workspace --doc

# 运行完整测试（准备测试环境 + nextest + doctest）
test: test-nextest test-doc



# 运行所有 CI 检查
check:
    cargo check --quiet --workspace --all-targets
    cargo fmt --all -- --check
    cargo clippy --quiet --workspace --all-targets --all-features -- -D warnings -W clippy::all

# 运行完整 CI（check + test）
ci: check test

# 运行 cargo-deny 和 typos（需额外安装）
audit:
    cargo deny check -d
    typos
