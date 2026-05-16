import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { apiFetch } from "@/shared/api/client";
import type { Channel, DmThread, Workspace } from "@/shared/api/types";
import { useAuth } from "@/features/auth/useAuth";
import { ChannelView } from "@/features/chat/ChannelView";
import { Sidebar } from "@/features/chat/Sidebar";
import { WsClient } from "@/shared/ws/client";

export function Shell() {
  const token = useAuth(s => s.accessToken)!;
  const user  = useAuth(s => s.user)!;
  const logout = useAuth(s => s.clear);
  const api = useMemo(() => apiFetch(token), [token]);

  const wsRef = useState<WsClient | null>(null);
  useEffect(() => {
    const c = new WsClient(token);
    c.connect();
    wsRef[1](c);
    return () => c.close();
  }, [token]);

  const { data: workspaces = [] } = useQuery({
    queryKey: ["workspaces"],
    queryFn: () => api<Workspace[]>("/workspaces"),
  });
  const [wsId, setWsId] = useState<string | null>(null);
  const activeWs = workspaces.find(w => w.id === wsId) ?? workspaces[0];

  const { data: channels = [] } = useQuery({
    queryKey: ["channels", activeWs?.id],
    queryFn: () => api<Channel[]>(`/workspaces/${activeWs!.id}/channels`),
    enabled: !!activeWs,
  });
  const { data: dms = [] } = useQuery({
    queryKey: ["dms"],
    queryFn: () => api<DmThread[]>("/dms"),
  });

  const [selected, setSelected] = useState<
    | { kind: "channel"; id: string }
    | { kind: "dm"; id: string }
    | null
  >(null);

  // Auto-select the first channel when one arrives.
  useEffect(() => {
    if (!selected && channels.length > 0) {
      setSelected({ kind: "channel", id: channels[0].id });
    }
  }, [channels, selected]);

  return (
    <div className="h-full grid grid-cols-[260px_1fr]">
      <Sidebar
        user={user}
        workspaces={workspaces}
        activeWorkspace={activeWs}
        onPickWorkspace={(w) => setWsId(w.id)}
        channels={channels}
        dms={dms}
        selected={selected}
        onSelect={setSelected}
        onLogout={logout}
      />
      <main className="bg-white border-l border-slate-200 min-w-0">
        {selected && wsRef[0]
          ? <ChannelView selected={selected} ws={wsRef[0]} />
          : <div className="h-full flex items-center justify-center text-slate-500">
              Pick a channel or DM to start.
            </div>}
      </main>
    </div>
  );
}
