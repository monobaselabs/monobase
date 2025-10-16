# Build stage
FROM oven/bun:1 AS builder

WORKDIR /app

# Copy workspace files
COPY package.json bun.lock ./
COPY apps/account/package.json ./apps/account/
COPY packages/sdk/package.json ./packages/sdk/
COPY packages/ui/package.json ./packages/ui/
COPY specs/api/package.json ./specs/api/
COPY packages/typescript-config/package.json ./packages/typescript-config/

# Install dependencies
RUN bun install

# Copy all source files
COPY . .

# Build the account app
WORKDIR /app/apps/account
RUN bun run build

# Production stage using Caddy to serve the SPA
FROM caddy:2.8-alpine

LABEL org.opencontainers.image.source=https://github.com/monobaselabs/monobase
LABEL org.opencontainers.image.description="Monobase Account App"
LABEL org.opencontainers.image.licenses=MIT

# Create user with UID 101 to match K8s security context
RUN addgroup -g 101 -S caddy-k8s && \
    adduser -u 101 -S -G caddy-k8s caddy-k8s

# Copy the dist folder to Caddy's default serve directory
COPY --from=builder /app/apps/account/dist /srv

# Copy Caddy configuration
COPY apps/account/Caddyfile /etc/caddy/Caddyfile

# Copy and set up entrypoint script
COPY apps/account/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

# Create necessary directories with proper permissions for non-root user
RUN mkdir -p /data /config /tmp && \
    chown -R 101:101 /data /config /tmp /srv && \
    chmod -R 755 /data /config /tmp /srv

# Ensure Caddy binary is executable by user 101 (critical for K8s security context)
RUN chown 101:101 /usr/bin/caddy && \
    chmod +x /usr/bin/caddy

# Set environment variables for runtime configuration
ENV PORT=8080
ENV API_URL="http://localhost:7213"
ENV ONESIGNAL_APP_ID=""
ENV XDG_DATA_HOME=/data
ENV XDG_CONFIG_HOME=/config

# Switch to non-root user
USER 101:101

# Expose port (changed from 80 to 8080 for non-root)
EXPOSE 8080

# Use entrypoint to generate config.json before starting Caddy
ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile", "--adapter", "caddyfile"]
