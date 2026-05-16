import { useEffect, useMemo, useRef, useState } from "react";
import { Virtuoso, VirtuosoHandle } from "react-virtuoso";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "@/shared/api/client";
import type { Message } from "@/shared/api/types";
import { OP, WsClient } from "@/shared/ws/client";
import { useAuth } from "@/features/auth/useAuth";
import { MessageRow } from "./MessageRow";
import { Composer } from "./Composer";

type Selected =
  | { kind: "channel"; id: string }
  | { kind: "dm"; id: string };

interface Props { selected: Selected; ws: WsClient; }

export function ChannelView({ selected, ws }: Props) {
  const token = useAuth(s => s.accessToken)!;
  const api = useMemo(() => apiFetch(token), [token]);
  const qc = useQueryClient();
  const ref = useRef<VirtuosoHandle | null>(null);

  const path = selected.kind === "channel"
    ? `/channels/${selected.id}/messages`
    : `/dms/${selected.id}/messages`;
  const key = ["msgs", selected.kind, selected.id] as const;

  const { data: messages = [], isLoading } = useQuery({
    queryKey: key,
    queryFn: () => api<Message[]>(path),
  });

  // Subscribe over WS to this target.
  useEffect(() => {
    if (selected.kind === "channel") {
      ws.send(OP.SUBSCRIBE, { channel_ids: [selected.id], dm_thread_ids: [] });
    } else {
      ws.send(OP.SUBSCRIBE, { channel_ids: [], dm_thread_ids: [selected.id] });
    }
    const off = ws.on(OP.MSG_CREATED, (f) => {
      const m = f.d as Message;
      const matches =
        (selected.kind === "channel" && (m.target as any).channel_id === selected.id) ||
        (selected.kind === "dm" && (m.target as any).dm_thread_id === selected.id);
      if (!matches) return;
      qc.setQueryData<Message[]>(key, (old = []) => {
        if (old.find(x => x.id === m.id)) return old;
        return [m, ...old];
      });
    });
    return () => { off(); };
  }, [selected.kind, selected.id, ws, qc, key]);

  // Auto-scroll to bottom when our own send arrives — Virtuoso handles this if
  // followOutput is true.
  const [autoFollow, setAutoFollow] = useState(true);

  // Order ascending in the UI (Virtuoso renders top→bottom).
  const ordered = useMemo(() => [...messages].reverse(), [messages]);

  return (
    <div className="h-full flex flex-col">
      <header className="px-4 py-3 border-b border-slate-200 flex items-center gap-2">
        <span className="font-medium">
          {selected.kind === "channel" ? "#channel" : "Direct message"}
        </span>
      </header>

      {isLoading ? (
        <div className="flex-1 grid place-items-center text-slate-500">Loading…</div>
      ) : (
        <Virtuoso
          ref={ref}
          data={ordered}
          followOutput={autoFollow ? "auto" : false}
          atBottomStateChange={(atBottom) => setAutoFollow(atBottom)}
          itemContent={(_, m) => <MessageRow m={m} />}
          components={{ EmptyPlaceholder: () =>
            <div className="h-full grid place-items-center text-slate-500">No messages yet — say hi.</div>
          }}
        />
      )}

      <Composer target={selected} ws={ws} />
    </div>
  );
}
