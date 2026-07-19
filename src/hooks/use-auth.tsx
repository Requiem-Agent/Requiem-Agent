import React, { createContext, useContext, useEffect, useState } from 'react';
import { useTelegramAuth, User, setAuthTokenGetter } from '@workspace/api-client-react';
import type { InitData } from '@tma.js/sdk';
import { retrieveRawInitData } from '@tma.js/sdk-react';

interface AuthContextType {
  user: User | null;
  token: string | null;
  isLoading: boolean;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType>({
  user: null, token: null, isLoading: true,
  logout: () => {},
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const authMutation = useTelegramAuth();

  useEffect(() => {
    setAuthTokenGetter(() => localStorage.getItem('requiem_token'));

    const initAuth = async () => {
      // Check stored token first
      const storedToken = localStorage.getItem('requiem_token');
      const storedUser = localStorage.getItem('requiem_user');
      
      if (storedToken && storedUser) {
        try {
          const apiBase = import.meta.env.VITE_API_URL || '';
          const check = await fetch(apiBase + '/api/usage', {
            headers: { Authorization: `Bearer ${storedToken}` },
          });
          if (check.ok || check.status !== 401) {
            setToken(storedToken);
            setUser(JSON.parse(storedUser));
            setIsLoading(false);
            return;
          }
          localStorage.removeItem('requiem_token');
          localStorage.removeItem('requiem_user');
        } catch {
          setToken(storedToken);
          setUser(JSON.parse(storedUser));
          setIsLoading(false);
          return;
        }
      }

      // Get initData from Telegram SDK
      const initDataRaw = retrieveRawInitData();
      
      if (initDataRaw) {
        try {
          const authResult = await authMutation.mutateAsync({ data: { initData: initDataRaw } });
          setToken(authResult.token);
          setUser(authResult.user);
          localStorage.setItem('requiem_token', authResult.token);
          localStorage.setItem('requiem_user', JSON.stringify(authResult.user));
        } catch (error) {
          console.error('Auth failed:', error);
        }
      }
      
      // Create local user if initData is empty (keyboard launch)
      if (!user || !token) {
        let localId = localStorage.getItem('requiem_local_id');
        if (!localId) {
          localId = 'tg-' + Math.random().toString(36).substring(2, 10);
          localStorage.setItem('requiem_local_id', localId);
        }
        const localUser: User = {
          id: localId, telegramId: 0, firstName: 'Telegram User',
          createdAt: new Date().toISOString(), quotaReadUsed: 0, quotaWriteUsed: 0,
        };
        setToken(localId);
        setUser(localUser);
        localStorage.setItem('requiem_token', localId);
        localStorage.setItem('requiem_user', JSON.stringify(localUser));
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

export function useAuth() { return useContext(AuthContext); }
