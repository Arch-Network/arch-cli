FROM node:16 as builder

WORKDIR /workspace

RUN apt-get update && \
    apt-get install -y git && \
    git clone --branch v3.4.0 https://github.com/joundy/janoside-btc-rpc-explorer.git .

RUN npm install

FROM node:16-alpine

WORKDIR /workspace

COPY --from=builder /workspace .

EXPOSE 3003

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3003/ || exit 1

STOPSIGNAL SIGINT

CMD ["npm", "start"]
