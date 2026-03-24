#!/usr/bin/env node
// WhatsApp bridge for Hydra — reads/writes JSON lines on stdio.
// Requires: npm install @whiskeysockets/baileys qrcode-terminal
// First run shows QR code in terminal for pairing.

const readline = require('readline');
const path = require('path');
const fs = require('fs');

let makeWASocket, useMultiFileAuthState, DisconnectReason;
try {
  const baileys = require('@whiskeysockets/baileys');
  makeWASocket = baileys.default;
  useMultiFileAuthState = baileys.useMultiFileAuthState;
  DisconnectReason = baileys.DisconnectReason;
} catch {
  process.stderr.write('ERROR: npm install @whiskeysockets/baileys qrcode-terminal\n');
  process.exit(1);
}

const SESSION_DIR = process.env.WHATSAPP_SESSION_PATH
  || path.join(process.env.HOME || '.', '.hydra/data/bridges/whatsapp/session');

async function start() {
  fs.mkdirSync(SESSION_DIR, { recursive: true });
  const { state, saveCreds } = await useMultiFileAuthState(SESSION_DIR);

  const sock = makeWASocket({
    auth: state,
    printQRInTerminal: true,
  });

  sock.ev.on('creds.update', saveCreds);

  sock.ev.on('connection.update', (update) => {
    const { connection, lastDisconnect } = update;
    if (connection === 'close') {
      const reason = lastDisconnect?.error?.output?.statusCode;
      if (reason !== DisconnectReason.loggedOut) {
        process.stderr.write('WhatsApp reconnecting...\n');
        start();
      } else {
        process.stderr.write('WhatsApp logged out. Delete session and re-pair.\n');
        process.exit(1);
      }
    } else if (connection === 'open') {
      process.stderr.write('WhatsApp connected\n');
      process.stdout.write(JSON.stringify({ type: 'ready' }) + '\n');
    }
  });

  // Incoming messages
  sock.ev.on('messages.upsert', ({ messages }) => {
    for (const msg of messages) {
      if (msg.key.fromMe) continue;
      const text = msg.message?.conversation
        || msg.message?.extendedTextMessage?.text
        || '';
      if (!text) continue;

      const event = {
        type: 'message',
        from: msg.key.remoteJid,
        text,
        timestamp: new Date(msg.messageTimestamp * 1000).toISOString(),
      };
      process.stdout.write(JSON.stringify(event) + '\n');
    }
  });

  // Outgoing: read from stdin
  const rl = readline.createInterface({ input: process.stdin });
  rl.on('line', async (line) => {
    try {
      const cmd = JSON.parse(line);
      if (cmd.type === 'init') {
        process.stdout.write(JSON.stringify({ type: 'ready' }) + '\n');
      } else if (cmd.type === 'ping') {
        process.stdout.write(JSON.stringify({ type: 'pong' }) + '\n');
      } else if (cmd.type === 'shutdown') {
        sock.end();
        process.exit(0);
      } else if (cmd.to && cmd.text) {
        await sock.sendMessage(cmd.to, { text: cmd.text });
      }
    } catch (e) {
      process.stderr.write(`Parse error: ${e.message}\n`);
    }
  });
}

start().catch((e) => {
  process.stderr.write(`WhatsApp bridge error: ${e.message}\n`);
  process.exit(1);
});
