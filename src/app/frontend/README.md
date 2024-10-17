# Arch Network Demo Frontend

This is the frontend application for the Arch Network demo. It provides a user interface for interacting with the Arch Network and viewing network statistics.

## Prerequisites

Before you begin, ensure you have the following installed:
- Node.js (version 18 or later)
- npm (usually comes with Node.js)

## Getting Started

1. Install dependencies:
   ```
   npm install
   ```

2. Set up environment variables:
   - Copy the `.env.example` file to `.env`:
     ```
     cp .env.example .env
     ```
   - Open the `.env` file and update the variables as needed. For example:
     ```
     VITE_INDEXER_API_URL=http://localhost:5175/api
     ```

## Running the Application

### Using Docker (Recommended)

If you're using the Arch CLI, you can start the entire demo application (including the frontend) using:

```
arch-cli demo start
```

This will use the Docker setup and handle all the necessary configurations.

### Manual Start

If you want to run the frontend server manually without Docker:

1. Ensure you've set up the `.env` file as described in the "Getting Started" section.

2. Start the development server:
   ```
   INDEXER_PORT=5175 npm run dev
   ```
   Replace `5175` with the actual port your indexer is running on if it's different.

3. The application should now be running. Open your browser and navigate to:
   ```
   http://localhost:5173
   ```

## Development

- The main application code is located in the `src` directory.
- To add new components, place them in the `src/components` directory.
- Styles are managed using Tailwind CSS. You can customize the configuration in `tailwind.config.js`.

## Building for Production

To create a production build:

```
npm run build
```

This will generate optimized static files in the `dist` directory.

## Troubleshooting

- If you encounter CORS issues, ensure that your backend services (indexer, Arch Network nodes) are configured to allow requests from your frontend origin.
- Check that the `VITE_INDEXER_API_URL` in your `.env` file points to the correct indexer endpoint.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.