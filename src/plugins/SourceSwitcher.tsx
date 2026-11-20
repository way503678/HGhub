import { useEffect, useRef, useState } from "react";
import { useHost, type LibraryEntry, type LocalProgress } from "./HostContext";
import { pluginError, type InstalledPlugin } from "./client";
import { usePluginRequests } from "./usePluginRequests";
import type { Episode, MediaDetail, MediaItem, Page } from "./protocol";
import { Notice } from "./BrowsePages";
import { matchedSourceEpisode, metadataCompatible, sourceEpisodes, sourceScore, type SourceCandidate, type SourceStart } from "./sourceMatching";

export default function SourceSwitcher({ plugin, detail, previousEpisode, getPosition, beforeSwitch, onConfirm, onClose }: {
  plugin: InstalledPlugin; detail: MediaDetail; previousEpisode?: Episode;
  getPosition: () => number; beforeSwitch: () => Promise<void>;
  onConfirm: (pluginId: string, mediaId: string, start?: SourceStart) => void; onClose: () => void;
}) {
  const host = useHost(), request = usePluginRequests(), searchEpoch = useRef(0), targetEpoch = useRef(0);
  const [hits, setHits] = useState<SourceCandidate[]>([]), [linked, setLinked] = useState<LibraryEntry[]>([]);
  const [errors, setErrors] = useState<Record<string, string>>({}), [loading, setLoading] = useState<Record<string, boolean>>({});
  const [chosen, setChosen] = useState<SourceCandidate | null>(null), [target, setTarget] = useState<MediaDetail | null>(null);
  const [episodeKey, setEpisodeKey] = useState(""), [saved, setSaved] = useState<LocalProgress | null>(null), [usePosition, setUsePosition] = useState(false);
  const [error, setError] = useState(""), [busy, setBusy] = useState(false), [progressReady, setProgressReady] = useState(false);
  const [pages, setPages] = useState<Record<string, Page<MediaItem>>>({});
  const commitLock = useRef(false), dialog = useRef<HTMLDivElement>(null);
  const available = host.sources.filter(p => p.meta.id !== plugin.meta.id && p.methods.includes("media.getDetail"));
  const searchable = available.filter(p => p.methods.includes("search.search"));
  const availableIds = available.map(p => p.meta.id + p.revision).join("|");
  const search = async (p: InstalledPlugin, n: number, cursor?: string) => {
    setLoading(old => ({ ...old, [p.meta.id]: true })); setErrors(old => ({ ...old, [p.meta.id]: "" }));
    try {
      const result = await request<Page<MediaItem>>("call", { id: p.meta.id, method: "search.search", args: { query: detail.title, cursor, limit: 30 } });
      if (n !== searchEpoch.current) return;
      setPages(old => ({ ...old, [p.meta.id]: { ...result, next: result.next === cursor ? null : result.next } }));
      setHits(old => {
        const keep = cursor ? old : old.filter(h => h.pluginId !== p.meta.id);
        const ids = new Set(keep.filter(h => h.pluginId === p.meta.id).map(h => h.item.id));
        return [...keep, ...result.items.filter(item => { if (ids.has(item.id)) return false; ids.add(item.id); return true; }).map(item => ({ pluginId: p.meta.id, item }))];
      });
    } catch (e) { if (n === searchEpoch.current && (e as { code?: string }).code !== "CANCELLED") setErrors(old => ({ ...old, [p.meta.id]: pluginError(e) })); }
    finally { if (n === searchEpoch.current) setLoading(old => ({ ...old, [p.meta.id]: false })); }
  };
  useEffect(() => {
    const n = ++searchEpoch.current; let live = true;
    setHits([]); setPages({}); setErrors({}); setLoading({}); setChosen(null); setTarget(null); setBusy(false);
    void request<LibraryEntry[]>("library.sources.get", { pluginId: plugin.meta.id, mediaId: detail.id }).then(result => { if (live) setLinked(result); }).catch(e => { if (live) setError(pluginError(e)); });
    for (const source of searchable) void search(source, n);
    return () => { live = false; searchEpoch.current++; targetEpoch.current++; request.cancelAll(); };
  }, [availableIds, plugin.meta.id, detail.id]);
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    dialog.current?.focus();
    const key = (e: KeyboardEvent) => {
      if (e.key === "Escape") { e.stopImmediatePropagation(); if (!commitLock.current) onClose(); }
      if (e.key === "Tab") {
        const nodes = [...dialog.current!.querySelectorAll<HTMLElement>("button:not(:disabled),input:not(:disabled),select:not(:disabled),[tabindex='0']")];
        const first = nodes[0], last = nodes[nodes.length - 1];
        if (e.shiftKey && (document.activeElement === first || document.activeElement === dialog.current)) { e.preventDefault(); last?.focus(); }
        else if (!e.shiftKey && (document.activeElement === last || document.activeElement === dialog.current)) { e.preventDefault(); first?.focus(); }
      }
    };
    window.addEventListener("keydown", key, true);
    return () => { window.removeEventListener("keydown", key, true); previous?.focus(); };
  }, []);
  const choose = async (hit: SourceCandidate) => {
    const n = ++targetEpoch.current; setChosen(hit); setTarget(null); setEpisodeKey(""); setUsePosition(false); setError(""); setBusy(true);
    try {
      const result = await request<MediaDetail>("call", { id: hit.pluginId, method: "media.getDetail", args: { id: hit.item.id } });
      if (n !== targetEpoch.current) return;
      setTarget(result);
      const match = matchedSourceEpisode(previousEpisode, result);
      if (match && !match.episode.locked) setEpisodeKey(JSON.stringify([match.playlistId, match.episode.id]));
    } catch (e) { if (n === targetEpoch.current) setError(pluginError(e)); }
    finally { if (n === targetEpoch.current) setBusy(false); }
  };
  const selected = target && sourceEpisodes(target).find(e => JSON.stringify([e.playlistId, e.episode.id]) === episodeKey);
  useEffect(() => {
    let live = true; setSaved(null); setUsePosition(false); setProgressReady(false);
    if (chosen && selected) void request<LocalProgress | null>("library.progress.get", { pluginId: chosen.pluginId, mediaId: target!.id, playlistId: selected.playlistId, episodeId: selected.episode.id }).then(result => { if (live) { setSaved(result); setProgressReady(true); } }).catch(e => { if (live) setError(pluginError(e)); });
    return () => { live = false; };
  }, [chosen?.pluginId, target?.id, episodeKey]);
  const more = async (playlistId: string, cursor: string) => {
    if (!chosen) return;
    const n = targetEpoch.current; setBusy(true); setError("");
    try {
      const result = await request<Page<Episode>>("call", { id: chosen.pluginId, method: "media.listEpisodes", args: { mediaId: target!.id, playlistId, cursor, limit: 100 } });
      if (n === targetEpoch.current) setTarget(old => old && ({ ...old, playlists: old.playlists.map(p => {
        if (p.id !== playlistId) return p;
        const ids = new Set(p.episodes.map(e => e.id));
        return { ...p, next: result.next === cursor ? null : result.next, episodes: [...p.episodes, ...result.items.filter(e => { if (ids.has(e.id)) return false; ids.add(e.id); return true; })] };
      }) }));
    } catch (e) { if (n === targetEpoch.current) setError(pluginError(e)); }
    finally { if (n === targetEpoch.current) setBusy(false); }
  };
  const confirm = async () => {
    if (!chosen || !target || commitLock.current) return;
    commitLock.current = true; setBusy(true); setError("");
    try {
      const seconds = getPosition();
      await beforeSwitch();
      await request("library.sources.link", { from: { pluginId: plugin.meta.id, item: detail }, to: { pluginId: chosen.pluginId, item: target } });
      await host.refreshLibrary();
      onConfirm(chosen.pluginId, target.id, selected ? {
        playlistId: selected.playlistId, episodeId: selected.episode.id,
        positionSec: saved?.positionSec ?? (usePosition && Number.isFinite(seconds) && seconds >= 0 ? seconds : 0),
      } : undefined);
    } catch (e) { setError(pluginError(e)); }
    finally { commitLock.current = false; setBusy(false); }
  };
  const candidates: SourceCandidate[] = [];
  for (const hit of [...linked.map(e => ({ pluginId: e.pluginId, item: e.item })), ...hits]) {
    if (available.some(p => p.meta.id === hit.pluginId) && !candidates.some(c => c.pluginId === hit.pluginId && c.item.id === hit.item.id)) candidates.push(hit);
  }
  candidates.sort((a, b) => sourceScore(detail, b.item) - sourceScore(detail, a.item));
  return <div className="hpp-source-overlay"><div ref={dialog} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="hpp-source-title" className="hpp-page hpp-source-dialog">
    <header className="hpp-header"><h2 id="hpp-source-title">换源 · {detail.title}</h2><div className="hpp-header-tools"><button disabled={commitLock.current} onClick={onClose}>取消换源</button></div></header>
    <div className="hpp-scroll"><div className="hpp-content"><Notice error={error}/>
      {!chosen ? <><p className="hpp-muted">确认其他来源后保留原来源的收藏和进度。</p>
        {!available.length && <p>没有其他可用数据源，请先导入并启用数据源。</p>}
        <div className="hpp-source-candidates">{candidates.map(hit => <button key={JSON.stringify([hit.pluginId, hit.item.id])} onClick={() => void choose(hit)}><strong>{hit.item.title}</strong><span>{available.find(p => p.meta.id === hit.pluginId)?.meta.name} · {[hit.item.year, hit.item.type].filter(Boolean).join(" · ")}</span><small>{!metadataCompatible(detail, hit.item) ? "年份或类型不同，请确认作品" : sourceScore(detail, hit.item) >= 1000 ? "标题一致" : "相似候选，请确认作品"}{linked.some(e => e.pluginId === hit.pluginId && e.mediaId === hit.item.id) ? " · 已关联" : ""}</small></button>)}</div>
        {searchable.map(p => <section key={p.meta.id}><Notice error={errors[p.meta.id] || ""}/>{loading[p.meta.id] && <p>正在搜索 {p.meta.name}…</p>}{errors[p.meta.id] && <button onClick={() => void search(p, searchEpoch.current)}>重试换源搜索 · {p.meta.name}</button>}{pages[p.meta.id]?.next && <button disabled={loading[p.meta.id]} onClick={() => void search(p, searchEpoch.current, pages[p.meta.id].next!)}>更多换源候选 · {p.meta.name}</button>}</section>)}
        {!candidates.length && !Object.values(loading).some(Boolean) && <p>未找到候选，可重试其他来源或先在搜索页查找。</p>}</>
        : <><button disabled={busy} onClick={() => { targetEpoch.current++; setChosen(null); setTarget(null); setError(""); }}>返回候选来源</button>
          <h3>{available.find(p => p.meta.id === chosen.pluginId)?.meta.name} · {target?.title || chosen.item.title}</h3>
          {busy && !target && <p>正在读取候选详情…</p>}
          {!target && !busy && <button onClick={() => void choose(chosen)}>重试候选详情</button>}
          {target && <><p className="hpp-muted">{matchedSourceEpisode(previousEpisode, target) ? "已按明确季集编号定位候选，请确认。" : "无法唯一对齐季集编号，请选择分集；也可先打开新来源详情。"}</p>
            <label className="hpp-source-episode">新来源分集<select aria-label="新来源分集" disabled={busy} value={episodeKey} onChange={e => setEpisodeKey(e.target.value)}><option value="">先打开详情，不自动播放</option>{sourceEpisodes(target).map(({ playlistId, episode }) => <option key={JSON.stringify([playlistId, episode.id])} value={JSON.stringify([playlistId, episode.id])} disabled={episode.locked}>{target.playlists.find(p => p.id === playlistId)?.label || playlistId} · {episode.season === undefined ? "" : `第 ${episode.season} 季 · `}{episode.label || episode.title || `第 ${episode.index} 集`}{episode.locked ? " · 不可播放" : ""}</option>)}</select></label>
            {available.find(p => p.meta.id === chosen.pluginId)?.methods.includes("media.listEpisodes") && target.playlists.filter(p => p.next).map(p => <button key={p.id} disabled={busy} onClick={() => void more(p.id, p.next!)}>加载候选分集 · {p.label || p.id}</button>)}
            {selected && (saved ? <p role="status">优先使用新来源进度 · {Math.floor(saved.positionSec)} 秒</p> : progressReady && <label className="hpp-check"><input type="checkbox" checked={usePosition} onChange={e => setUsePosition(e.target.checked)}/>沿用原来源播放位置（当前约 {Math.floor(getPosition())} 秒，不同剪辑可能不一致）</label>)}
            <button className="primary" disabled={busy || (!!selected && !progressReady)} onClick={() => void confirm()}>确认换源{selected ? "并播放" : "并打开详情"}</button>
          </>}
        </>}
    </div></div>
  </div></div>;
}
