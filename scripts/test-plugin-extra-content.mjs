import { openRecommendations, selectCategorySource } from "./frontend-navigation.mjs";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

export async function testPluginExtraContent({ page, dispatch, root, base, output, report, rail }) {
  const code = (await readFile(join(root, "src-tauri/crates/hpp/tests/fixtures/extra-content.js"), "utf8")).replaceAll("__FIXTURE_BASE__", base);
  const installed = await dispatch({ action: "import", params: { code } });
  assert(installed.ok && installed.value.status === "ready");
  assert(!installed.value.methods.some(m => m.startsWith("media.")));
  const inspect = async () => {
    const response = await dispatch({ action: "call", params: { id: "fixture.extra", method: "ui.submit", args: { actionId: "inspect", values: {} }, requestId: crypto.randomUUID() } });
    assert(response.ok); return JSON.parse(response.value.message);
  };
  await page.reload();
  await selectCategorySource(page,"虚构直播图文");
  await page.getByRole("button", { name: "直播 · 虚构直播图文", exact: true }).click();
  await page.getByRole("button", { name: "更多频道", exact: true }).click();
  await page.getByRole("button", { name: /虚构频道二/ }).waitFor();
  await page.getByRole("button", { name: "虚构分组", exact: true }).click();
  await page.getByRole("button", { name: /虚构频道一/ }).waitFor();
  await page.getByRole("button", { name: /虚构频道一/ }).click();
  await page.waitForFunction(() => document.querySelector("video")?.currentTime > 0.5);
  const beforeProgramme = await inspect();
  assert.equal(beforeProgramme["last.live.getProgrammes"], undefined);
  assert.equal(beforeProgramme["last.live.listChannels"].groupId, "group:one");
  assert.equal(beforeProgramme["last.live.resolve"].channelId, "channel:one");
  assert.equal(beforeProgramme["last.live.resolve"].episodeId, undefined);
  await page.getByRole("button", { name: "节目单", exact: true }).click();
  await page.getByText("虚构节目单", { exact: true }).waitFor();
  await page.locator(".hpp-content-settings summary").click();
  await page.getByLabel("源播放线路", { exact: true }).selectOption("signal:backup");
  await page.waitForFunction(() => document.querySelector('[aria-label="源播放线路"]')?.value === "signal:backup" && !document.querySelector('[aria-label="源播放线路"]').disabled);
  await page.getByLabel("源播放流", { exact: true }).selectOption("1");
  await page.waitForFunction(() => document.querySelector("video")?.currentTime > 0.5);
  await page.getByRole("button", { name: "重试播放", exact: true }).click();
  await page.waitForFunction(() => document.querySelector("video")?.currentTime > 0.5);
  const live = await inspect();
  assert.equal(live["last.live.resolve"].reason, "refresh");
  assert.equal(live["last.live.resolve"].serverId, "signal:backup");
  assert(live["last.live.getProgrammes"].to > live["last.live.getProgrammes"].from);
  await page.screenshot({ path: join(output, "live-channel.png") });
  await page.getByRole("button", { name: "返回", exact: true }).click();
  assert.equal(await page.locator("video").count(), 0);
  await selectCategorySource(page,"虚构直播图文");
  await page.getByRole("button", { name: "图文 · 虚构直播图文", exact: true }).click();
  await page.getByRole("button", { name: "更多图文", exact: true }).click();
  await page.getByRole("button", { name: /虚构纯文本/ }).waitFor();
  await page.getByRole("button", { name: /虚构图文正文/ }).click();
  await page.getByRole("heading", { name: "清洗后的图文", exact: true }).waitFor();
  await page.waitForFunction(() => document.querySelector(".hpp-article-html img")?.naturalWidth > 0);
  const markup = await page.locator(".hpp-article-html").evaluate(root => ({
    unsafe: root.querySelectorAll("script,iframe,style,form,input,svg,[style],[onclick],[onerror],[onload]").length,
    images: [...root.querySelectorAll("img")].map(i => i.src), executed: window.__ARTICLE_EXECUTED__,
  }));
  assert.equal(markup.unsafe, 0); assert.equal(markup.executed, undefined);
  assert.equal(markup.images.length, 1); assert(markup.images.every(u => new URL(u).hostname === "127.0.0.1" && new URL(u).pathname.startsWith("/hpp/")));
  assert.equal(await page.getByRole("link", { name: "无效链接", exact: true }).count(), 0);
  await page.getByRole("link", { name: "外部参考链接", exact: true }).click();
  assert((await page.evaluate(() => window.__HPP_TEST_OPENED_URLS__)).includes("https://article.example/read"));
  await page.getByRole("button", { name: "云端点赞", exact: true }).click();
  await page.getByRole("button", { name: "取消云端点赞", exact: true }).waitFor();
  await page.getByRole("button", { name: "查看评论", exact: true }).click();
  await page.getByText("图文评论", { exact: true }).waitFor();
  await page.getByRole("button", { name: "播放文内视频", exact: true }).click();
  await page.waitForFunction(() => document.querySelector("video")?.currentTime > 0.5);
  await page.screenshot({ path: join(output, "article-detail.png") });
  const article = await inspect();
  assert.deepEqual(article["last.articles.getDetail"], { id: "article:html" });
  assert.deepEqual(article["last.interact.like"].target, { kind: "article", articleId: "article:html" });
  assert.deepEqual(article["last.comments.list"].target, { kind: "article", articleId: "article:html" });
  await page.getByRole("button", { name: "返回", exact: true }).last().click();
  await page.getByRole("button", { name: /虚构纯文本/ }).click();
  await page.getByText("<b>保留纯文本</b>", { exact: true }).waitFor();
  assert.equal(await page.locator(".hpp-article-text b").count(), 0);
  await rail.getByRole("button", { name: "首页", exact: true }).click();
  report.passed.push("Pure-live source initializes without VOD, home exposes live/articles entries, channel grouping/pagination and HLS playback work, signal/stream selection and refresh retain live identities; programme guide is fetched only on demand");
  report.passed.push("Article pagination, HTML allow-list reconstruction, provider-proxied images, external-only links, literal plain text, embedded playback and article-scoped comments/interactions work without executing or requesting discarded markup");
}
