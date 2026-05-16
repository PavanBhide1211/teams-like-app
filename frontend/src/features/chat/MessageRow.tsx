import { Reply } from "lucide-react";
import type { Message } from "@/shared/api/types";

interface Props { m: Message; }

export function MessageRow({ m }: Props) {
  const when = new Date(m.created_at);
  return (
    <div className="msg-row px-4 py-2 hover:bg-slate-50 group">
      <div className="flex items-start gap-3">
        <div className="w-8 h-8 rounded-full bg-brand-500 grid place-items-center text-xs text-white font-medium shrink-0">
          {m.author_id.slice(0, 2).toUpperCase()}
        </div>
        <div className="min-w-0">
          <div className="flex items-baseline gap-2">
            <span className="font-medium text-sm">{m.author_id.slice(0, 8)}</span>
            <time className="text-xs text-slate-500">
              {when.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
            </time>
            {m.edited_at && <span className="text-xs text-slate-400">(edited)</span>}
          </div>
          <div className="text-sm whitespace-pre-wrap break-words">{m.body}</div>
          {m.parent_id && (
            <div className="text-xs text-slate-500 mt-1 inline-flex items-center gap-1">
              <Reply size={12} /> reply
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
