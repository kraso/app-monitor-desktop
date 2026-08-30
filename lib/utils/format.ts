export function formatDuration(ms: number): string {
  if (ms < 0) ms = 0;

  const totalSeconds = Math.floor(ms / 1000);
  const milliseconds = ms % 1000;

  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = totalSeconds % 60;

  const parts: string[] = [];

  if (h > 0) parts.push(`${h}h`);
  if (m > 0 || h > 0) parts.push(`${m}m`);
  if (s > 0 || m > 0 || h > 0) parts.push(`${s}s`);
  if (milliseconds > 0 || parts.length === 0) parts.push(`${milliseconds}ms`);

  return parts.join(" ");
}