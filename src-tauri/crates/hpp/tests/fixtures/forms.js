// Entirely synthetic accounts and challenges. No real service or credentials.
export const meta = {
  id: "fixture.forms", name: "虚构账号与表单", version: "1.0.0", protocol: 1,
  allowInsecureHttp: true, defaultHeaders: { "X-Fixture-Token": "local-fixture" },
};
export default function(hg) {
  const base = "__FIXTURE_BASE__";
  const user = () => ({ id: "synthetic-user", name: "虚构观众", avatar: base + "/cover.svg", stats: { 关注: 3 }, ignored: "drop" });
  return {
    media: {
      getDetail({ id }) { return { id, title: "虚构作品", playlists: [{ id: "main", episodes: [{ id: "one", index: 1 }] }] }; },
      resolvePlayback() { return { streams: [{ url: base + "/clip.mp4", format: "progressive" }] }; },
    },
    auth: {
      getMethods() { return { registerUrl: "https://accounts.example/register", ignored: "drop", methods: [
        { id: "password", label: "虚构账号登录", form: { title: "验证虚构账号", fields: [
          { key: "account", type: "text", label: "账号", required: true },
          { key: "password", type: "password", label: "密码", required: true },
          { key: "challenge", type: "hidden", value: "first" },
        ] } },
        { id: "direct", label: "直接登录虚构账号" },
      ] }; },
      async submit({ methodId, values }) {
        if (methodId === "direct") { await hg.storage.set("session", true); return { status: "ok", user: user() }; }
        if (values.account !== "viewer" || values.password !== "synthetic-password") throw new hg.PluginError("AUTH_INVALID", "虚构凭据不正确");
        if (!values.otp) return { status: "next", form: { title: "完成虚构验证", submitLabel: "完成登录", fields: [
          { key: "challenge", type: "hidden", value: "second" },
          { key: "email", type: "email", label: "邮箱", required: true },
          { key: "number", type: "number", label: "验证数字", required: true },
          { key: "region", type: "select", label: "区域", required: true, options: [{ value: "local", label: "本地" }] },
          { key: "otp", type: "otp", label: "短信码", required: true },
          { key: "captcha", type: "captcha", label: "图形验证码", required: true, image: base + "/cover.svg" },
        ] } };
        if (values.challenge !== "second" || values.otp !== "123456" || values.captcha !== "HGhub" || values.region !== "local" || values.number !== "12.5" || values.email !== "viewer@example.com") throw new hg.PluginError("AUTH_INVALID", "虚构验证未通过");
        await hg.storage.set("session", true);
        return { status: "ok", user: user() };
      },
      async getProfile() { return await hg.storage.get("session") ? user() : null; },
      async logout() {
        try { if (await hg.storage.get("logout-failure")) throw new hg.PluginError("UPSTREAM_ERROR", "虚构退出接口暂不可用"); }
        finally { await hg.storage.delete("session"); }
      },
    },
    library: {
      async listMedia() {
        if (!await hg.storage.get("session")) throw new hg.PluginError("AUTH_REQUIRED", "请先登录虚构账号");
        return { items: [{ id: "cloud-one", title: "云端虚构收藏", type: "short" }], next: null };
      },
    },
    updates: {
      check({items}) { return [...items.map(item => ({mediaId:item.mediaId,episodeCount:7,latestLabel:"更新至第七集"})),{mediaId:"not-requested",latestLabel:"忽略本条"}]; },
    },
    ui: {
      getActions() { return [
        { id: "alias", label: "设置虚构别名", description: "验证多步操作与账号限制", form: { fields: [{ key: "alias", type: "text", label: "别名", required: true, value: "观众" }] } },
        { id: "expire", label: "模拟登录失效", form: { fields: [] } },
        { id: "logout-failure", label: "模拟退出接口失败", form: { fields: [] } },
        { id: "cancel", label: "等待中的操作", form: { fields: [] } },
      ]; },
      async submit({ actionId, values }, ctx) {
        if (actionId === "cancel") { await new Promise(resolve => setTimeout(resolve, 5000)); await hg.storage.set("cancelled-operation", true); return { status: "ok", message: ctx.requestId }; }
        if (actionId === "expire") { await hg.storage.delete("session"); throw new hg.PluginError("AUTH_EXPIRED", "虚构登录已失效"); }
        if (!await hg.storage.get("session")) throw new hg.PluginError("AUTH_REQUIRED", "请先登录虚构账号");
        if (actionId === "logout-failure") { await hg.storage.set("logout-failure", true); return { status: "ok", message: "已启用虚构退出失败" }; }
        if (values.confirm !== "yes") return { status: "next", form: { title: "确认虚构别名", fields: [{ key: "confirm", type: "hidden", value: "yes" }] } };
        await hg.storage.set("alias", values.alias);
        return { status: "ok", message: "虚构别名已保存：" + values.alias };
      },
    },
  };
}
