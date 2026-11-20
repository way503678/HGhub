// Pure-live source plus articles: no VOD namespace and no external content requests.
export const meta = { id: "fixture.extra", name: "虚构直播图文", version: "1.0.0", protocol: 1, allowInsecureHttp: true, defaultHeaders: { "X-Fixture-Token": "local-fixture" } };
export default function(hg) {
  const base = "__FIXTURE_BASE__";
  const remember = (method, args) => hg.storage.set("last." + method, args);
  return {
    live: {
      getGroups() { return [{ id: "group:one", label: "虚构分组" }]; },
      async listChannels(args) {
        await remember("live.listChannels", args);
        return { items: [{ id: args.cursor ? "channel:two" : "channel:one", name: args.cursor ? "虚构频道二" : "虚构频道一", logo: base + "/cover.svg", now: "本机合成画面", groupId: "group:one", number: args.cursor ? 2 : 1 }], next: args.cursor ? null : "channels:next" };
      },
      async resolve(args) {
        if (args.channelId !== "channel:one" && args.channelId !== "channel:two") throw new hg.PluginError("NOT_FOUND", "未知虚构频道");
        if (args.mediaId || args.episodeId) throw new Error("VOD identity leaked into live");
        await remember("live.resolve", args);
        return { streams: [{ url: base + "/hls/list.m3u8", format: "hls", live: true, label: "虚构直播" }, { url: base + "/clip.mp4", format: "progressive", live: true, label: "虚构备用直播" }], servers: [{ id: "signal:main", label: "虚构主信号" }, { id: "signal:backup", label: "虚构备用信号" }] };
      },
      async getProgrammes(args) {
        await remember("live.getProgrammes", args);
        return [{ title: "虚构节目单", startAt: args.from, endAt: args.from + 3600_000, description: "仅在打开时读取" }];
      },
    },
    articles: {
      list({ cursor }) { return { items: [{ id: cursor ? "article:text" : "article:html", title: cursor ? "虚构纯文本" : "虚构图文正文", summary: "协议图文展示", cover: base + "/cover.svg" }], next: cursor ? null : "articles:next" }; },
      async getDetail(args) {
        await remember("articles.getDetail", args);
        return { id: args.id, title: args.id === "article:text" ? "虚构纯文本" : "虚构图文正文", contentText: "<b>保留纯文本</b>",
          ...(args.id === "article:html" ? { contentHtml: `<h2>清洗后的图文</h2><p style="color:red" onclick="window.__ARTICLE_EXECUTED__=true">正常段落 <strong>粗体正文</strong></p><script>window.__ARTICLE_EXECUTED__=true</script><iframe src="https://discard.example/frame"></iframe><style>body{display:none}</style><form><input value="should-disappear"></form><img src="${base}/cover.svg" onerror="window.__ARTICLE_EXECUTED__=true" alt="正文配图"><img src="data:image/svg+xml,invalid" onload="alert(1)"><a href="javascript:alert(1)">无效链接</a><a href="https://article.example/read">外部参考链接</a><svg><script>window.__ARTICLE_EXECUTED__=true</script></svg>`, streams: [{ url: base + "/clip.mp4", format: "progressive" }], interaction: { liked: false } } : {}),
        };
      },
    },
    comments: { async list(args) { await remember("comments.list", args); return { items: [{ id: "article-comment", author: { name: "图文读者" }, text: "图文评论" }] }; } },
    interact: { async like(args) { await remember("interact.like", args); } },
    ui: { getActions() { return [{ id: "inspect", label: "检查虚构参数", form: { fields: [] } }]; }, async submit() { const keys = await hg.storage.keys("last."); const result = {}; for (const key of keys) result[key] = await hg.storage.get(key); return { status: "ok", message: JSON.stringify(result) }; } },
  };
}
