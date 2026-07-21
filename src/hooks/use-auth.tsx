import React, { createContext, useContext, useEffect, useState } from 'react';
import { useTelegramAuth, User, setAuthTokenGetter } from '@workspace/api-client-react';

interface AuthContextType {
  user: User | null;
  token: string | null;
  isLoading: boolean;
  isTelegram: boolean;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType>({
  user: null,
  token: null,
  isLoading: true,
  isTelegram: true,
  logout: () => {},
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isTelegram, setIsTelegram] = useState(true);

  const authMutation = useTelegramAuth();

  useEffect(() => {
    // Register token getter for the API client
    // S1-06: استخدام sessionStorage بدلاً من localStorage للـ token
    // sessionStorage يُمسح عند إغلاق التبويب — أأمن من localStorage
    setAuthTokenGetter(() => {
      return sessionStorage.getItem('rq_tok') || localStorage.getItem('requiem_token');
    });
    
    // S1-06: حذف الـ token القديم من localStorage إذا وُجد
    const oldToken = localStorage.getItem('requiem_token');
    if (oldToken) {
      sessionStorage.setItem('rq_tok', oldToken);
      localStorage.removeItem('requiem_token');
      localStorage.removeItem('requiem_user');
    }

    const initAuth = async () => {
      // 1. Check local storage
      // S1-06: قراءة من sessionStorage أولاً (أأمن)
      const storedToken = sessionStorage.getItem('rq_tok');
      const storedUser = sessionStorage.getItem('rq_user');
      
      if (storedToken && storedUser) {
        try {
          // Validate stored token is still accepted by the API
          const apiBase = import.meta.env.VITE_API_URL || "";
          const check = await fetch(`${apiBase}/api/usage`, {
            headers: { Authorization: `Bearer ${storedToken}` },
          });
          if (check.ok || check.status !== 401) {
            setToken(storedToken);
            setUser(JSON.parse(storedUser));
            setIsLoading(false);
            return;
          }
          // 401 → token invalid, clear and re-auth below
          sessionStorage.removeItem('rq_tok');
          sessionStorage.removeItem('rq_user');
        } catch {
          // network error — trust stored token optimistically
          setToken(storedToken);
          setUser(JSON.parse(storedUser));
          setIsLoading(false);
          return;
        }
      }

      // 2. Try Telegram Auth
      const webApp = (window as any).Telegram?.WebApp;
      if (webApp) {
        webApp.ready();
        webApp.expand();
      }

      const initData = webApp?.initData;
      if (initData) {
        try {
          const authResult = await authMutation.mutateAsync({ data: { initData } });
          setToken(authResult.token);
          setUser(authResult.user);
          setIsTelegram(true);
          // S1-06: حفظ في sessionStorage (يُمسح عند إغلاق التبويب)
          sessionStorage.setItem('rq_tok', authResult.token);
          sessionStorage.setItem('rq_user', JSON.stringify(authResult.user));
        } catch (error) {
          console.error('Failed to auth with Telegram', error);
        }
      } else {
        // Non-Telegram access — show rejection message
        setIsTelegram(false);
        setIsLoading(false);
        return;
      }
      
      setIsLoading(false);
    };

    initAuth();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const logout = () => {
    // S1-06: مسح من sessionStorage
    sessionStorage.removeItem('rq_tok');
    sessionStorage.removeItem('rq_user');
    // مسح القديم من localStorage أيضاً
    localStorage.removeItem('requiem_token');
    localStorage.removeItem('requiem_user');
    setToken(null);
    setUser(null);
  };

  return (
    <AuthContext.Provider value={{ user, token, isLoading, isTelegram, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}

/** Hook for Telegram-only access check */
export function useRequireTelegram() {
  const { isLoading, isTelegram, user } = useAuth();
  return { isReady: !isLoading && isTelegram && !!user, isLoading, isTelegram };
}
