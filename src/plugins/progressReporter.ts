import { pluginCommand } from "./client";

export interface ProgressReport {
  mediaId: string; episodeId: string; positionSec: number; durationSec?: number;
  event: "start" | "heartbeat" | "pause" | "end";
}

/** One playback consumer; refreshes and stream switches keep its episode identity. */
export function createProgressReporter(mediaId: string, send: (report: ProgressReport) => void, now = () => performance.now()) {
  let episodeId = "", positionSec = 0, durationSec: number | undefined;
  let running = false, started = false, ended = false, heartbeatAt = 0;
  const emit = (event: ProgressReport["event"]) => {
    if (!episodeId) return;
    try { send({ mediaId, episodeId, positionSec, ...(durationSec === undefined ? {} : { durationSec }), event }); } catch { /* Reporting never interrupts playback. */ }
  };
  const pause = () => { if (running && !ended) { running = false; emit("pause"); } };
  return {
    select(id: string, position = 0) {
      pause(); episodeId = id; positionSec = position; durationSec = undefined;
      running = false; started = false; ended = false; heartbeatAt = now();
    },
    update(position: number, duration?: number) {
      if (Number.isFinite(position) && position >= 0) positionSec = position;
      if (duration !== undefined && Number.isFinite(duration) && duration >= 0) durationSec = duration;
      if (running && now() - heartbeatAt >= 30_000) { heartbeatAt = now(); emit("heartbeat"); }
    },
    playing() {
      if (!episodeId) return;
      if (ended) { started = false; ended = false; }
      running = true;
      if (!started) { started = true; heartbeatAt = now(); emit("start"); }
    },
    pause,
    end() { if (episodeId && started && !ended) { running = false; ended = true; emit("end"); } },
  };
}

export function pluginProgressReporter(pluginId: string, mediaId: string, enabled: boolean) {
  return createProgressReporter(mediaId, report => {
    if (enabled) void pluginCommand("call", {
      id: pluginId, method: "progress.report", args: report, requestId: crypto.randomUUID(),
    }, { notifyAuth: false }).catch(() => {});
  });
}
