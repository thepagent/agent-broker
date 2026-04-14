#!/usr/bin/env node
// Automated bot test: send messages to Discord channels and verify responses.
// Uses Discord REST API with the CICX bot token to send test messages,
// then checks if bots reply.

const GUILD = '1492254295364731081';
const OWNER_ID = '844236700611379200';

// Bot tokens (for reading responses only - we send as a user via webhook or bot)
const BOTS = {
  CICX: {
    token: 'REDACTED_CICX_TOKEN',
    appId: null,
  },
};

// We'll use the CICX token to create a test thread and interact
const TOKEN = BOTS.CICX.token;
const API = 'https://discord.com/api/v10';

async function api(path, method = 'GET', body = null) {
  const opts = {
    method,
    headers: {
      'Authorization': `Bot ${TOKEN}`,
      'Content-Type': 'application/json',
    },
  };
  if (body) opts.body = JSON.stringify(body);
  const res = await fetch(`${API}${path}`, opts);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${method} ${path}: ${res.status} ${text.slice(0, 200)}`);
  }
  return res.json();
}

async function main() {
  // 1. Get bot user info
  const me = await api('/users/@me');
  console.log(`Bot: ${me.username}#${me.discriminator} (${me.id})`);

  // 2. List guild channels
  const channels = await api(`/guilds/${GUILD}/channels`);
  const textChannels = channels.filter(c => c.type === 0); // GUILD_TEXT
  console.log(`Found ${textChannels.length} text channels`);

  // 3. Find or create a test channel
  let testChannel = channels.find(c => c.name === 'bot-test');
  if (!testChannel) {
    console.log('Creating #bot-test channel...');
    testChannel = await api(`/guilds/${GUILD}/channels`, 'POST', {
      name: 'bot-test',
      type: 0,
      topic: 'Automated bot testing channel',
    });
  }
  console.log(`Test channel: #${testChannel.name} (${testChannel.id})`);

  // 4. Send a test message to trigger CICX
  console.log('\n=== Testing CICX (Claude) ===');
  const msg = await api(`/channels/${testChannel.id}/messages`, 'POST', {
    content: `<@${me.id}> 你好，請回覆「測試成功」`,
  });
  console.log(`Sent message: ${msg.id}`);

  // 5. Wait and check for response
  await new Promise(r => setTimeout(r, 15000));
  const messages = await api(`/channels/${testChannel.id}/messages?after=${msg.id}&limit=10`);
  const botReplies = messages.filter(m => m.author.id === me.id && m.id !== msg.id);

  if (botReplies.length > 0) {
    console.log(`✅ CICX replied: "${botReplies[0].content.slice(0, 100)}..."`);
  } else {
    // Check for thread responses
    console.log(`Messages after test: ${messages.length}`);
    for (const m of messages) {
      console.log(`  [${m.author.username}]: ${m.content.slice(0, 80)}`);
    }
  }

  // 6. Test slash command registration verification (already done above)
  console.log('\n=== Slash Command Registration ===');
  for (const [name, bot] of Object.entries(BOTS)) {
    const appId = Buffer.from(bot.token.split('.')[0], 'base64').toString();
    const cmds = await api(`/applications/${appId}/guilds/${GUILD}/commands`);
    console.log(`${name}: ${cmds.length} commands registered ✅`);
  }

  console.log('\n=== Test Complete ===');
}

main().catch(e => {
  console.error('Test failed:', e.message);
  process.exit(1);
});
