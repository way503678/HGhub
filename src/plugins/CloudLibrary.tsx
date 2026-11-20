import { useEffect, useRef, useState } from "react";
import type { Author, MediaItem, Page } from "./protocol";
import { onPluginAuthFailure, pluginError, type InstalledPlugin } from "./client";
import { useHost } from "./HostContext";
import { usePluginRequests } from "./usePluginRequests";
import MediaCards, { useAssetUrls } from "./MediaCards";
import { Notice, type OpenMedia } from "./BrowsePages";

type CloudKind = "favorites" | "likes" | "history" | "following";
type Folder = { id: string; label: string; count?: number };
export default function CloudLibrary({ plugin: p, open }: { plugin: InstalledPlugin; open: OpenMedia }) {
  const host = useHost(), request = usePluginRequests(), epoch = useRef(0);
  const [kind, setKind] = useState<CloudKind>("favorites"), [folderId, setFolderId] = useState("");
  const [folders, setFolders] = useState<Folder[]>([]), [page, setPage] = useState<Page<MediaItem> | null>(null), [following, setFollowing] = useState<Page<Author> | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [notice, setNotice] = useState("");
  useEffect(() => onPluginAuthFailure(f => {
    if (f.id === p.meta.id) { epoch.current++; request.cancelAll(); setPage(null); setFollowing(null); setFolders([]); setFolderId(""); setBusy(false); if (!f.silent) setError(f.message); }
  }), [p.meta.id, request]);
  const load = async (nextKind: CloudKind, cursor?: string, folder = "") => {
    const n = ++epoch.current; request.cancelAll(); setKind(nextKind); setFolderId(folder); setBusy(true); setError(""); setNotice("");
    if (!cursor) { setPage(null); setFollowing(null); }
    try {
      if (nextKind === "following") {
        const result = await request<Page<Author>>("call", { id: p.meta.id, method: "library.listFollowing", args: { cursor, limit: 40 } });
        if (n === epoch.current) setFollowing(old => ({ ...result, items: cursor && old ? merge(old.items, result.items) : result.items }));
      } else {
        const result = await request<Page<MediaItem>>("call", { id: p.meta.id, method: "library.listMedia", args: { kind: nextKind, cursor, limit: 40, ...(nextKind === "favorites" && folder ? { folderId: folder } : {}) } });
        if (n === epoch.current) setPage(old => ({ ...result, items: cursor && old ? merge(old.items, result.items) : result.items }));
        if (nextKind === "favorites" && !cursor && p.methods.includes("library.listFolders")) {
          try {
            const rows = await request<Folder[]>("call", { id: p.meta.id, method: "library.listFolders", args: {} });
            if (n === epoch.current) setFolders(rows);
          } catch (e) { if (n === epoch.current && (e as { code?: string })?.code !== "UNSUPPORTED") setError(pluginError(e)); }
        }
      }
    } catch (e) { if (n === epoch.current) setError(pluginError(e)); }
    finally { if (n === epoch.current) setBusy(false); }
  };
  const importPage = async () => {
    if (!page || busy) return; setBusy(true); setError("");
    try { for (const item of page.items) await host.put(p.meta.id, item, { favorite: true }); setNotice("本页条目已导入本地收藏"); }
    catch (e) { setError(pluginError(e)); } finally { setBusy(false); }
  };
  useEffect(() => { void load("favorites"); return () => { epoch.current++; request.cancelAll(); }; }, [p.meta.id, p.revision]);
  return <div className="hpp-cloud-library"><Notice error={error}/><div className="hpp-source-tabs">
    {([["favorites", "云端收藏"], ["likes", "云端点赞"], ["history", "云端历史"]] as const).map(([id, label]) => <button key={id} disabled={busy} aria-pressed={kind === id} onClick={() => void load(id)}>{label}</button>)}
    {p.methods.includes("library.listFollowing") && <button disabled={busy} aria-pressed={kind === "following"} onClick={() => void load("following")}>云端关注</button>}
  </div>
    {kind === "favorites" && folders.length > 0 && <label>收藏目录<select aria-label={`${p.meta.name}收藏目录`} value={folderId} disabled={busy} onChange={e => void load("favorites", undefined, e.target.value)}><option value="">全部收藏</option>{folders.map(f => <option key={f.id} value={f.id}>{f.label}{f.count === undefined ? "" : ` · ${f.count}`}</option>)}</select></label>}
    {kind === "following" ? <>{following?.items.map(author => <FollowingAuthor key={author.id} plugin={p} author={author} removed={() => setFollowing(old => old ? { ...old, items: old.items.filter(a => a.id !== author.id) } : old)}/>)}{following?.next && <button disabled={busy} onClick={() => void load(kind, following.next || undefined)}>更多关注</button>}</>
      : <><MediaCards pluginId={p.meta.id} items={page?.items || []} open={i => open(p.meta.id, i.id)}/>{page?.next && <button disabled={busy} onClick={() => void load(kind, page.next || undefined, folderId)}>更多云端条目</button>}{!!page?.items.length && <button disabled={busy} onClick={() => void importPage()}>本页导入到本地片库</button>}</>}
    {busy && <p role="status">正在读取云端片库…</p>}{notice && <p role="status">{notice}</p>}
  </div>;
}
function merge<T extends { id: string }>(a: T[], b: T[]) { const ids = new Set(a.map(i => i.id)); return [...a, ...b.filter(i => { if (ids.has(i.id)) return false; ids.add(i.id); return true; })]; }
function FollowingAuthor({ plugin, author, removed }: { plugin: InstalledPlugin; author: Author; removed: () => void }) {
  const request = usePluginRequests(), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const avatar = useAssetUrls(plugin.meta.id, author.avatar ? [author.avatar] : []);
  return <article className="hpp-following-author"><div className="hpp-profile">{avatar[0] && <img src={avatar[0]} alt=""/>}<strong>{author.name}</strong></div><p className="hpp-muted">{author.description}</p><Notice error={error}/>
    {plugin.methods.includes("interact.follow") && <button disabled={busy} onClick={() => { setBusy(true); setError(""); void request("call", { id: plugin.meta.id, method: "interact.follow", args: { authorId: author.id, on: false } }).then(removed).catch(e => setError(pluginError(e))).finally(() => setBusy(false)); }}>取消关注 · {author.name}</button>}
  </article>;
}
