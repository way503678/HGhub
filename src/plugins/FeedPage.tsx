import PluginMediaDetail from "./PluginMediaDetail";
import { useStore } from "../store";
import { useEffect, useMemo, useRef, useState } from "react";
import type { Episode, MediaDetail, MediaItem, Page } from "./protocol";
import { pluginCommand, pluginError, type InstalledPlugin } from "./client";
import { usePluginRequests } from "./usePluginRequests";
import { useHost, libraryKey, type LocalProgress } from "./HostContext";
import { Notice } from "./BrowsePages";
import ContentPlayer from "./ContentPlayer";
import CommentsPanel from "./CommentsPanel";
import MediaInteractions from "./MediaInteractions";
import { pluginProgressReporter } from "./progressReporter";
import { IStar, IStarF, IMore, IClose } from "../icons";

export default function FeedPage({ plugin, onBack, active, openMedia }: {
  plugin: InstalledPlugin; onBack: () => void; active: boolean; openMedia: (mediaId: string) => void;
}) {
  const request = usePluginRequests(), host = useHost(), epoch = useRef(0), lock = useRef(false);
  const [page, setPage] = useState<Page<MediaItem> | null>(null), [index, setIndex] = useState(0);
  const [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const failedCursor = useRef<string | undefined>(undefined), advanceAfterRetry = useRef(false);
  const load = async (cursor?: string, advance = false) => {
    if (lock.current) return;
    lock.current = true;
    const n = ++epoch.current; setBusy(true); setError("");
    failedCursor.current = cursor; advanceAfterRetry.current = advance;
    try {
      const result = await request<Page<MediaItem>>("call", { id: plugin.meta.id, method: "feed.list", args: { cursor, limit: 20 } });
      if (n !== epoch.current) return;
      const oldItems = cursor ? page?.items || [] : [], ids = new Set(oldItems.map(i => i.id));
      const added = result.items.filter(item => { if (ids.has(item.id)) return false; ids.add(item.id); return true; });
      setPage({ ...result, next: cursor && result.next === cursor ? null : result.next, items: [...oldItems, ...added] });
      if (!cursor) setIndex(0);
      else if (advance && added.length) setIndex(oldItems.length);
    } catch (e) {
      if (n === epoch.current && (e as { code?: string }).code !== "CANCELLED") setError(pluginError(e));
    } finally { if (n === epoch.current) { lock.current = false; setBusy(false); } }
  };
  useEffect(() => { void load(); return () => { epoch.current++; lock.current = false; request.cancelAll(); }; }, [plugin.meta.id, plugin.revision]);
  const next = () => {
    if (page && index + 1 < page.items.length) setIndex(index + 1);
    else if (page?.next) void load(page.next, true);
  };
  const previous = () => setIndex(i => Math.max(0, i - 1));
  const current = page?.items[index];
  const app=useStore();
  return <div className="legacy-home">{current ? <PluginMediaDetail key={current.id} plugin={plugin} mediaId={current.id} entryEpisodeId={current.entryEpisodeId} entryItem={current} active={active} onBack={onBack} manage={()=>app.push({kind:"plugins",focusId:plugin.meta.id})} onSwitch={(pluginId,mediaId,start)=>app.push({kind:"media",pluginId,mediaId,start})} presentation={{title:current.title,onNext:page?.next||index+1<(page?.items.length||0)?next:undefined,onPrevious:index?previous:undefined,queueIndex:index,total:page?.items.length}}/>:<div className="hpp-empty"><button onClick={onBack}>返回</button><Notice error={error}/>{busy?"正在读取信息流…":"暂无信息流"}<button onClick={()=>void load(failedCursor.current,advanceAfterRetry.current)}>重试信息流</button></div>}{current && error && <div className="hpp-home-errors"><Notice error={error}/><button disabled={busy} onClick={()=>void load(failedCursor.current,advanceAfterRetry.current)}>重试信息流</button></div>}</div>;
}

export function FeedEntry({plugin,item,active,onEnded,openMedia,immersive=false}:{plugin:InstalledPlugin;item:MediaItem;active:boolean;onEnded:()=>void;openMedia:(id:string)=>void;immersive?:boolean}) {
 const app=useStore();
 return <PluginMediaDetail plugin={plugin} mediaId={item.id} entryEpisodeId={item.entryEpisodeId} entryItem={item} active={active} manage={()=>app.push({kind:"plugins",focusId:plugin.meta.id})} onSwitch={(id,mediaId)=>app.push({kind:"media",pluginId:id,mediaId})} presentation={{title:item.title,onNext:onEnded}}/>;
}
