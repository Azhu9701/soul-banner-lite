"use client";

import { useState, useEffect, createContext, useContext } from "react";

const API_BASE = import.meta.env.VITE_API_URL || "/api/v1";

interface AuthState {
  authenticated: boolean;
  username: string | null;
  loading: boolean;
}

const AuthContext = createContext<AuthState>({
  authenticated: false,
  username: null,
  loading: true,
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [auth, setAuth] = useState<AuthState>({
    authenticated: false,
    username: null,
    loading: true,
  });

  useEffect(() => {
    fetch(`${API_BASE}/auth/me`, { credentials: 'include' })
      .then((res) => res.json())
      .then((data) => {
        setAuth({
          authenticated: data.authenticated || false,
          username: data.username || null,
          loading: false,
        });
      })
      .catch(() => {
        setAuth({ authenticated: false, username: null, loading: false });
      });
  }, []);

  return <AuthContext.Provider value={auth}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  return useContext(AuthContext);
}
