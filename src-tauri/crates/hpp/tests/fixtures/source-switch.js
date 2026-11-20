// Two independent synthetic sources share a title, never an episode identity.
export const meta = { id: "__PLUGIN_ID__", name: "__PLUGIN_NAME__", version: "1.0.0", protocol: 1, allowInsecureHttp: true, defaultHeaders: { "X-Fixture-Token": "local-fixture" } };
export default function(hg) {
  const base = "__FIXTURE_BASE__", original = meta.id === "fixture.switch-a";
  const item = (id = "same") => ({ id, title: id === "similar" ? "跨源合成作品 · 特别版" : "跨源合成作品", year: id === "conflict" ? 2025 : 2026, type: "short" });
  const episode = (id, index, season, episodeNumber) => ({ id, index, ...(season === undefined ? {} : { season }), ...(episodeNumber === undefined ? {} : { episodeNumber }), label: id });
  return {
    home: { getSections() { return { items: [{ id: "switch", title: meta.name, layout: "row", items: [item()] }] }; } },
    search: { async search(args) {
      await hg.storage.set("query", args);
      if (meta.id === "fixture.switch-fail") throw new hg.PluginError("UPSTREAM_CHANGED", "虚构换源搜索失败");
      return { items: original ? [item()] : [item("similar"), item("conflict"), item(), item("ambiguous"), item("paged"), item("index-only")] };
    } },
    media: {
      async getDetail({ id }) {
        await hg.storage.set("detail", id);
        const episodes = original ? [episode("a:actual", 1, 2, 1)]
          : id === "index-only" ? [episode("b:index", 1)]
          : id === "ambiguous" ? [episode("b:ambiguous:one", 1, 2, 1), episode("b:ambiguous:two", 2, 2, 1)]
          : id === "paged" ? [] : [episode("b:actual", 41, 2, 1), episode("b:other", 1, 1, 1)];
        return { ...item(id), playlists: [{ id: original ? "a:list" : "b:list", label: original ? "原始正片" : "候选正片", episodes, ...(id === "paged" ? { next: "opaque:episodes" } : {}) }] };
      },
      async listEpisodes(args) { await hg.storage.set("episodes", args); return { items: [episode("b:paged:actual", 88, 2, 1)], next: null }; },
      async resolvePlayback(args) {
        const ids = original ? ["a:actual"] : ["b:actual", "b:other", "b:ambiguous:one", "b:ambiguous:two", "b:paged:actual", "b:index"];
        if (!ids.includes(args.episodeId) || args.playlistId !== (original ? "a:list" : "b:list")) throw new Error("Cross-source ID leaked into playback");
        await hg.storage.set("playback", args);
        return { streams: [{ url: base + "/clip.mp4", format: "progressive" }] };
      },
    },
    ui: {
      getActions() { return [{ id: "inspect", label: "Inspect", form: { fields: [] } }]; },
      async submit() { return { status: "ok", message: JSON.stringify({ query: await hg.storage.get("query"), playback: await hg.storage.get("playback"), episodes: await hg.storage.get("episodes") }) }; },
    },
  };
}
