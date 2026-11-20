import type { PreparedMedia } from "./client";

export interface PlaybackSelection {
  streamIndex: number; serverId?: string; resolverId?: string; resolverHost?: string;
  resolverStreamIndex: number; resolverServers: Record<string, string>;
}
function servers(): Record<string, string> {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem("hpp.playback.servers") || "{}");
    return raw && typeof raw === "object" && !Array.isArray(raw)
      ? Object.fromEntries(Object.entries(raw).filter(([, value]) => typeof value === "string")) : {};
  } catch { return {}; }
}
const key = (stage: "source" | "resolver", id: string) => JSON.stringify([stage, id]);
export function initialPlaybackSelection(pluginId: string): PlaybackSelection {
  const saved = servers();
  return { streamIndex: 0, resolverStreamIndex: 0, serverId: saved[key("source", pluginId)],
    resolverServers: Object.fromEntries(Object.entries(saved).flatMap(([entry, value]) => {
      try { const [stage, id] = JSON.parse(entry); return stage === "resolver" && typeof id === "string" ? [[id, value]] : []; } catch { return []; }
    })) };
}
export function rememberPlaybackSelection(media: PreparedMedia, selection: PlaybackSelection): PlaybackSelection {
  const { source, resolver } = media.choices, saved = servers();
  const remember = (stage: "source" | "resolver", id: string, server: string | null | undefined) => {
    if (server) saved[key(stage, id)] = server; else delete saved[key(stage, id)];
  };
  remember("source", source.pluginId, source.serverId);
  if (resolver) remember("resolver", resolver.pluginId, resolver.serverId);
  try { localStorage.setItem("hpp.playback.servers", JSON.stringify(saved)); } catch { /* Playback remains usable without persistence. */ }
  return { ...selection, streamIndex: source.streamIndex, serverId: source.serverId || undefined,
    resolverId: media.choices.resolverRoute?.forced ? undefined : resolver?.pluginId,
    resolverHost: media.choices.resolverRoute?.host,
    resolverStreamIndex: resolver?.streamIndex || 0,
    resolverServers: resolver?.serverId ? { ...selection.resolverServers, [resolver.pluginId]: resolver.serverId } : selection.resolverServers };
}
