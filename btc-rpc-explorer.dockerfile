FROM node:16 as builder

WORKDIR /workspace

# Create a user and group
RUN groupadd -r appuser && useradd -r -g appuser appuser

# Switch to the new user
USER appuser

RUN apt-get update && \
    apt-get install -y --no-install-recommends git=1:2.30.2-1 && \
    git clone --branch v3.4.0 https://github.com/joundy/janoside-btc-rpc-explorer.git .

RUN npm install

FROM node:16-alpine

WORKDIR /workspace

COPY --from=builder /workspace .

EXPOSE 3002

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3002/ || exit 1

STOPSIGNAL SIGINT

CMD ["npm", "start"]
