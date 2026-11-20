import { useEffect, useRef, useState } from "react";
import type { MediaDetail } from "./protocol";
import { onPluginAuthFailure, onPluginAuthSuccess, pluginError, type InstalledPlugin } from "./client";
import { usePluginRequests } from "./usePluginRequests";
import { Notice } from "./BrowsePages";
import { IHeart, IHeartF } from "../icons";

export default function MediaInteractions({ plugin, detail, kind = "media", compact = false }: { plugin: InstalledPlugin; detail: Pick<MediaDetail, "id" | "interaction" | "author">; kind?: "media" | "article"; compact?: boolean }) {
  const request = usePluginRequests(), lock = useRef(false);
  const [liked, setLiked] = useState(detail.interaction?.liked), [favorite, setFavorite] = useState(detail.interaction?.favorited), [followed, setFollowed] = useState(false);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const methods = plugin.methods;
  const authEpoch = useRef(0);
  useEffect(() => onPluginAuthFailure(f => {
    if (f.id === plugin.meta.id) { authEpoch.current++; setLiked(undefined); setFavorite(undefined); setFollowed(false); }
  }), [plugin.meta.id]);
  useEffect(() => onPluginAuthSuccess(id => {
    if (id !== plugin.meta.id) return;
    const epoch = ++authEpoch.current;
    void request<MediaDetail>("call", { id, method: kind === "article" ? "articles.getDetail" : "media.getDetail", args: { id: detail.id } }).then(result => {
      if (epoch === authEpoch.current) { setLiked(result.interaction?.liked); setFavorite(result.interaction?.favorited); setFollowed(false); }
    }).catch(() => {});
  }), [plugin.meta.id, detail.id, kind, request]);
  const act = async (method: string, args: Record<string, unknown>, done: () => void) => {
    if (lock.current) return; lock.current = true; setBusy(true); setError("");
    try { await request("call", { id: plugin.meta.id, method, args }); done(); }
    catch (e) { if ((e as { code?: string })?.code !== "CANCELLED") setError(pluginError(e)); }
    finally { lock.current = false; setBusy(false); }
  };
  if (!methods.some(m => ["interact.like", "interact.favorite"].includes(m)) && !(detail.author && methods.includes("interact.follow"))) return null;
  const target = kind === "article" ? { kind, articleId: detail.id } : { kind, mediaId: detail.id };
  if (compact) return methods.includes("interact.like") ? <><button aria-label={liked ? "取消云端点赞" : "云端点赞"} disabled={busy} aria-pressed={!!liked} onClick={()=>void act("interact.like",{target,on:!liked},()=>setLiked(!liked))}>{liked?<IHeartF size={30} className="text-neon"/>:<IHeart size={30}/>}<span>点赞</span></button>{error&&<span role="alert" className="text-xs max-w-24">{error}</span>}</> : null;
  return <section aria-label="云端互动" className="hpp-interactions"><p className="hpp-muted">云端互动 · {plugin.meta.name}</p><Notice error={error}/><div className="hpp-source-tabs">
    {methods.includes("interact.like") && <button disabled={busy} aria-pressed={!!liked} onClick={() => void act("interact.like", { target, on: !liked }, () => setLiked(!liked))}>{liked ? "取消云端点赞" : "云端点赞"}</button>}
    {methods.includes("interact.favorite") && <button disabled={busy} aria-pressed={!!favorite} onClick={() => void act("interact.favorite", { target, on: !favorite }, () => setFavorite(!favorite))}>{favorite ? "取消云端收藏" : "云端收藏"}</button>}
    {detail.author && methods.includes("interact.follow") && <button disabled={busy} aria-pressed={followed} onClick={() => void act("interact.follow", { authorId: detail.author!.id, on: !followed }, () => setFollowed(!followed))}>{followed ? "取消关注作者" : "关注作者"} · {detail.author.name}</button>}
  </div></section>;
}
