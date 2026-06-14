# Builds the dashboard and serves the static bundle with nginx.
# The browser talks to NATS directly over ws://127.0.0.1:8080 (published by the
# `nats` service), so nginx only needs to serve the SPA.
FROM node:22-alpine AS build
WORKDIR /app
RUN corepack enable
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM nginx:alpine
COPY --from=build /app/dist /usr/share/nginx/html
