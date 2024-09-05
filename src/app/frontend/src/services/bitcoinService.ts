import axios from 'axios';

const API_URL = 'http://localhost:18443'; // Adjust to your regtest server's address

export const bitcoinService = {
  getBalance: async (address: any) => {
    const response = await axios.post(API_URL, {
      jsonrpc: '1.0',
      id: 'curltest',
      method: 'getbalance',
      params: [address]
    }, {
      auth: {
        username: 'bitcoin',
        password: 'password'
      }
    });
    return response.data.result;
  },
  // Add other methods as needed
};