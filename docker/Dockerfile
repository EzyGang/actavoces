# Use Node.js v20 with Alpine for a smaller base image
FROM node:20-alpine AS builder

# Install pnpm globally using corepack
RUN corepack enable && corepack prepare pnpm@latest --activate

# Set working directory
WORKDIR /app

# Copy package files
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Install dependencies with frozen lockfile for reproducible builds
RUN pnpm install --frozen-lockfile

# Copy source files
COPY . .

ARG RELEASE_VERSION=0.0.0
ARG ENVIRONMENT=production

# Build the landing page
RUN pnpm build:landing \
    && mkdir -p /dist \
    && cp -r dist-landing/* /dist/ \
    && rm -rf /dist/assets/*.map \
    # Cleanup unnecessary files and dependencies
    && rm -rf node_modules \
    && pnpm store prune \
    && rm -rf /root/.local/share/pnpm/store \
    && rm -rf /root/.cache \
    && rm -rf /app/* \
    && corepack disable \
    && rm -rf /usr/local/share/.cache \
    && rm -rf /tmp/*

# Set the working directory to /dist where our built files are
WORKDIR /dist

# Keep container running for file extraction
CMD ["tail", "-f", "/dev/null"]

FROM nginx:alpine AS runner

# Copy build to /dist
COPY --from=builder /dist /dist
COPY ./docker/nginx.conf /etc/nginx/conf.d/default.conf

# Replace default NGINX root with a symlink to /dist
RUN rm -rf /usr/share/nginx/html && ln -s /dist /usr/share/nginx/html

EXPOSE 80
