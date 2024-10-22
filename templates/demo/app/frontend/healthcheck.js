const http = require('http');

const options = {
  host: 'localhost',
  port: process.env.DEMO_FRONTEND_PORT,
  timeout: 2000,
  path: '/health' // Adjust this to a suitable health check endpoint
};

const request = http.request(options, (res) => {
  console.log(`Health check status: ${res.statusCode}`);
  if (res.statusCode == 200) {
    process.exit(0);
  } else {
    process.exit(1);
  }
});

request.on('error', function(err) {
  console.error('Health check failed:', err);
  process.exit(1);
});

request.end();