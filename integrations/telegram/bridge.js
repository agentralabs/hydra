#!/usr/bin/env node
// Telegram bridge for Hydra — reads/writes JSON lines on stdio.
// Requires: npm install node-telegram-bot-api
// Env: TELEGRAM_BOT_TOKEN

const readline = require('readline');

const TOKEN = process.env.TELEGRAM_BOT_TOKEN;
if (!TOKEN) {
  process.stderr.write('ERROR: TELEGRAM_BOT_TOKEN not set\n');
  process.exit(1);
}

let TelegramBot;
try {
  TelegramBot = require('node-telegram-bot-api');
} catch {
  process.stderr.write('ERROR: npm install node-telegram-bot-api\n');
  process.exit(1);
}

const bot = new TelegramBot(TOKEN, { polling: true });

// Incoming: Telegram message → JSON line to stdout
bot.on('message', (msg) => {
  const event = {
    type: 'message',
    from: msg.from?.username || msg.from?.first_name || String(msg.from?.id),
    chat_id: String(msg.chat.id),
    text: msg.text || '',
    timestamp: new Date(msg.date * 1000).toISOString(),
  };
  process.stdout.write(JSON.stringify(event) + '\n');
});

// Outgoing: JSON line from stdin → Telegram reply
const rl = readline.createInterface({ input: process.stdin });
rl.on('line', (line) => {
  try {
    const cmd = JSON.parse(line);
    if (cmd.type === 'init') {
      process.stdout.write(JSON.stringify({ type: 'ready' }) + '\n');
    } else if (cmd.type === 'ping') {
      process.stdout.write(JSON.stringify({ type: 'pong' }) + '\n');
    } else if (cmd.type === 'shutdown') {
      bot.stopPolling();
      process.exit(0);
    } else if (cmd.chat_id && cmd.text) {
      bot.sendMessage(cmd.chat_id, cmd.text);
    }
  } catch (e) {
    process.stderr.write(`Parse error: ${e.message}\n`);
  }
});

process.stderr.write('Telegram bridge started\n');
