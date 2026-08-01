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
  login: (username: string, password: string) => Promise<{ success: boolean; error?: string }>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType>({
  user: null,
  token: null,
  isLoading: true,
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

    if (!storedToken) {
      setIsLoading(false);
      return;
    }

    const apiBase = import.meta.env.VITE_API_URL || '';

    fetch(`${apiBase}/api/auth/me`, {
      headers: { Authorization: `Bearer ${storedToken}` },
    })
      .then((res) => {
        if (res.status === 401) {
          sessionStorage.removeItem('rq_tok');
          sessionStorage.removeItem('rq_user');
          setToken(null);
          setUser(null);
          setIsLoading(false);
          return;
        }
        return res.json().then((data) => {
          setToken(storedToken);
          setUser({
            id: data.user_id,
            username: data.username || safeUserField(storedUser, 'username'),
            plan: data.plan || 'free',
          });
          setIsLoading(false);
        });
      })
      .catch(() => {
        // فشل الشبكة — نُبقي الجلسة المخزنة محلياً
        if (storedUser) {
          try { setUser(JSON.parse(storedUser)); } catch { setUser(null); }
        }
        setToken(storedToken);
        setIsLoading(false);
      });
  }, []);

/** Safe parse of a stored user JSON field (never throws JSON.parse errors). */
const safeUserField = (raw: string | null, field: 'username' | 'plan'): string | undefined => {
  if (!raw) return undefined;
  try { return JSON.parse(raw)?.[field] as string | undefined; } catch { return undefined; }
};

  const login = async (username: string, password: string) => {
    const apiBase = import.meta.env.VITE_API_URL || '';
    // Small helper: parse JSON safely — a proxy/replica may return an empty
    // body or non-JSON payload; never crash with "JSON.parse:" errors.
    const tryJson = async (res: Response) => {
      const text = await res.text();
      if (!text) return {};
      try { return JSON.parse(text); } catch { return { raw: text }; }
    };

    // One retry on empty/opaque responses (cold replica wake-up etc.)
    for (let attempt = 0; attempt < 2; attempt++) {
      try {
        const res = await fetch(`${apiBase}/api/auth/login`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ username, password }),
        });
        const data = await tryJson(res);
        if (res.ok) {
          if (!data.token) {
            if (attempt === 0) { await new Promise(r => setTimeout(r, 1200)); continue; }
            return { success: false, error: 'استجابة فارغة من الخادم — حاول مجدداً' };
          }
          const authUser: AuthUser = {
            id: data.user_id,
            username: data.username || username,
            plan: data.plan || 'free',
          };
          setToken(data.token);
          setUser(authUser);
          sessionStorage.setItem('rq_tok', data.token);
          sessionStorage.setItem('rq_user', JSON.stringify(authUser));
          return { success: true };
        }
        return { success: false, error: data.error || data.raw || `فشل تسجيل الدخول (${res.status})` };
      } catch (e: any) {
        if (attempt === 0) { await new Promise(r => setTimeout(r, 1200)); continue; }
        return { success: false, error: e?.message?.includes('JSON.parse') ? 'استجابة غير صالحة من الخادم — حاول مجدداً' : (e?.message || 'Connection error') };
      }
    }
    return { success: false, error: 'Connection error' };
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
    <AuthContext.Provider value={{ user, token, isLoading, login, logout }}>
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
