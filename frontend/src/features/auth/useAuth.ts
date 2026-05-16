import { create } from "zustand";
import type { User } from "@/shared/api/types";

interface AuthState {
  accessToken:  string | null;
  refreshToken: string | null;
  user:         User   | null;
  setSession:   (s: { accessToken: string; refreshToken: string; user: User }) => void;
  clear:        () => void;
}

export const useAuth = create<AuthState>((set) => ({
  accessToken:  null,
  refreshToken: null,
  user:         null,
  setSession: ({ accessToken, refreshToken, user }) =>
    set({ accessToken, refreshToken, user }),
  clear: () => set({ accessToken: null, refreshToken: null, user: null }),
}));
