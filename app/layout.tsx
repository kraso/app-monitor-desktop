import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Navigation } from "@/components/ui/navigation";
import { MonitoringProvider } from "./monitoring-provider";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "App Monitor",
  description: "Aplicación de escritorio para monitorizar el uso de aplicaciones y analizar productividad.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}>
      <head>
        <script
          dangerouslySetInnerHTML={{
            __html: `
              (function() {
                try {
                  var t = localStorage.getItem('theme') || 'system';
                  var d = t === 'system' ? window.matchMedia('(prefers-color-scheme: dark)').matches : t === 'dark';
                  document.documentElement.classList.add(d ? 'dark' : 'light');
                } catch(e) {}
              })();
            `,
          }}
        />
      </head>
      <body className="flex min-h-screen flex-col items-center">
        <MonitoringProvider>
          <Navigation />
          <main className="flex w-full max-w-6xl flex-1 flex-col items-center gap-8 px-6 py-10">
            {children}
          </main>
        </MonitoringProvider>
      </body>
    </html>
  );
}
