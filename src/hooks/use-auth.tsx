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
  user: null, token: null, isLoading: true, isTelegram: false,
  logout: () => {},
});

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isTelegram, setIsTelegram] = useState(false);

  const authMutation = useTelegramAuth();

  useEffect(() => {
    setAuthTokenGetter(() => localStorage.getItem('requiem_token'));

    const initAuth = async () => {
      // Check for stored token first
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

      // Require Telegram WebView
      const webApp = window.Telegram?.WebApp;
      if (!webApp) {
        setIsTelegram(false);
        setIsLoading(false);
        return;
      }
      // Telegram WebView detected
      setIsTelegram(true);
      webApp.ready();
      webApp.expand();
      
      const initData = webApp.initData || '';
      if (!initData) {
        setIsLoading(false);
        return;
      }
}
