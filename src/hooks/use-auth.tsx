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
    setAuthTokenGetter(() => {
      return localStorage.getItem('requiem_token');
    });

    const initAuth = async () => {
      // 1. Check local storage
      const storedToken = localStorage.getItem('requiem_token');
      const storedUser = localStorage.getItem('requiem_user');
      
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
          localStorage.removeItem('requiem_token');
          localStorage.removeItem('requiem_user');
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
          localStorage.setItem('requiem_token', authResult.token);
          localStorage.setItem('requiem_user', JSON.stringify(authResult.user));
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
