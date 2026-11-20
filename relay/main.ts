import * as mf from "mineflayer";
import { EventEmitter } from "node:events";
import { Mutex } from "async-mutex";

import * as pc from "prismarine-chat";

const bot = mf.createBot({
  host: "hypixel.net", // i love playing with fire :333
  username: "placeholder",
  auth: "microsoft",
  version: "1.8.9",
});

function limboConfirmation(msg: pc.ChatMessage): boolean {
  return msg.json.text == "You were spawned in Limbo." &&
    msg.json.color == "red"; // FUCK YOU FUCK YOU FUCK YOU WHY IS MSG.JSON ANY
}

interface guildMessage {
  ign: string;
  uuid: string;
  role?: string;
  message: string;
}

const guildMessageHeaderRegex =
  /^§2Guild > §.(?:\[[^\]]+\] )?([a-zA-Z0-9_]{2,16}) §.\[([a-zA-Z]+)\]§.:/mu;

function parseGuildMessage(rawmsg: pc.ChatMessage): guildMessage | null {
  const header = rawmsg.extra?.[0]?.json.text;
  if (typeof header !== "string") return null;

  const match = header.match(guildMessageHeaderRegex);
  if (!match) return null;

  const [, ign, role] = match;

  const message = rawmsg.extra
    ?.[1].json.text;

  if (typeof message !== "string") {
    console.warn("hold on what the fuck? header parsing worked", rawmsg);
    return null;
  }

  const uuid = rawmsg.extra?.[0]?.json.clickEvent.value.split(
    "/viewprofile ",
  )[1];

  return { // what do you mean this is anonymous where are my precious structs
    ign: ign,
    uuid: uuid,
    role: role,
    message: message,
  };
}

bot.on("kicked", console.warn);
bot.on("error", console.error);
bot.once("spawn", () => {
  console.log("we joined");
  setTimeout(() => {
    bot.chat("/chat guild");
  }, 500);
  setTimeout(() => {
    bot.chat("/limbo");
    console.log("Limbo command sent, waiting for confirmation...");
  }, 1000);
});

let inLimbo = false;
const eventbullshit = new EventEmitter();

bot.on("messagestr", (_, __, message) => {
  // js and mc is weird jesus christ i can't wait to get back to the land of sanity

  if (limboConfirmation(message)) {
    inLimbo = true;
    console.debug("we're in limbo :3");
    setTimeout(createAndOwnServer, 0); // where is my precious tokio::spawn qwq
    return;
  }

  if (!inLimbo) {
    console.debug("dropping a message due to not being in limbo");
    return;
  }

  const gmsg = parseGuildMessage(message);
  if (gmsg != null) {
    console.log(`${gmsg.ign} [${gmsg.role ?? "NON"}]: ${gmsg.message}`);

    eventbullshit.emit("hypixelToRelayMsg", gmsg);
  }
});

interface SocketClientToServerMessage {
  message: string;
  addressing: {
    ign: string;
    uuid: string;
  };
  ephemeral: boolean;
}

async function createAndOwnServer(): Promise<void> {
  const SOCKET_PATH = "/tmp/imperium-hypixel.sock";

  try {
    Deno.removeSync(SOCKET_PATH);
  } catch { /* ignore */ }

  const listener = Deno.listen({ path: SOCKET_PATH, transport: "unix" });

  console.log(`Listening on ${SOCKET_PATH}...`);

  for await (const conn of listener) {
    handleClient(conn);
  }
}

const decoder = new TextDecoder();
const encoder = new TextEncoder();

async function handleClient(conn: Deno.Conn) {
  const SEPARATOR = "\n";

  console.log("Client connected!");

  const write_mutex = new Mutex();

  const onHypixelToRelayMsg = (gmsg: guildMessage) => {
    write_mutex.runExclusive(async () => {
      const encoded = encoder.encode(JSON.stringify(gmsg) + SEPARATOR);
      const len = encoded.length;
      let written = 0;
      while (written < len) {
        const immediately_written = await conn.write(encoded.slice(written));

        if (immediately_written === 0) {
          throw new Error("wrote 0 bytes, tf?");
        }
        written += immediately_written;
      }

      if (written > len) {
        console.warn(
          "..... is this even reachable? how the fuck did we over-write",
        );
      }
    });
  };

  eventbullshit.on("hypixelToRelayMsg", onHypixelToRelayMsg);

  const buffer = new Uint8Array(4096);

  let pending = "";

  try {
    // stuff

    while (1) {
      const n = await conn.read(buffer);

      if (n === null) break;

      pending += decoder.decode(buffer.subarray(0, n));

      let separator_pos;

      while ((separator_pos = pending.indexOf(SEPARATOR)) !== -1) {
        const msg = pending.slice(0, separator_pos);

        pending = pending.slice(separator_pos + 1);

        if (!msg.trim()) continue;

        const parsed = JSON.parse(msg);

        eventbullshit.emit(
          "imperiumToHypixelMsg",
          parsed as SocketClientToServerMessage,
        );
      }
    }
  } catch (err) {
    console.error("handling client error", err);
  } finally {
    eventbullshit.off("hypixelToRelayMsg", onHypixelToRelayMsg);
    conn.close();
    console.log("Client closed.");
  }
}

eventbullshit.on("imperiumToHypixelMsg", (msg: SocketClientToServerMessage) => {
  bot.chat(msg.message);
  console.warn("ephemeral messages are unimplemented!");
});
