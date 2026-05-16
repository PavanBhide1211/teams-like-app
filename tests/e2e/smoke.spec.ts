import { test, expect } from "@playwright/test";
import { encode, decode } from "@msgpack/msgpack";
import WebSocket from "ws";

const API = process.env.API_BASE ?? "http://localhost:8000";
const WS  = process.env.WS_BASE  ?? "ws://localhost:8000";

async function api(path: string, init: RequestInit & { token?: string } = {}) {
  const h = new Headers(init.headers);
  h.set("Content-Type", "application/json");
  if (init.token) h.set("Authorization", `Bearer ${init.token}`);
  const r = await fetch(API + path, { ...init, headers: h });
  if (!r.ok) throw new Error(`${path} -> ${r.status} ${await r.text()}`);
  if (r.status === 204) return undefined;
  return r.json();
}

test("happy path: register → workspace → channel → WS send/receive", async () => {
  const stamp = Date.now();
  const email = `smoke-${stamp}@example.eu`;

  // 1. Register
  const auth = await api("/auth/register", {
    method: "POST",
    body: JSON.stringify({ email, display_name: "Smoke Tester", password: "very-long-password-1234" }),
  });
  expect(auth.access_token).toBeTruthy();
  const token = auth.access_token;

  // 2. Create workspace
  const ws_ = await api("/workspaces", {
    method: "POST", token,
    body: JSON.stringify({ name: "Smoke Org", slug: `smoke-${stamp}` }),
  });
  expect(ws_.id).toBeTruthy();

  // 3. Create a channel
  const ch = await api(`/workspaces/${ws_.id}/channels`, {
    method: "POST", token,
    body: JSON.stringify({ name: "general", topic: "smoke", kind: "public" }),
  });
  expect(ch.id).toBeTruthy();

  // 4. Open WS
  const sock = new WebSocket(`${WS}/ws?token=${encodeURIComponent(token)}`);
  await new Promise<void>((res, rej) => {
    sock.once("open", () => res());
    sock.once("error", rej);
  });

  // 5. Wait for HELLO frame
  const hello = await new Promise<any>((res) => {
    sock.once("message", (data) => res(decode(new Uint8Array(data as Buffer))));
  });
  expect((hello as any).op).toBe(0x00);

  // 6. Subscribe to channel
  sock.send(encode({ op: 0x10, d: { channel_ids: [ch.id], dm_thread_ids: [] }, nonce: 1 }));

  // Wait for ACK
  await new Promise<any>((res) => sock.once("message", (data) => res(decode(new Uint8Array(data as Buffer)))));

  // 7. Send a message over WS, expect ACK + MSG_CREATED echo
  sock.send(encode({ op: 0x20, d: { channel_id: ch.id, body: "hello from smoke!" }, nonce: 2 }));

  let gotAck = false;
  let gotEvent = false;
  await new Promise<void>((res) => {
    sock.on("message", (data) => {
      const frame = decode(new Uint8Array(data as Buffer)) as any;
      if (frame.op === 0xF0 && frame.nonce === 2) gotAck = true;
      if (frame.op === 0x30 && frame.d?.body === "hello from smoke!") gotEvent = true;
      if (gotAck && gotEvent) res();
    });
  });

  expect(gotAck).toBe(true);
  expect(gotEvent).toBe(true);

  sock.close();
});
