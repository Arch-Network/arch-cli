FROM node:18.20.0-slim

WORKDIR /usr/src/app

# Copy package files
COPY package*.json ./

# Copy the rest of the application
COPY . .

# Install dependencies
RUN rm -rf node_modules && npm install


# Set environment variables
ARG DEMO_FRONTEND_PORT
ENV DEMO_FRONTEND_PORT=${DEMO_FRONTEND_PORT}
ARG INDEXER_PORT
ENV INDEXER_PORT=${INDEXER_PORT}

# Ensure proper permissions
RUN chown -R node:node /usr/src/app
USER node

EXPOSE ${DEMO_FRONTEND_PORT}

HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
  CMD node healthcheck.js || exit 1

# Use the start script instead of dev for containerized environments
CMD ["npm", "run", "start"]