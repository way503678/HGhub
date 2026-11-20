import { createElement, useEffect, useMemo, useState, type ReactNode } from "react";
import type { ArticleDetail, ArticleItem, Page } from "./protocol";
import { openPluginLink, pluginError, type InstalledPlugin } from "./client";
import { usePluginRequests } from "./usePluginRequests";
import { BrowseFrame, Notice } from "./BrowsePages";
import { useAssetUrls } from "./MediaCards";
import ContentPlayer from "./ContentPlayer";
import CommentsPanel from "./CommentsPanel";
import MediaInteractions from "./MediaInteractions";

export function ArticleList({ plugin, onBack, open }: { plugin: InstalledPlugin; onBack: () => void; open: (articleId: string) => void }) {
  const request = usePluginRequests(), [page, setPage] = useState<Page<ArticleItem> | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState("");
  const covers = useAssetUrls(plugin.meta.id, page?.items.flatMap(i => i.cover ? [i.cover] : []) || []); let coverIndex = 0;
  const load = async (cursor?: string) => {
    setBusy(true); setError("");
    try {
      const result = await request<Page<ArticleItem>>("call", { id: plugin.meta.id, method: "articles.list", args: { cursor, limit: 40 } });
      setPage(old => ({ ...result, items: cursor && old ? [...old.items, ...result.items.filter(i => !old.items.some(o => o.id === i.id))] : result.items }));
    } catch (e) { setError(pluginError(e)); } finally { setBusy(false); }
  };
  useEffect(() => { void load(); }, []);
  return <BrowseFrame title={`${plugin.meta.name} · 图文`} tools={<button onClick={onBack}>返回</button>}><Notice error={error}/>
    {page?.items.map(item => <ArticleCard key={item.id} cover={item.cover ? covers[coverIndex++] : undefined} item={item} open={() => open(item.id)}/>)}
    {busy && <p role="status">正在读取图文…</p>}{error && <button onClick={() => void load()}>重试图文</button>}{page && !page.items.length && <p>暂无图文</p>}
    {page?.next && <button disabled={busy} onClick={() => void load(page.next || undefined)}>更多图文</button>}
  </BrowseFrame>;
}
function ArticleCard({ cover, item, open }: { cover?: string; item: ArticleItem; open: () => void }) {
  return <button className="hpp-article-card" onClick={open}>{cover && <img src={cover} alt="" loading="lazy"/>}<div><h2>{item.title}</h2><p>{item.summary}</p>{item.author && <small>{item.author.name}</small>}</div></button>;
}
export default function ArticlePage({ plugin, articleId, onBack, openMedia, active }: {
  plugin: InstalledPlugin; articleId: string; onBack: () => void; openMedia: (mediaId: string) => void; active: boolean;
}) {
  const request = usePluginRequests(), [detail, setDetail] = useState<ArticleDetail | null>(null), [error, setError] = useState("");
  const [play, setPlay] = useState(false);
  useEffect(() => { void request<ArticleDetail>("call", { id: plugin.meta.id, method: "articles.getDetail", args: { id: articleId } }).then(setDetail).catch(e => setError(pluginError(e))); }, []);
  return <BrowseFrame title={detail?.title || "图文详情"} tools={<button onClick={onBack}>返回</button>}><Notice error={error}/>
    {detail && <article className="hpp-article-detail"><p className="hpp-muted">{plugin.meta.name}{detail.author ? ` · ${detail.author.name}` : ""}{detail.publishedAt !== undefined ? ` · ${new Date(detail.publishedAt).toLocaleDateString()}` : ""}</p>
      {detail.contentHtml !== undefined ? <ArticleHtml plugin={plugin} html={detail.contentHtml}/> : <p className="hpp-article-text">{detail.contentText || detail.summary}</p>}
      {!!detail.streams?.length && (play ? <ContentPlayer plugin={plugin} action="article.prepare" contentId={articleId} active={active}/> : <button onClick={() => setPlay(true)}>播放文内视频</button>)}
      {detail.mediaId && plugin.methods.includes("media.getDetail") && <button onClick={() => openMedia(detail.mediaId!)}>查看关联作品</button>}
      {active && <><MediaInteractions plugin={plugin} detail={detail} kind="article"/><CommentsPanel plugin={plugin} articleId={articleId}/></>}
    </article>}
  </BrowseFrame>;
}

type RichNode = string | { tag: string; children: RichNode[]; href?: string; image?: number; alt?: string };
const allowedTags = new Set(["p", "div", "span", "br", "hr", "h1", "h2", "h3", "h4", "h5", "h6", "strong", "b", "em", "i", "u", "s", "del", "blockquote", "ul", "ol", "li", "pre", "code", "table", "thead", "tbody", "tr", "th", "td", "figure", "figcaption", "a", "img"]);
const discardTags = new Set(["script", "style", "iframe", "form", "input", "textarea", "select", "button", "object", "embed", "svg", "math", "video", "audio", "source", "link", "meta", "base", "template"]);
function articleUrl(raw: string | null, insecure: boolean) {
  if (!raw || /[\u0000-\u0020\u007f]/.test(raw)) return;
  try { const u = new URL(raw); if ((u.protocol === "https:" || insecure && u.protocol === "http:") && !u.username && !u.password) return u.href; } catch { /* Invalid resource stays absent. */ }
}
/** Parse in inert template storage. Rebuild every visible node from an allow-list. */
export function parseArticleHtml(html: string, insecure: boolean) {
  const template = document.createElement("template"); template.innerHTML = html;
  const images: string[] = [];
  const visit = (node: Node, depth: number): RichNode[] => {
    if (depth > 40) return [];
    if (node.nodeType === Node.TEXT_NODE) return [node.textContent || ""];
    if (!(node instanceof Element)) return [];
    const tag = node.localName;
    if (discardTags.has(tag)) return [];
    const children = Array.from(node.childNodes).flatMap(child => visit(child, depth + 1));
    if (!allowedTags.has(tag)) return children;
    if (tag === "img") {
      const url = articleUrl(node.getAttribute("src"), insecure); if (!url) return [];
      const image = images.length; images.push(url);
      return [{ tag, children: [], image, alt: (node.getAttribute("alt") || "").slice(0, 200) }];
    }
    return [{ tag, children, ...(tag === "a" ? { href: articleUrl(node.getAttribute("href"), insecure) } : {}) }];
  };
  return { nodes: Array.from(template.content.childNodes).flatMap(n => visit(n, 0)), images };
}
function ArticleHtml({ plugin, html }: { plugin: InstalledPlugin; html: string }) {
  const tree = useMemo(() => parseArticleHtml(html, !!plugin.meta.allowInsecureHttp), [html, plugin.meta.allowInsecureHttp]);
  const urls = useAssetUrls(plugin.meta.id, tree.images), [error, setError] = useState("");
  const render = (nodes: RichNode[], prefix = ""): ReactNode[] => nodes.map((node, i) => {
    const key = `${prefix}/${i}`;
    if (typeof node === "string") return node;
    if (node.tag === "img") return urls[node.image!] ? <img key={key} src={urls[node.image!]} alt={node.alt} loading="lazy"/> : null;
    if (node.tag === "br" || node.tag === "hr") return createElement(node.tag, { key });
    if (node.tag === "a") return node.href ? <a key={key} href={node.href} onClick={e => { e.preventDefault(); void openPluginLink(node.href!).catch(e => setError(pluginError(e))); }}>{render(node.children, key)}</a> : <span key={key}>{render(node.children, key)}</span>;
    return createElement(node.tag, { key }, render(node.children, key));
  });
  return <><Notice error={error}/><div className="hpp-article-html">{render(tree.nodes)}</div></>;
}
