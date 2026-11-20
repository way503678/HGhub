import { useEffect, useRef, useState } from "react";
import type { Channel, Page, Programme } from "./protocol";
import { type InstalledPlugin, pluginError } from "./client";
import { usePluginRequests } from "./usePluginRequests";
import { BrowseFrame, Notice } from "./BrowsePages";
import { useAssetUrls } from "./MediaCards";
import ContentPlayer from "./ContentPlayer";

export default function LivePage({ plugin, onBack, active }: { plugin: InstalledPlugin; onBack: () => void; active: boolean }) {
  const request = usePluginRequests(), epoch = useRef(0);
  const [groups, setGroups] = useState<{ id: string; label: string }[]>([]), [group, setGroup] = useState("");
  const [page, setPage] = useState<Page<Channel> | null>(null), [channel, setChannel] = useState<Channel | null>(null);
  const [busy, setBusy] = useState(false), [error, setError] = useState("");
  const logoAssets = page?.items.flatMap(c => c.logo ? [c.logo] : []) || [], logos = useAssetUrls(plugin.meta.id, logoAssets);
  let logoIndex = 0;
  useEffect(() => { void request<{ id: string; label: string }[]>("call", { id: plugin.meta.id, method: "live.getGroups", args: {} }).then(setGroups).catch(e => setError(pluginError(e))); }, []);
  const load = async (cursor?: string) => {
    const n = ++epoch.current; setBusy(true); setError("");
    try {
      const result = await request<Page<Channel>>("call", { id: plugin.meta.id, method: "live.listChannels", args: { ...(group ? { groupId: group } : {}), cursor, limit: 40 } });
      if (n === epoch.current) setPage(old => ({ ...result, items: cursor && old ? [...old.items, ...result.items.filter(c => !old.items.some(o => o.id === c.id))] : result.items }));
    } catch (e) { if (n === epoch.current) setError(pluginError(e)); }
    finally { if (n === epoch.current) setBusy(false); }
  };
  useEffect(() => { setPage(null); void load(); return () => { epoch.current++; }; }, [group]);
  return <BrowseFrame title={`${plugin.meta.name} · 直播`} tools={<button onClick={onBack}>返回</button>}><Notice error={error}/>
    {channel && <><h2>{channel.name}</h2><ContentPlayer key={channel.id} plugin={plugin} action="live.prepare" contentId={channel.id} active={active}/>{plugin.methods.includes("live.getProgrammes") && <Programmes key={channel.id} plugin={plugin} channelId={channel.id}/>}</>}
    <div className="hpp-source-tabs"><button aria-pressed={!group} onClick={() => setGroup("")}>全部频道</button>{groups.map(g => <button key={g.id} aria-pressed={group === g.id} onClick={() => setGroup(g.id)}>{g.label}</button>)}</div>
    <div className="hpp-channel-grid">{page?.items.map(c => <ChannelCard key={c.id} logo={c.logo ? logos[logoIndex++] : undefined} channel={c} selected={c.id === channel?.id} select={() => setChannel(c)}/>)}</div>
    {busy && <p role="status">正在读取频道…</p>}{!busy && page && !page.items.length && <p>暂无频道</p>}{error && <button onClick={() => void load()}>重试频道</button>}
    {page?.next && <button disabled={busy} onClick={() => void load(page.next || undefined)}>更多频道</button>}
  </BrowseFrame>;
}
function ChannelCard({ logo, channel, selected, select }: { logo?: string; channel: Channel; selected: boolean; select: () => void }) {
  return <button className="hpp-channel" aria-pressed={selected} onClick={select}>{logo && <img src={logo} alt="" loading="lazy"/>}<strong>{channel.number !== undefined ? `${channel.number} · ` : ""}{channel.name}</strong>{channel.now && <span>{channel.now}</span>}</button>;
}
function Programmes({ plugin, channelId }: { plugin: InstalledPlugin; channelId: string }) {
  const request = usePluginRequests(), [open, setOpen] = useState(false), [rows, setRows] = useState<Programme[] | null>(null), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const load = async () => {
    setBusy(true); setError(""); const from = Date.now();
    try { setRows(await request<Programme[]>("call", { id: plugin.meta.id, method: "live.getProgrammes", args: { channelId, from, to: from + 24 * 3600_000 } })); }
    catch (e) { setError(pluginError(e)); } finally { setBusy(false); }
  };
  return <section><button aria-expanded={open} onClick={() => {
    setOpen(v => !v); if (!open && !rows && !busy) void load();
  }}>节目单</button>{open && <><Notice error={error}/>{busy && <p role="status">正在读取节目单…</p>}{error && <button disabled={busy} onClick={() => void load()}>重试节目单</button>}{rows?.map((r, i) => <article className="hpp-programme" key={i}><time>{new Date(r.startAt).toLocaleTimeString()} – {new Date(r.endAt).toLocaleTimeString()}</time><strong>{r.title}</strong><p>{r.description}</p></article>)}{rows && !rows.length && <p>暂无节目单</p>}</>}</section>;
}
