import { API_BASE } from "./config";
import type { ErrorBody } from "./types";

type Init = Omit<RequestInit, "body"> & { body?: unknown };

export class ApiError extends Error {
  constructor(public code: string, public status: number, message: string) {
    super(message);
  }
}

export function apiFetch(token: string | null) {
  return async function <T>(path: string, init: Init = {}): Promise<T> {
    const headers = new Headers(init.headers);
    headers.set("Accept", "application/json");
    if (init.body !== undefined) headers.set("Content-Type", "application/json");
    if (token) headers.set("Authorization", `Bearer ${token}`);

    const res = await fetch(`${API_BASE}${path}`, {
      ...init,
      headers,
      body: init.body === undefined ? undefined : JSON.stringify(init.body),
    });

    if (!res.ok) {
      let body: ErrorBody = { code: "Internal", message: res.statusText };
      try { body = await res.json(); } catch { /* leave default */ }
      throw new ApiError(body.code, res.status, body.message);
    }
    if (res.status === 204) return undefined as T;
    return (await res.json()) as T;
  };
}
