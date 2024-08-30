FROM node:16 as builder

WORKDIR /workspace

RUN apt-get update
RUN apt-get install -y git

RUN git clone --branch v3.4.0  https://github.com/joundy/janoside-btc-rpc-explorer.git .

RUN npm install

FROM node:16-alpine

WORKDIR /workspace

COPY --from=builder /workspace .

EXPOSE 3002

STOPSIGNAL SIGINT

CMD ["npm", "start"]
