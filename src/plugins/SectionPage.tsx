import { BrowseFrame, Notice, type OpenMedia } from "./BrowsePages";
import type { InstalledPlugin } from "./client";
import MediaCards from "./MediaCards";
import type { MediaItem } from "./protocol";
import { PageLoadMore, usePagedPlugin } from "./usePagedPlugin";

export type OpenSection = (pluginId: string, sectionId: string, title?: string) => void;
export default function SectionPage({ plugin, sectionId, title, open, onBack }: {
  plugin: InstalledPlugin; sectionId: string; title?: string; open: OpenMedia; onBack: () => void;
}) {
  const list = usePagedPlugin<MediaItem>({ plugin, method: "home.listSection", args: { sectionId } });
  return <BrowseFrame title={title || "分区列表"} tools={<button onClick={onBack}>返回</button>}><p className="hpp-muted">{plugin.meta.name}</p><Notice error={list.error || list.warning}/>
    {list.busy && !list.page ? <div className="hpp-skeleton"/> : <MediaCards pluginId={plugin.meta.id} items={list.page?.items || []} open={item => open(plugin.meta.id, item.id)}/>}
    {list.error && <button disabled={list.busy} onClick={list.retry}>重试分区</button>}
    {list.page && <PageLoadMore next={list.page.next} busy={list.busy} error={list.error} more={list.more}/>}
  </BrowseFrame>;
}
