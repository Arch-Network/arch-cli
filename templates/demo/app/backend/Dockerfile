FROM node:16

WORKDIR /usr/src/app

COPY package*.json ./
RUN npm install

COPY . .

ARG DEMO_BACKEND_PORT
ENV DEMO_BACKEND_PORT=${DEMO_BACKEND_PORT}

EXPOSE ${DEMO_BACKEND_PORT}

CMD ["npm", "start"]