// Mirrors backend `proto` crate DTOs. Keep in lock-step.

export type Uuid = string;

export type ChannelKind = "public" | "private";
export type Role        = "owner" | "admin" | "member";
export type PresenceStatus = "online" | "away" | "dnd" | "offline";

export interface User {
  id: Uuid; email: string; display_name: string; avatar_url?: string | null;
}

export interface Workspace { id: Uuid; name: string; slug: string; }

export interface Channel  {
  id: Uuid; workspace_id: Uuid; name: string; topic: string; kind: ChannelKind;
}

export interface DmThread { id: Uuid; members_hash: string; }

export type MessageTarget =
  | { kind: "channel"; channel_id: Uuid }
  | { kind: "dm"; dm_thread_id: Uuid };

export interface Message {
  id: Uuid;
  target: MessageTarget;
  parent_id?: Uuid | null;
  author_id: Uuid;
  body: string;
  mentions: Uuid[];
  edited_at?: string | null;
  created_at: string;
  deleted_at?: string | null;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  access_expires_at: string;
  refresh_expires_at: string;
  user: User;
}

export interface ErrorBody { code: string; message: string; }
