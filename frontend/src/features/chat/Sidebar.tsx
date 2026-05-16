import { Hash, Lock, MessageSquare, LogOut } from "lucide-react";
import type { Channel, DmThread, User, Workspace } from "@/shared/api/types";

type Selected =
  | { kind: "channel"; id: string }
  | { kind: "dm"; id: string }
  | null;

interface Props {
  user: User;
  workspaces: Workspace[];
  activeWorkspace?: Workspace;
  channels: Channel[];
  dms: DmThread[];
  selected: Selected;
  onPickWorkspace: (w: Workspace) => void;
  onSelect: (s: Selected) => void;
  onLogout: () => void;
}

export function Sidebar(props: Props) {
  const { user, workspaces, activeWorkspace, channels, dms, selected, onPickWorkspace, onSelect, onLogout } = props;
  return (
    <aside className="flex flex-col bg-slate-900 text-slate-100">
      {/* Workspace switcher */}
      <div className="p-3 border-b border-slate-800">
        <select
          className="w-full bg-slate-800 border border-slate-700 rounded px-2 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-brand-500"
          value={activeWorkspace?.id ?? ""}
          onChange={(e) => {
            const w = workspaces.find(x => x.id === e.target.value);
            if (w) onPickWorkspace(w);
          }}>
          {workspaces.length === 0 && <option value="">(no workspaces yet)</option>}
          {workspaces.map(w => (<option key={w.id} value={w.id}>{w.name}</option>))}
        </select>
      </div>

      {/* Channels */}
      <div className="flex-1 overflow-y-auto px-2 py-3 space-y-4 text-sm">
        <section>
          <div className="px-2 text-xs uppercase tracking-wide text-slate-400">Channels</div>
          <ul>
            {channels.map(c => {
              const active = selected?.kind === "channel" && selected.id === c.id;
              return (
                <li key={c.id}>
                  <button
                    onClick={() => onSelect({ kind: "channel", id: c.id })}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded ${active ? "bg-brand-600 text-white" : "hover:bg-slate-800"}`}>
                    {c.kind === "public" ? <Hash size={14} /> : <Lock size={14} />}
                    <span className="truncate">{c.name}</span>
                  </button>
                </li>
              );
            })}
            {channels.length === 0 && <li className="px-2 py-1 text-slate-500">no channels</li>}
          </ul>
        </section>

        <section>
          <div className="px-2 text-xs uppercase tracking-wide text-slate-400">Direct messages</div>
          <ul>
            {dms.map(d => {
              const active = selected?.kind === "dm" && selected.id === d.id;
              return (
                <li key={d.id}>
                  <button
                    onClick={() => onSelect({ kind: "dm", id: d.id })}
                    className={`w-full flex items-center gap-2 px-2 py-1.5 rounded ${active ? "bg-brand-600 text-white" : "hover:bg-slate-800"}`}>
                    <MessageSquare size={14} />
                    <span className="truncate">{d.members_hash.slice(0, 12)}…</span>
                  </button>
                </li>
              );
            })}
            {dms.length === 0 && <li className="px-2 py-1 text-slate-500">no DMs</li>}
          </ul>
        </section>
      </div>

      {/* User footer */}
      <div className="px-3 py-3 border-t border-slate-800 flex items-center gap-2">
        <div className="w-7 h-7 rounded-full bg-brand-600 grid place-items-center text-xs font-medium">
          {user.display_name.split(" ").slice(0, 2).map(x => x[0]).join("").toUpperCase()}
        </div>
        <div className="flex-1 min-w-0">
          <div className="text-sm truncate">{user.display_name}</div>
          <div className="text-xs text-slate-400 truncate">{user.email}</div>
        </div>
        <button title="Log out" onClick={onLogout} className="text-slate-400 hover:text-slate-100">
          <LogOut size={16} />
        </button>
      </div>
    </aside>
  );
}
