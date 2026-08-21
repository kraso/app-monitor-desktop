"use client";

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { Card, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export default function Settings() {
  const router = useRouter();
  const [theme, setTheme] = useLocalStorage("theme", "system");
  const [notifications, setNotifications] = useLocalStorageBoolean("notifications", true);

  useEffect(() => {
    if (typeof window === "undefined") return;
    const root = window.document.documentElement;
    const systemDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const effective = theme === "system" ? (systemDark ? "dark" : "light") : theme;
    root.classList.remove("dark", "light");
    root.classList.add(effective);
  }, [theme]);

  const handlePrivacy = useCallback(() => {
    router.push("/privacy");
  }, [router]);

  return (
    <div className="flex flex-1 flex-col px-6 py-12">
      <header className="text-center">
        <h1 className="text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
          Ajustes
        </h1>
        <p className="mt-2 text-slate-500 dark:text-slate-400">
          Configuración y privacidad
        </p>
      </header>

      <div className="mx-auto mt-8 w-full max-w-2xl space-y-6 animate-fade-in">
        <Card>
          <CardHeader title="Apariencia" description="Personaliza el tema de la aplicación" />
          <div className="mt-4 flex items-center gap-3">
            <Button
              variant={theme === "light" ? "primary" : "secondary"}
              onClick={() => setTheme("light")}
              className="flex-1"
            >
              Claro
            </Button>
            <Button
              variant={theme === "dark" ? "primary" : "secondary"}
              onClick={() => setTheme("dark")}
              className="flex-1"
            >
              Oscuro
            </Button>
            <Button
              variant={theme === "system" ? "primary" : "secondary"}
              onClick={() => setTheme("system")}
              className="flex-1"
            >
              Sistema
            </Button>
          </div>
        </Card>

        <Card>
          <CardHeader title="Notificaciones" description="Configura las alertas de actividad" />
          <div className="mt-4 flex items-center gap-3">
            <Button
              variant={notifications ? "primary" : "secondary"}
              onClick={() => setNotifications(true)}
              className="flex-1"
            >
              Activar
            </Button>
            <Button
              variant={!notifications ? "danger" : "secondary"}
              onClick={() => setNotifications(false)}
              className="flex-1"
            >
              Desactivar
            </Button>
          </div>
          <p className="mt-3 text-sm text-slate-500 dark:text-slate-400">
            {notifications ? "Las notificaciones están activadas." : "Las notificaciones están desactivadas."}
          </p>
        </Card>

        <Card>
          <CardHeader title="Privacidad" description="Datos y diagnóstico" />
          <Button variant="secondary" onClick={handlePrivacy} className="w-full justify-center mt-4">
            Ver política de privacidad
          </Button>
        </Card>
      </div>
    </div>
  );
}

function useLocalStorage(key: string, fallback: string): [string, (val: string) => void] {
  const [value, setValue] = useState<string>(() => {
    if (typeof window === "undefined") return fallback;
    return window.localStorage.getItem(key) ?? fallback;
  });

  useEffect(() => {
    window.localStorage.setItem(key, value);
  }, [key, value]);

  return [value, setValue];
}

function useLocalStorageBoolean(key: string, fallback: boolean): [boolean, (val: boolean) => void] {
  const [value, setValue] = useState<boolean>(() => {
    if (typeof window === "undefined") return fallback;
    const stored = window.localStorage.getItem(key);
    if (stored === "true") return true;
    if (stored === "false") return false;
    return fallback;
  });

  useEffect(() => {
    window.localStorage.setItem(key, value ? "true" : "false");
  }, [key, value]);

  return [value, setValue];
}
