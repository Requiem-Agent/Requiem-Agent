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

const DEV_MODE = import.meta.env.DEV || (typeof window !== 'undefined' && window.location.search.includes('dev=1'));

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isTelegram, setIsTelegram] = useState(true);

  const authMutation = useTelegramAuth();

  useEffect(() => {
    // Dev mode: bypass Telegram auth
    if (DEV_MODE) {
      const mockUser: User = {
        id: 'dev-user',
        telegramId: 0,
        firstName: 'Dev',
        lastName: 'User',
        username: 'dev_user',
        createdAt: new Date().toISOString(),
      };
      const mockToken = 'dev-token';
      setToken(mockToken);
      setUser(mockUser);
      setIsTelegram(true);
      setIsLoading(false);
      setAuthTokenGetter(() => mockToken);
      return;
    }

    setAuthTokenGetter(() => {
      return sessionStorage.getItem('rq_tok') || localStorage.getItem('requiem_token');
    });
    
    const oldToken = localStorage.getItem('requiem_token');
    if (oldToken) {
      sessionStorage.setItem('rq_tok', oldToken);
      localStorage.removeItem('requiem_token');
      localStorage.removeItem('requiem_user');
    }

    const initAuth = async () => {
      const storedToken = sessionStorage.getItem('rq_tok');
      const storedUser = sessionStorage.getItem('rq_user');
      
      if (storedToken && storedUser) {
        try {
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
          sessionStorage.removeItem('rq_tok');
          sessionStorage.removeItem('rq_user');
        } catch {
          setToken(storedToken);
          setUser(JSON.parse(storedUser));
          setIsLoading(false);
          return;
        }
      }

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
          sessionStorage.setItem('rq_tok', authResult.token);
          sessionStorage.setItem('rq_user', JSON.stringify(authResult.user));
        } catch (error) {
          console.error('Failed to auth with Telegram', error);
        }
      } else {
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
    sessionStorage.removeItem('rq_tok');
    sessionStorage.removeItem('rq_user');
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
