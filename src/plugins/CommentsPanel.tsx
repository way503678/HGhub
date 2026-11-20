import { useEffect, useRef, useState } from "react";
import type { Comment, CommentTarget, Page } from "./protocol";
import { pluginError, type InstalledPlugin } from "./client";
import { usePluginRequests } from "./usePluginRequests";
import { useAssetUrls } from "./MediaCards";
import { Notice } from "./BrowsePages";

export default function CommentsPanel({ plugin, mediaId, episodeId, articleId, embedded = false }: { plugin: InstalledPlugin; mediaId?: string; episodeId?: string; articleId?: string; embedded?: boolean }) {
  const [open, setOpen] = useState(embedded), [scope, setScope] = useState("media");
  if (!plugin.methods.includes("comments.list")) return null;
  const target: CommentTarget = articleId ? { kind: "article", articleId } : { kind: "media", mediaId: mediaId!, ...(scope === "episode" && episodeId ? { episodeId } : {}) };
  return <section className="hpp-comments" aria-label="评论面板">
    {!embedded && <div className="hpp-section-title"><h3>评论</h3><button aria-expanded={open} onClick={() => setOpen(v => !v)}>{open ? "收起评论" : "查看评论"}</button></div>}
    {open && <>{!articleId && <div className="hpp-source-tabs"><button aria-pressed={scope === "media"} onClick={() => setScope("media")}>作品评论</button>{episodeId && <button aria-pressed={scope === "episode"} onClick={() => setScope("episode")}>当前分集评论</button>}</div>}
      <CommentList key={JSON.stringify(target)} plugin={plugin} target={target}/></>}
  </section>;
}

function mergeComments(old: Comment[], added: Comment[]) {
  const ids = new Set(old.map(c => c.id));
  return [...old, ...added.filter(c => { if (ids.has(c.id)) return false; ids.add(c.id); return true; })];
}
function CommentList({ plugin, target, onUnsupported }: { plugin: InstalledPlugin; target: CommentTarget; onUnsupported?: () => void }) {
  const request = usePluginRequests(), epoch = useRef(0), lock = useRef(false);
  const [page, setPage] = useState<Page<Comment> | null>(null), [error, setError] = useState("");
  const [loading, setLoading] = useState(false), [sending, setSending] = useState(false), [unsupported, setUnsupported] = useState(false), [canSend, setCanSend] = useState(true), [text, setText] = useState("");
  const failure = (e: unknown) => {
    if ((e as { code?: string })?.code === "UNSUPPORTED") { setUnsupported(true); onUnsupported?.(); }
    else if ((e as { code?: string })?.code !== "CANCELLED") setError(pluginError(e));
  };
  const load = async (cursor?: string) => {
    const n = ++epoch.current; setLoading(true); setError("");
    try {
      const result = await request<Page<Comment>>("call", { id: plugin.meta.id, method: "comments.list", args: { target, cursor, limit: 30 } });
      if (n === epoch.current) setPage(old => ({ ...result, next: cursor && result.next === cursor ? null : result.next, items: cursor && old ? mergeComments(old.items, result.items) : result.items }));
    } catch (e) { if (n === epoch.current) failure(e); }
    finally { if (n === epoch.current) setLoading(false); }
  };
  useEffect(() => { void load(); return () => { epoch.current++; }; }, []);
  const send = async () => {
    if (lock.current || !text.trim()) return;
    lock.current = true; setSending(true); setError("");
    try {
      const result = await request<Comment | null>("call", { id: plugin.meta.id, method: "comments.send", args: { target, text: text.trim() } });
      setText("");
      if (result) setPage(old => ({ ...old, items: mergeComments([result], old?.items || []) }));
      else await load();
    } catch (e) {
      if ((e as { code?: string })?.code === "UNSUPPORTED") { setCanSend(false); setError("此来源不支持向这里发送评论。"); }
      else failure(e);
    }
    finally { lock.current = false; setSending(false); }
  };
  return <div className="hpp-comment-list" aria-label={target.kind === "replies" ? "回复列表" : "评论列表"}>
    <Notice error={error}/>{unsupported ? <p className="hpp-muted">此来源不支持这类评论。</p> : <>
      {page?.items.map(comment => <CommentRow key={comment.id} plugin={plugin} comment={comment}/>)}
      {loading && <p role="status">正在读取评论…</p>}{!loading && page && !page.items.length && <p className="hpp-muted">还没有评论。</p>}
      {page?.next && <button disabled={loading || sending} onClick={() => void load(page.next || undefined)}>更多评论</button>}
      {!page && !loading && error && <button onClick={() => void load()}>重试评论</button>}
      {canSend && plugin.methods.includes("comments.send") && <form className="hpp-comment-compose" onSubmit={e => { e.preventDefault(); void send(); }}>
        <label>写{target.kind === "replies" ? "回复" : "评论"}<textarea aria-label={target.kind === "replies" ? "回复内容" : "评论内容"} value={text} maxLength={5000} required disabled={sending} onChange={e => setText(e.target.value)}/></label>
        <button type="submit" disabled={loading || sending || !text.trim()}>{sending ? "正在发送…" : target.kind === "replies" ? "发送回复" : "发送评论"}</button>
      </form>}
    </>}
  </div>;
}
function CommentRow({ plugin, comment }: { plugin: InstalledPlugin; comment: Comment }) {
  const request = usePluginRequests(), lock = useRef(false);
  const [liked, setLiked] = useState(comment.liked), [likes, setLikes] = useState(comment.likes);
  const [busy, setBusy] = useState(false), [error, setError] = useState(""), [replies, setReplies] = useState(false), [unsupported, setUnsupported] = useState(false), [canLike, setCanLike] = useState(true);
  const avatar = useAssetUrls(plugin.meta.id, comment.author.avatar ? [comment.author.avatar] : []);
  useEffect(() => { setLiked(comment.liked); setLikes(comment.likes); }, [comment]);
  const like = async () => {
    if (lock.current) return; lock.current = true; setBusy(true); setError("");
    try {
      await request("call", { id: plugin.meta.id, method: "comments.like", args: { commentId: comment.id, on: !liked } });
      setLiked(!liked); if (likes !== undefined) setLikes(Math.max(0, likes + (liked ? -1 : 1)));
    } catch (e) {
      if ((e as { code?: string })?.code === "UNSUPPORTED") setCanLike(false);
      else if ((e as { code?: string })?.code !== "CANCELLED") setError(pluginError(e));
    }
    finally { lock.current = false; setBusy(false); }
  };
  return <article className="hpp-comment"><div className="hpp-comment-author">{avatar[0] && <img src={avatar[0]} alt="" loading="lazy"/>}<strong>{comment.author.name}</strong>{comment.pinned && <small>置顶</small>}{comment.region && <small>{comment.region}</small>}{comment.createdAt !== undefined && <time>{new Date(comment.createdAt).toLocaleString()}</time>}</div>
    <p className="hpp-comment-text">{comment.text}</p><Notice error={error}/><div className="hpp-source-tabs">
      {canLike && plugin.methods.includes("comments.like") && <button disabled={busy} aria-pressed={!!liked} onClick={() => void like()}>{liked ? "取消评论点赞" : "点赞评论"}{likes !== undefined ? ` · ${likes}` : ""}</button>}
      {!unsupported && <button aria-expanded={replies} onClick={() => setReplies(v => !v)}>{replies ? "收起回复" : `查看回复${comment.replyCount !== undefined ? ` · ${comment.replyCount}` : ""}`}</button>}
    </div>{replies ? <CommentList plugin={plugin} target={{ kind: "replies", commentId: comment.id }} onUnsupported={() => { setUnsupported(true); setReplies(false); }}/>
      : !!comment.replies?.length && <div className="hpp-comment-preview">{comment.replies.map(reply => <p key={reply.id}><strong>{reply.author.name}：</strong>{reply.text}</p>)}</div>}
  </article>;
}
