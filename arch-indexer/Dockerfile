FROM node:14

WORKDIR /usr/src/app

COPY package*.json ./

RUN npm install

COPY . .

ARG INDEXER_PORT
ENV INDEXER_PORT=${INDEXER_PORT}

EXPOSE ${INDEXER_PORT}

CMD [ "node", "src/index.js" ]