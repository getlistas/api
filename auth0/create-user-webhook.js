
// Custom Javascript snippet that runs in a secure, isolated sandbox in the
// Auth0 service as part of the authentication pipeline.
// https://manage.auth0.com/dashboard/us/doneq/rules
async function createUser(user, context, callback) {

  const API_URL = 'https://d1a3f8d8dae6.ngrok.io';
  const API_POST_LOGIN_WEBHOOK_ENDPOINT = '/webhooks/auth0/users';

  const axios = require('axios');
  const pRetry = require('p-retry');

  async function post() {
    await axios.post(API_URL + API_POST_LOGIN_WEBHOOK_ENDPOINT, user);
  }

  try {
    await pRetry(post, { retries: 10 });
    callback(null, user, context);
  } catch (err) {
    callback(err, user, context);
  }
}