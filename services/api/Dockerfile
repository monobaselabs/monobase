# Multi-stage Dockerfile with bundling
# Build from monorepo root: docker build -f services/api/Dockerfile -t monobase-api .
# Note: Automatic Bun package caching is enabled via BuildKit cache mounts for faster builds

# Stage 1: Build API Specifications
FROM oven/bun:1.2.21-alpine AS spec-builder

WORKDIR /specs

# Copy specs source (assumes build from monorepo root)
COPY specs/api/package.json ./
COPY specs/api/src ./src/
COPY specs/api/scripts ./scripts/
COPY specs/api/tspconfig.yaml ./

# Install spec dependencies and build using bun (faster than npm)
RUN --mount=type=cache,target=/home/bun/.bun/install/cache bun install
RUN bun run build

# Stage 2: Bundle API Application
FROM oven/bun:1.2.21-alpine AS api-builder

WORKDIR /app

# Copy built specs from spec-builder
COPY --from=spec-builder /specs/dist ./specs/api/dist
COPY --from=spec-builder /specs/package.json ./specs/api/

# Copy API service files
COPY services/api/package.json ./
RUN cat package.json | \
    sed 's/"@monobase\/api-spec": "workspace:\*"/"@monobase\/api-spec": "file:specs\/api"/' | \
    sed 's/"@monobase\/typescript-config": "workspace:\*",//' > package.temp.json && \
    mv package.temp.json package.json

# Install ALL dependencies (including devDependencies for build)
RUN --mount=type=cache,target=/home/bun/.bun/install/cache bun install

# Copy source code for bundling (including generated files)
COPY services/api/src ./src
COPY services/api/scripts ./scripts
COPY services/api/tsconfig.json ./
COPY services/api/drizzle.config.ts ./

# Bundle the application
RUN bun run bundle

# Stage 3: Runtime
FROM oven/bun:1.2.21-alpine

LABEL org.opencontainers.image.source=https://github.com/monobaselabs/monobase
LABEL org.opencontainers.image.description="Monobase API Service"
LABEL org.opencontainers.image.licenses=MIT

WORKDIR /app

# Install wget for healthcheck
RUN apk add --no-cache wget

# Copy built specs from spec-builder
COPY --from=spec-builder /specs/dist ./specs/api/dist
COPY --from=spec-builder /specs/package.json ./specs/api/

# Copy bundled application from api-builder
COPY --from=api-builder /app/dist ./dist

# Copy migrations folder (required by Drizzle ORM)
COPY services/api/src/generated/migrations ./src/generated/migrations

# Copy package.json and install production dependencies only
COPY services/api/package.json ./
RUN cat package.json | \
    sed 's/"@monobase\/api-spec": "workspace:\*"/"@monobase\/api-spec": "file:specs\/api"/' | \
    sed 's/"@monobase\/typescript-config": "workspace:\*",//' > package.temp.json && \
    mv package.temp.json package.json

RUN --mount=type=cache,target=/home/bun/.bun/install/cache bun install --production

# Expose port (default production port)
EXPOSE 7213

# Health check (using PORT env variable for flexibility)
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:${PORT:-7213}/readyz || exit 1

# Start the application using bundled code
CMD ["bun", "run", "start"]
