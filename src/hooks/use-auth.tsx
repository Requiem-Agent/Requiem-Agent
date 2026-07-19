import React, { createContext, useContext, useEffect, useState } from 'react';
import { useTelegramAuth, User, setAuthTokenGetter } from '@workspace/api-client-react';

interface AuthContextType {
  user: User | null;
  token: string | null;
  isLoading: boolean;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType>({
  user: null,
  token: null,
  isLoading: true,
  logout: () => {},
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

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
          const check = await fetch('/api/usage', {
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
      const webApp = window.Telegram?.WebApp;
      if (webApp) {
        webApp.ready();
        webApp.expand();
        
        // Optional: override theme variables based on Telegram theme
        // For Requiem we prefer our own dark theme, but we map if required
      }

      const initData = webApp?.initData;
      if (initData) {
        try {
          const authResult = await authMutation.mutateAsync({ data: { initData } });
          setToken(authResult.token);
          setUser(authResult.user);
          localStorage.setItem('requiem_token', authResult.token);
          localStorage.setItem('requiem_user', JSON.stringify(authResult.user));
        } catch (error) {
          console.error('Failed to auth with Telegram', error);
        }
      } else {
        // Browser mode — generate unique ID per browser, auth with backend
        let localId = localStorage.getItem('requiem_local_id');
        if (!localId) {
          localId = 'local-' + Math.random().toString(36).substring(2, 10);
          localStorage.setItem('requiem_local_id', localId);
        }
        try {
          const authResult = await authMutation.mutateAsync({ data: { initData: localId } });
          setToken(authResult.token);
          setUser(authResult.user);
          localStorage.setItem('requiem_token', authResult.token);
          localStorage.setItem('requiem_user', JSON.stringify(authResult.user));
        } catch {
          // Fallback: use local ID
          const mockUser: User = {
            id: localId, telegramId: 0, firstName: 'User-' + localId.slice(-4),
            createdAt: new Date().toISOString(), quotaReadUsed: 0, quotaWriteUsed: 0,
          };
          setToken(localId);
          setUser(mockUser);
        }
      }
      
      setIsLoading(false);
    };

    initAuth();
  }, []);

  const logout = () => {
    localStorage.removeItem('requiem_token');
    localStorage.removeItem('requiem_user');
    setToken(null);
    setUser(null);
  };

  return (
    <AuthContext.Provider value={{ user, token, isLoading, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  return useContext(AuthContext);
}
