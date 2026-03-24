#!/usr/bin/env node
// Discord bridge for Hydra — reads/writes JSON lines on stdio.
// Requires: npm install discord.js
// Env: DISCORD_BOT_TOKEN

const readline = require('readline');

const TOKEN = process.env.DISCORD_BOT_TOKEN;
if (!TOKEN) {
  process.stderr.write('ERROR: DISCORD_BOT_TOKEN not set\n');
  process.exit(1);
}

let Discord;
try {
  Discord = require('discord.js');
} catch {
  process.stderr.write('ERROR: npm install discord.js\n');
  process.exit(1);
}

const client = new Discord.Client({
  intents: [
    Discord.GatewayIntentBits.Guilds,
    Discord.GatewayIntentBits.GuildMessages,
    Discord.GatewayIntentBits.MessageContent,
    Discord.GatewayIntentBits.DirectMessages,
  ],
});

client.on('messageCreate', (msg) => {
  if (msg.author.bot) return;
  const event = {
    type: 'message',
    from: msg.author.username,
    channel_id: msg.channel.id,
    text: msg.content,
    timestamp: msg.createdAt.toISOString(),
  };
  process.stdout.write(JSON.stringify(event) + '\n');
});

const rl = readline.createInterface({ input: process.stdin });
rl.on('line', (line) => {
  try {
    const cmd = JSON.parse(line);
    if (cmd.type === 'init') {
      process.stdout.write(JSON.stringify({ type: 'ready' }) + '\n');
    } else if (cmd.type === 'ping') {
      process.stdout.write(JSON.stringify({ type: 'pong' }) + '\n');
    } else if (cmd.type === 'shutdown') {
      client.destroy();
      process.exit(0);
    } else if (cmd.channel_id && cmd.text) {
      const ch = client.channels.cache.get(cmd.channel_id);
      if (ch) ch.send(cmd.text);
    }
  } catch (e) {
    process.stderr.write(`Parse error: ${e.message}\n`);
  }
});

client.login(TOKEN);
process.stderr.write('Discord bridge started\n');
