import { useState } from "react";
import { pluginCommand, pluginError, type ResolverRoute } from "./client";

/** Only the host-derived hostname is shown and saved; playback page URLs stay in the host. */
export default function ResolverChooser({ route, busy, select }: {
  route: ResolverRoute; busy: boolean; select: (id: string, host: string) => Promise<ResolverRoute | undefined>;
}) {
  const [remember, setRemember] = useState(false), [saving, setSaving] = useState(false), [message, setMessage] = useState("");
  if (route.forced) return null;
  const choose = async (id: string) => {
    setSaving(true); setMessage("");
    try {
      const result = await select(id, route.host);
      if (remember && result?.host === route.host && result.selectedId === id && !result.forced) {
        await pluginCommand("resolver.preferences.set", { host: result.host, pluginId: id });
        setMessage("已记住这个站点的解析器");
      }
    } catch (error) { setMessage(`无法保存解析器偏好：${pluginError(error)}`); }
    finally { setSaving(false); }
  };
  return <section className="hpp-resolver-choice" aria-label="解析器选择"><h3>解析器 · {route.host}</h3>
    {route.resolvers.length ? <><label className="hpp-check"><input type="checkbox" checked={remember} disabled={busy || saving} onChange={e => setRemember(e.target.checked)}/>记住这个站点的选择</label>
      <div className="hpp-source-tabs">{route.resolvers.map(resolver => <button key={resolver.id} disabled={busy || saving} className={resolver.id === route.selectedId ? "selected" : ""} onClick={() => void choose(resolver.id)}>使用解析器 · {resolver.name}</button>)}</div></>
      : <p className="hpp-muted">该来源需要安装解析器。请前往插件中心导入。</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
