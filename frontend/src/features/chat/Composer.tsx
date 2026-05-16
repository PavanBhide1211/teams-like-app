import { useState, useRef, useEffect } from "react";
import { Send } from "lucide-react";
import { OP, WsClient } from "@/shared/ws/client";

type Target =
  | { kind: "channel"; id: string }
  | { kind: "dm"; id: string };

interface Props { target: Target; ws: WsClient; }

export function Composer({ target, ws }: Props) {
  const [body, setBody] = useState("");
  const ref = useRef<HTMLTextAreaElement>(null);
  const lastTyping = useRef(0);

  useEffect(() => { ref.current?.focus(); }, [target.id]);

  function send() {
    const trimmed = body.trim();
    if (!trimmed) return;
    const payload: any = { body: trimmed };
    if (target.kind === "channel") payload.channel_id   = target.id;
    else                           payload.dm_thread_id = target.id;
    ws.send(OP.MSG_SEND, payload);
    setBody("");
  }

  function maybePingTyping() {
    const now = Date.now();
    if (now - lastTyping.current < 3000) return;
    lastTyping.current = now;
    const payload: any = {};
    if (target.kind === "channel") payload.channel_id   = target.id;
    else                           payload.dm_thread_id = target.id;
    ws.send(OP.TYPING, payload);
  }

  return (
    <div className="p-3 border-t border-slate-200 bg-slate-50">
      <div className="flex items-end gap-2">
        <textarea
          ref={ref}
          rows={1}
          className="flex-1 resize-none px-3 py-2 border border-slate-300 rounded-lg focus:outline-none focus:ring-1 focus:ring-brand-500 max-h-40"
          value={body}
          placeholder="Type a message…"
          onChange={(e) => { setBody(e.target.value); maybePingTyping(); }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); }
          }} />
        <button
          onClick={send}
          disabled={!body.trim()}
          className="px-3 py-2 bg-brand-600 hover:bg-brand-700 text-white rounded-lg disabled:opacity-50">
          <Send size={16} />
        </button>
      </div>
    </div>
  );
}
