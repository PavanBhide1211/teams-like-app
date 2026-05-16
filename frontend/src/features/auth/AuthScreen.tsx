import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { apiFetch } from "@/shared/api/client";
import type { AuthResponse } from "@/shared/api/types";
import { useAuth } from "./useAuth";

export function AuthScreen() {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const setSession = useAuth(s => s.setSession);
  const nav = useNavigate();

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true); setErr(null);
    try {
      const api = apiFetch(null);
      const path = mode === "login" ? "/auth/login" : "/auth/register";
      const body = mode === "login"
        ? { email, password }
        : { email, display_name: displayName, password };
      const r = await api<AuthResponse>(path, { method: "POST", body });
      setSession({
        accessToken: r.access_token,
        refreshToken: r.refresh_token,
        user: r.user,
      });
      nav("/", { replace: true });
    } catch (e: any) {
      setErr(e?.message ?? "request failed");
    } finally { setBusy(false); }
  }

  return (
    <div className="h-full flex items-center justify-center bg-gradient-to-br from-brand-50 to-slate-100">
      <form onSubmit={submit} className="w-full max-w-sm bg-white rounded-2xl shadow-md p-8 space-y-4 border border-slate-200">
        <h1 className="text-xl font-semibold">
          {mode === "login" ? "Sign in to Cowork" : "Create a Cowork account"}
        </h1>

        {mode === "register" && (
          <label className="block">
            <span className="text-sm text-slate-600">Display name</span>
            <input
              required minLength={2}
              className="mt-1 w-full px-3 py-2 rounded-lg border border-slate-300 focus:border-brand-500 focus:ring-1 focus:ring-brand-500 outline-none"
              value={displayName} onChange={e => setDisplayName(e.target.value)} />
          </label>
        )}

        <label className="block">
          <span className="text-sm text-slate-600">Email</span>
          <input
            type="email" required
            className="mt-1 w-full px-3 py-2 rounded-lg border border-slate-300 focus:border-brand-500 focus:ring-1 focus:ring-brand-500 outline-none"
            value={email} onChange={e => setEmail(e.target.value)} />
        </label>

        <label className="block">
          <span className="text-sm text-slate-600">Password</span>
          <input
            type="password" required minLength={12}
            className="mt-1 w-full px-3 py-2 rounded-lg border border-slate-300 focus:border-brand-500 focus:ring-1 focus:ring-brand-500 outline-none"
            value={password} onChange={e => setPassword(e.target.value)} />
          {mode === "register" && (
            <span className="text-xs text-slate-500">12+ characters</span>
          )}
        </label>

        {err && <div className="text-sm text-red-600 bg-red-50 border border-red-200 rounded px-3 py-2">{err}</div>}

        <button
          type="submit" disabled={busy}
          className="w-full py-2 rounded-lg bg-brand-600 hover:bg-brand-700 text-white font-medium disabled:opacity-60">
          {busy ? "..." : (mode === "login" ? "Sign in" : "Create account")}
        </button>

        <div className="text-sm text-center text-slate-600">
          {mode === "login" ? (
            <>No account? <button type="button" onClick={() => setMode("register")} className="text-brand-600 hover:underline">Create one</button></>
          ) : (
            <>Already a member? <button type="button" onClick={() => setMode("login")} className="text-brand-600 hover:underline">Sign in</button></>
          )}
        </div>
      </form>
    </div>
  );
}
