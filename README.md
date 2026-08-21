# App Monitor

Aplicación de escritorio para monitorizar cuánto tiempo se utiliza cada
aplicación del ordenador, mostrando estadísticas, historial, categorías,
objetivos y análisis de productividad.

- **UI / Dashboard:** Next.js 16 + React 19 + TypeScript (App Router).
- **Agente nativo (Fase 2-3):** Tauri + Rust (acceso al sistema operativo).
- **Persistencia (Fase 4):** SQLite local + Drizzle ORM.

## Fase 1 — Inicialización (estado actual)

Base de la interfaz y reglas técnicas:

- Next.js 16.3 con App Router (sin `src/`, alias `@/*`).
- TypeScript estricto (`strict`, `noUncheckedIndexedAccess`, etc.).
- ESLint (config de Next 16) + Prettier (plugin Tailwind).
- Tailwind CSS v4.
- Estructura de carpetas por dominio (`app/`, `components/`, `lib/`, `features/`, `native/`, `tests/`).
- Componentes UI reutilizables (`Button`, `Card`).

## Scripts

| Comando             | Acción                      |
| ------------------- | --------------------------- |
| `npm run dev`       | Servidor de desarrollo      |
| `npm run build`     | Build de producción         |
| `npm run start`     | Sirve el build              |
| `npm run lint`      | ESLint                      |
| `npm run format`    | Prettier (write)            |
| `npm run typecheck` | Comprobación de tipos (tsc) |

## Estructura

```
app/         rutas por sección (dashboard, history, applications, ...)
components/  ui/ dashboard/ charts/ forms/
lib/         db/ analytics/ validation/ utils/
features/    módulos por dominio (monitoring, applications, ...)
native/      agente nativo (Rust/Tauri) — Fase 2+
tests/       pruebas (Vitest / Playwright)
```

Ver `E:\Planes para aplicaciones\plan_creacion_monitorizacion_nextjs16.qmd`.
