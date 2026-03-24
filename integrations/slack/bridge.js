#!/usr/bin/env node
// Slack bridge for Hydra — reads/writes JSON lines on stdio.
// Requires: npm install @slack/bolt
// Env: SLACK_BOT_TOKEN, SLACK_APP_TOKEN (for socket mode)

const readline = require('readline');

const BOT_TOKEN = process.env.SLACK_BOT_TOKEN;
const APP_TOKEN = process.env.SLACK_APP_TOKEN;
if (!BOT_TOKEN || !APP_TOKEN) {
  process.stderr.write('ERROR: SLACK_BOT_TOKEN and SLACK_APP_TOKEN must be set\n');
  process.exit(1);
}

let App;
try {
  App = require('@slack/bolt').App;
} catch {
  process.stderr.write('ERROR: npm install @slack/bolt\n');
  process.exit(1);
}

const app = new App({
  token: BOT_TOKEN,
  appToken: APP_TOKEN,
  socketMode: true,
});

// Incoming: Slack message → JSON line to stdout
app.message(async ({ message, say }) => {
  if (message.subtype) return; // skip system messages
  const event = {
    type: 'message',
    from: message.user,
    channel: message.channel,
    text: message.text || '',
    timestamp: new Date(parseFloat(message.ts) * 1000).toISOString(),
  };
  process.stdout.write(JSON.stringify(event) + '\n');
});

// Outgoing: JSON line from stdin → Slack reply
const rl = readline.createInterface({ input: process.stdin });
rl.on('line', async (line) => {
  try {
    const cmd = JSON.parse(line);
    if (cmd.type === 'init') {
      process.stdout.write(JSON.stringify({ type: 'ready' }) + '\n');
    } else if (cmd.type === 'ping') {
      process.stdout.write(JSON.stringify({ type: 'pong' }) + '\n');
    } else if (cmd.type === 'shutdown') {
      process.exit(0);
    } else if (cmd.channel && cmd.text) {
      await app.client.chat.postMessage({
        channel: cmd.channel,
        text: cmd.text,
      });
    }
  } catch (e) {
    process.stderr.write(`Parse error: ${e.message}\n`);
  }
});

(async () => {
  await app.start();
  process.stderr.write('Slack bridge started (socket mode)\n');
})();
