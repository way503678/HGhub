// Original paged content for viewport, ordering and query-cancellation regressions.
export const meta = { id: "__PLUGIN_ID__", name: "__PLUGIN_NAME__", version: "1.0.0", protocol: 1 };
export default function(hg) {
  const remember = (method, args) => hg.storage.set("call:" + hg.crypto.randomUUID(), { method, args });
  const items = (prefix, start, count) => Array.from({ length: count }, (_, n) => ({ id: prefix + ":" + (start + n), title: meta.name + " · " + prefix + " · " + (start + n), type: "short", year: 2026 }));
  const page = async (method, args, prefix) => {
    await remember(method, args);
    if (!args.cursor) return { items: items(prefix, 0, 18), next: "opaque:second" };
    if (args.cursor === "opaque:second") {
      const failKey = "failed:" + method + ":" + prefix;
      if (!await hg.storage.get(failKey)) { await hg.storage.set(failKey, true); throw new hg.PluginError("NETWORK", "虚构下一页失败"); }
      return { items: [...items(prefix, 17, 1), ...items(prefix, 18, 4)], next: "opaque:third" };
    }
    return { items: [...items(prefix, 21, 1), ...items(prefix, 22, 1)], next: "opaque:third" };
  };
  return {
    home: {
      async getSections(args) {
        await remember("home.getSections", args);
        if (args.cursor) return { items: [{ id: "more", title: "分页更多分区", layout: "grid", items: items("更多", 0, 1) }], next: null };
        if (await hg.storage.get("failHome")) throw new hg.PluginError("NETWORK", "虚构推荐刷新失败");
        return { items: [{ id: "recommended", title: "分页推荐", layout: "grid", items: items("推荐", 0, 18), more: true }, { id: "latest", title: "分页更新", layout: "row", items: items("更新", 0, 1) }], next: "home:next" };
      },
      listSection(args) { return page("home.listSection", args, "分区"); },
    },
    catalog: {
      async getCategories() { await remember("catalog.getCategories", {}); return [{ id: "short", label: "分页短剧", filters: [{ id: "year", label: "年份", options: [{ id: "2026", label: "2026" }] }], sorts: [{ id: "slow", label: "慢速排序" }, { id: "fast", label: "快速排序" }] }]; },
      async list(args) { if (args.sort === "slow") await new Promise(resolve => setTimeout(resolve, 600)); return page("catalog.list", args, "分类" + (args.sort || "默认") + (args.filters?.year?.[0] || "")); },
    },
    search: {
      async search(args) {
        if (args.query === "slow") await new Promise(resolve => setTimeout(resolve, 600));
        if (args.query === "partial" && meta.id.endsWith("-b")) throw new hg.PluginError("NETWORK", "虚构单源搜索失败");
        return page("search.search", args, "搜索" + args.query);
      },
    },
    media: {
      getDetail({ id }) { return { id, title: meta.name, playlists: [{ id: "main", episodes: [{ id: "real", index: 1 }] }] }; },
      resolvePlayback() { return { streams: [{ url: "https://media.example/fixture.mp4", format: "progressive" }] }; },
    },
    ui: {
      getActions() { return [{ id: "inspect", label: "Inspect", form: { fields: [] } }, { id: "failHome", label: "Fail", form: { fields: [] } }]; },
      async submit({ actionId }) { if (actionId === "failHome") await hg.storage.set("failHome", true); const keys = await hg.storage.keys("call:"); return { status: "ok", message: JSON.stringify(await Promise.all(keys.map(key => hg.storage.get(key)))) }; },
    },
  };
}
