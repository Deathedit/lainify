# syntax=docker/dockerfile:1

# ── Build ────────────────────────────────────────────────────────────────────
# The output is a static site, so this stage always runs on the build host's
# native architecture — only the tiny runtime stage is per-platform.
FROM --platform=$BUILDPLATFORM node:22-alpine AS build

ARG PNPM_VERSION=12.9.0
RUN npm install -g pnpm@${PNPM_VERSION}

WORKDIR /app

COPY package.json pnpm-lock.yaml .npmrc ./
RUN --mount=type=cache,id=pnpm-store,target=/pnpm-store \
	pnpm install --frozen-lockfile --store-dir /pnpm-store

COPY . .
RUN pnpm build

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM nginxinc/nginx-unprivileged:stable-alpine

LABEL org.opencontainers.image.title="lainify" \
	org.opencontainers.image.description="Low-latency browser viewer for capture cards and webcams"

COPY docker/nginx.conf /etc/nginx/conf.d/default.conf
COPY docker/security-headers.conf /etc/nginx/snippets/security-headers.conf
COPY --from=build /app/build /usr/share/nginx/html

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
	CMD wget -qO /dev/null http://127.0.0.1:8080/ || exit 1
