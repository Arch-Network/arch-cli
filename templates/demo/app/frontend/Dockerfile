FROM node:18.20.0-slim

WORKDIR /usr/src/app

# Copy package files
COPY package*.json ./

# Copy the rest of the application
COPY . .

# Install dependencies
RUN rm -rf node_modules && npm install

# Build the application if in production mode
ARG NODE_ENV
RUN if [ "$NODE_ENV" = "production" ]; then npm run build; fi

# Set environment variables
ARG DEMO_FRONTEND_PORT
ENV DEMO_FRONTEND_PORT=${DEMO_FRONTEND_PORT}
ARG INDEXER_PORT
ENV INDEXER_PORT=${INDEXER_PORT}
ARG VITE_PROGRAM_PUBKEY
ENV VITE_PROGRAM_PUBKEY=${VITE_PROGRAM_PUBKEY}
ARG VITE_WALL_ACCOUNT_PUBKEY
ENV VITE_WALL_ACCOUNT_PUBKEY=${VITE_WALL_ACCOUNT_PUBKEY}
ARG VITE_RPC_URL
ENV VITE_RPC_URL=${VITE_RPC_URL}

# Ensure proper permissions
RUN chown -R node:node /usr/src/app
USER node

EXPOSE ${DEMO_FRONTEND_PORT}

HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
  CMD node healthcheck.js || exit 1

# Use different commands based on environment
# Start the application
CMD ["sh", "-c", "if [ \"$NODE_ENV\" = \"production\" ]; then npm run preview; else npm run start; fi"]