# syntax=docker/dockerfile:1.10

# Coordinate this one pin with the resolved luneth bridge package-lock.json.
ARG PLAYWRIGHT_VERSION=1.59.1

# Named contexts replace these empty stages only when explicitly requested.
FROM scratch AS vendor-input
FROM scratch AS browser-input
FROM node:22-bookworm-slim AS node-runtime

FROM rust:1.92-slim-bookworm AS builder
ARG APT_MIRROR=""
ARG PLAYWRIGHT_VERSION
ARG USE_VENDOR=0
ENV RUSTUP_TOOLCHAIN=1.92
SHELL ["/bin/bash", "-euo", "pipefail", "-c"]
WORKDIR /app

# Mirrors are optional public origins serving /debian and /debian-security.
RUN if [[ -n "$APT_MIRROR" ]]; then \
      for file in /etc/apt/sources.list /etc/apt/sources.list.d/debian.sources; do \
        [[ ! -f "$file" ]] || sed -i "s|http://deb.debian.org|${APT_MIRROR}|g; s|http://security.debian.org|${APT_MIRROR}|g" "$file"; \
      done; \
    fi \
 && apt-get update \
 && apt-get install -y --no-install-recommends libssl-dev pkg-config git ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Node is used here only to validate the private bridge's resolved version.
COPY --from=node-runtime /usr/local/bin/node /usr/local/bin/node
COPY Cargo.toml Cargo.lock rust-toolchain ./
COPY .cargo/config.toml .cargo/config.toml
COPY .github/scripts/fetch-dependencies.sh .github/scripts/fetch-dependencies.sh
COPY src/ src/
COPY migration/Cargo.toml migration/Cargo.toml
COPY migration/src/ migration/src/

# Credentials exist only for this fetch process. Private Cargo sources may remain
# in local builder layers; never export intermediate/max caches or build records.
# Secret contents do not invalidate cached layers, so track the normalized mapping.
ARG VENDOR_CONFIG_SHA256=""
RUN --mount=from=vendor-input,target=/app/vendor \
    --mount=type=secret,id=cargo_vendor_config \
    --mount=type=secret,id=git_credentials,env=GIT_CREDENTIALS \
    if [[ "$USE_VENDOR" == 1 ]]; then \
      test -s /run/secrets/cargo_vendor_config; \
      printf '%s  %s\n' "$VENDOR_CONFIG_SHA256" /run/secrets/cargo_vendor_config \
        | sha256sum --check --status; \
      cat /run/secrets/cargo_vendor_config >> .cargo/config.toml; \
    else \
      bash .github/scripts/fetch-dependencies.sh; \
    fi

RUN --mount=from=vendor-input,target=/app/vendor \
    cargo metadata --locked --offline --no-default-features --format-version 1 \
      | node -e 'const fs = require("fs"), path = require("path"); \
        const packages = JSON.parse(fs.readFileSync(0, "utf8")).packages; \
        const bridge = packages.find(p => p.name === "luneth"); \
        const lock = JSON.parse(fs.readFileSync(path.join(path.dirname(bridge.manifest_path), "playwright-bridge/package-lock.json"))); \
        if (lock.packages["node_modules/playwright"].version !== process.env.PLAYWRIGHT_VERSION) \
          throw new Error("Update the Dockerfile Playwright pin to match the locked luneth bridge");' \
 && cargo build --locked --release --no-default-features --offline --workspace --bins

# One runtime for online and cached-input builds; never copy Cargo or app sources.
FROM node-runtime AS runtime
ARG APT_MIRROR=""
ARG NPM_REGISTRY=""
ARG PLAYWRIGHT_VERSION
ARG USE_BROWSER_CACHE=0
SHELL ["/bin/bash", "-euo", "pipefail", "-c"]
WORKDIR /app

RUN if [[ -n "$APT_MIRROR" ]]; then \
      for file in /etc/apt/sources.list /etc/apt/sources.list.d/debian.sources; do \
        [[ ! -f "$file" ]] || sed -i "s|http://deb.debian.org|${APT_MIRROR}|g; s|http://security.debian.org|${APT_MIRROR}|g" "$file"; \
      done; \
    fi \
 && apt-get update \
 && apt-get install -y --no-install-recommends libssl3 ca-certificates curl file \
 && if [[ -n "$NPM_REGISTRY" ]]; then npm config set registry "$NPM_REGISTRY"; fi \
 && npm install -g "playwright@$PLAYWRIGHT_VERSION" \
 && npx playwright install-deps chromium \
 && rm -rf /var/lib/apt/lists/*

RUN --mount=from=browser-input,target=/browser-input \
    if [[ "$USE_BROWSER_CACHE" == 1 ]]; then \
      mkdir -p /root/.cache/ms-playwright; \
      cp -a /browser-input/chromium-* /browser-input/chromium_headless_shell-* \
        /browser-input/ffmpeg-* /root/.cache/ms-playwright/; \
      node -e 'const fs = require("fs"); \
        const playwright = require("/usr/local/lib/node_modules/playwright"); \
        const browsers = require("/usr/local/lib/node_modules/playwright/node_modules/playwright-core/browsers.json").browsers; \
        for (const name of ["chromium", "chromium-headless-shell", "ffmpeg"]) { \
          const browser = browsers.find(b => b.name === name); \
          const directory = "/root/.cache/ms-playwright/" + name.replaceAll("-", "_") + "-" + browser.revision; \
          if (!fs.existsSync(directory)) throw new Error("Browser cache does not match the pinned Playwright version"); \
        } \
        fs.accessSync(playwright.chromium.executablePath(), fs.constants.X_OK);'; \
    else \
      npx playwright install chromium; \
    fi

COPY --from=builder /app/target/release/lunirelust /app/lunirelust
COPY --from=builder /app/target/release/migration /app/migration
ENV RUST_LOG=info,sqlx=warn
ENTRYPOINT ["/app/lunirelust"]
