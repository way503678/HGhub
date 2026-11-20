// Synthetic interaction service. It never contacts a real upstream.
export const meta = { id: "fixture.social", name: "虚构评论影院", version: "1.0.0", protocol: 1, allowInsecureHttp: true, defaultHeaders: { "X-Fixture-Token": "local-fixture" } };
export default function(hg) {
  const base = "__FIXTURE_BASE__";
  const profile = { id: "viewer", name: "互动观众" };
  const signedIn = async () => { if (!await hg.storage.get("session")) throw new hg.PluginError("AUTH_REQUIRED", "请登录虚构互动账号"); };
  const record = async (method, args) => hg.storage.set("event." + hg.crypto.randomUUID(), { method, args });
  const row = (id, text) => ({ id, text, author: { name: "虚构评论者", avatar: base + "/cover.svg" }, likes: 0, liked: false, replyCount: 2 });
  return {
    media: {
      async getDetail({ id }) { return { id, title: "虚构互动作品", author: { id: "creator:opaque", name: "虚构作者" }, interaction: { liked: false, favorited: false }, playlists: [{ id: "main", episodes: [{ id: "ep:one", index: 1, label: "互动第一集" }, { id: "ep:two", index: 2, label: "互动第二集" }] }] }; },
      resolvePlayback() { return { streams: [{ url: base + "/clip.mp4", format: "progressive" }] }; },
    },
    auth: { getMethods() { return { methods: [{ id: "direct", label: "登录互动账号" }] }; }, async submit() { await hg.storage.set("session", true); return { status: "ok", user: profile }; }, async getProfile() { return await hg.storage.get("session") ? profile : null; }, async logout() { await hg.storage.delete("session"); } },
    comments: {
      async list(args) {
        await record("comments.list", args);
        if (args.target.episodeId === "ep:two") throw new hg.PluginError("UNSUPPORTED", "不支持此分集评论");
        if (args.target.kind === "replies") return { items: [row(args.cursor ? "reply-two" : "reply-one", args.cursor ? "分页回复二" : "分页回复一")], next: args.cursor ? null : "reply:next" };
        return { items: [row(args.cursor ? "comment-two" : "comment-one", args.cursor ? "分页评论二" : "<b>评论纯文本</b>")], next: args.cursor ? null : "comment:next" };
      },
      async send(args) {
        await signedIn();
        if (args.text === "发送不支持") throw new hg.PluginError("UNSUPPORTED", "发送不支持");
        if (args.text === "取消中的评论") await new Promise(resolve => setTimeout(resolve, 1500));
        await record("comments.send", args); if (args.text === "无返回评论") return; return row(hg.crypto.randomUUID(), args.text);
      },
      async like(args) { await signedIn(); if (args.commentId === "reply-two") throw new hg.PluginError("UNSUPPORTED", "回复点赞不支持"); await record("comments.like", args); },
    },
    interact: {
      async like(args) { await signedIn(); await record("interact.like", args); },
      async favorite(args) { await signedIn(); await record("interact.favorite", args); },
      async follow(args) { await signedIn(); await record("interact.follow", args); },
    },
    library: {
      async listFolders() { await signedIn(); return [{ id: "folder:opaque", label: "虚构目录", count: 1 }]; },
      async listMedia(args) { await signedIn(); await record("library.listMedia", args); return { items: [{ id: args.folderId || "social:media", title: args.folderId ? "目录内作品" : "虚构互动作品" }] }; },
      async listFollowing(args) { await signedIn(); await record("library.listFollowing", args); return { items: [{ id: args.cursor ? "author:two" : "creator:opaque", name: args.cursor ? "第二位作者" : "虚构作者", avatar: base + "/cover.svg" }], next: args.cursor ? null : "following:next" }; },
    },
    progress: { async report(args) { await record("progress.report", args); await signedIn(); } },
    ui: { getActions() { return [{ id: "events", label: "读取虚构事件", form: { fields: [] } }]; }, async submit() { const keys = await hg.storage.keys("event."); const events = await Promise.all(keys.map(key => hg.storage.get(key))); return { status: "ok", message: JSON.stringify(events) }; } },
  };
}
