import { encode, decode } from "@msgpack/msgpack";
import { WS_BASE } from "../api/config";

export const OP = {
  HELLO: 0x00, HEARTBEAT: 0x01, HEARTBEAT_ACK: 0x02,
  SUBSCRIBE: 0x10, SUBSCRIBE_ADD: 0x11, SUBSCRIBE_REMOVE: 0x12,
  MSG_SEND: 0x20, TYPING: 0x25, PRESENCE_SET: 0x26,
  MSG_CREATED: 0x30, MSG_EDITED: 0x31, MSG_DELETED: 0x32,
  REACT_CREATED: 0x33, REACT_REMOVED: 0x34,
  TYPING_EVENT: 0x35, PRESENCE_EVENT: 0x36,
  ACK: 0xF0, ERR: 0xF1,
} as const;

export interface Frame {
  op:     number;
  d:      unknown;
  nonce?: number;
  seq?:   number;
}

type Handler = (frame: Frame) => void;

export class WsClient {
  private ws?: WebSocket;
  private handlers = new Map<number, Set<Handler>>();
  private backoffMs = 250;
  private heartbeat?: number;
  private nextNonce = 1;

  constructor(private token: string) {}

  on(op: number, h: Handler) {
    let set = this.handlers.get(op);
    if (!set) { set = new Set(); this.handlers.set(op, set); }
    set.add(h);
    return () => set!.delete(h);
  }

  connect() {
    const url = `${WS_BASE}/ws?token=${encodeURIComponent(this.token)}`;
    const ws = new WebSocket(url);
    ws.binaryType = "arraybuffer";
    ws.onopen = () => {
      this.backoffMs = 250;
      this.heartbeat = window.setInterval(
        () => this.send(OP.HEARTBEAT, {}),
        15_000,
      );
    };
    ws.onmessage = (ev) => {
      try {
        const frame = decode(new Uint8Array(ev.data as ArrayBuffer)) as Frame;
        this.handlers.get(frame.op)?.forEach((h) => h(frame));
      } catch { /* drop bad frame */ }
    };
    ws.onclose = () => {
      if (this.heartbeat) window.clearInterval(this.heartbeat);
      this.heartbeat = undefined;
      setTimeout(() => this.connect(), this.backoffMs);
      this.backoffMs = Math.min(this.backoffMs * 2, 60_000);
    };
    ws.onerror = () => ws.close();
    this.ws = ws;
  }

  send(op: number, d: unknown, opts: { nonce?: number } = {}): number | undefined {
    if (!this.ws || this.ws.readyState !== 1) return;
    const nonce = opts.nonce ?? this.nextNonce++;
    this.ws.send(encode({ op, d, nonce }));
    return nonce;
  }

  close() {
    this.ws?.close();
  }
}
