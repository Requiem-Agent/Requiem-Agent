import React, { createContext, useContext, useEffect, useState } from 'react';

interface AuthUser {
  id: string;
  firstName?: string;
  username?: string;
  plan?: string;
}

interface AuthContextType {
  user: AuthUser | null;
  token: string | null;
  isLoading: boolean;
  isTelegram: boolean;
  login: (username: string, password: string) => Promise<{ success: boolean; error?: string }>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType>({
  user: null,
  token: null,
  isLoading: true,
  isTelegram: true,
  login: async () => ({ success: false }),
  logout: () => {},
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const storedToken = sessionStorage.getItem('rq_tok');
    const storedUser = sessionStorage.getItem('rq_user');

    if (storedToken && storedUser) {
      setToken(storedToken);
      setUser(JSON.parse(storedUser));
    }
    setIsLoading(false);
  }, []);

  const login = async (username: string, password: string) => {
    try {
      const apiBase = import.meta.env.VITE_API_URL || '';
      const res = await fetch(`${apiBase}/api/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username, password }),
      });
      const data = await res.json();
      if (!res.ok) return { success: false, error: data.error || 'Invalid credentials' };

      const authUser: AuthUser = {
        id: data.user_id,
        username,
        plan: 'premium',
      };

      setToken(data.token);
      setUser(authUser);
      sessionStorage.setItem('rq_tok', data.token);
      sessionStorage.setItem('rq_user', JSON.stringify(authUser));

      return { success: true };
    } catch (e: any) {
      return { success: false, error: e.message || 'Connection error' };
    }
  };

  const logout = () => {
    sessionStorage.removeItem('rq_tok');
    sessionStorage.removeItem('rq_user');
    localStorage.removeItem('requiem_token');
    localStorage.removeItem('requiem_user');
    setToken(null);
    setUser(null);
  };

  return (
    <AuthContext.Provider value={{ user, token, isLoading, isTelegram: true, login, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}

/** Access check — user must be logged in */
export function useRequireAuth() {
  const { isLoading, user } = useAuth();
  return { isReady: !isLoading && !!user, isLoading };
}
