import { openRecommendations, selectCategorySource, playerPanel, chooseEpisode } from "./frontend-navigation.mjs";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

export async function testPluginSourceSwitch({ page, dispatch, root, base, output, report, rail }) {
  const template = await readFile(join(root, "src-tauri/crates/hpp/tests/fixtures/source-switch.js"), "utf8");
  const ids = ["fixture.switch-a", "fixture.switch-b", "fixture.switch-fail"];
  for (const [i, id] of ids.entries()) {
    const code = template.replaceAll("__FIXTURE_BASE__", base).replaceAll("__PLUGIN_ID__", id).replaceAll("__PLUGIN_NAME__", ["原始虚构源", "候选虚构源", "失败虚构源"][i]);
    const installed = await dispatch({ action: "import", params: { code } });
    assert(installed.ok && installed.value.status === "ready", JSON.stringify(installed));
  }
  const operation = async (action, params) => { const r = await dispatch({ action, params }); assert(r.ok, JSON.stringify(r)); return r.value; };
  const inspect = async id => JSON.parse((await operation("call", { id, method: "ui.submit", args: { actionId: "inspect", values: {} }, requestId: crypto.randomUUID() })).message);
  const associations = () => operation("library.sources.get", { pluginId: ids[0], mediaId: "same" });
  const pauseAt = async seconds => {
    const saved = page.waitForResponse(r => {
      if (!r.url().endsWith("/__hpp_test_ipc")) return false;
      const p = r.request().postDataJSON(); return p.action === "library.progress.set" && p.params.positionSec >= seconds;
    });
    await page.evaluate(seconds => { const v = document.querySelector("video"); v.currentTime = seconds; v.dispatchEvent(new Event("timeupdate")); v.pause(); }, seconds);
    await saved;
  };
  await page.reload(); await openRecommendations(page,"原始虚构源");
  await page.getByLabel("原始虚构源的推荐").getByRole("button", { name: /跨源合成作品/ }).click();
  await chooseEpisode(page, 'a:actual');
  await page.waitForFunction(() => document.querySelector("video")?.currentTime > 0.5);
  await page.getByRole("button", { name: "本地收藏", exact: true }).click();
  await page.getByRole("button", { name: "取消本地收藏", exact: true }).waitFor();
  await playerPanel(page, '详情');
  await page.getByRole("button", { name: "追更", exact: true }).click();
  await pauseAt(2);
  await playerPanel(page, '播放设置'); await page.getByRole('button', { name: '换源', exact: true }).click();
  let dialog = page.getByRole("dialog", { name: "换源 · 跨源合成作品", exact: true });
  await dialog.getByRole("alert").filter({ hasText: "虚构换源搜索失败" }).waitFor();
  await dialog.locator(".hpp-source-candidates button").filter({ hasText: "候选虚构源" }).first().waitFor();
  const ranked = await dialog.locator(".hpp-source-candidates button").allTextContents();
  assert(ranked[0].includes("标题一致"), "Exact compatible titles rank first");
  assert(ranked.at(-1).includes("相似候选"), "Approximate title ranks after exact candidates");
  assert.equal((await inspect(ids[1])).query.query, "跨源合成作品");
  assert.equal((await associations()).length, 0, "Search must not establish associations");
  await dialog.getByRole("button", { name: "取消换源", exact: true }).click();
  assert.equal((await associations()).length, 0);
  const choose = async id => {
    await playerPanel(page, '播放设置'); await page.getByRole('button', { name: '换源', exact: true }).click();
    dialog = page.getByRole("dialog", { name: "换源 · 跨源合成作品", exact: true });
    // Matching candidate IDs are opaque; use their stable sequence through the fixture's labels.
    const candidate = dialog.locator(".hpp-source-candidates button").filter({ hasText: "候选虚构源" });
    await candidate.first().waitFor();
    const index = { same: 0, ambiguous: 1, paged: 2, "index-only": 3, conflict: 4, similar: 5 }[id];
    await candidate.nth(index).click();
    await dialog.getByLabel("新来源分集", { exact: true }).waitFor();
  };
  await choose("same");
  assert.equal(await dialog.getByLabel("新来源分集", { exact: true }).inputValue(), JSON.stringify(["b:list", "b:actual"]));
  const carry = dialog.getByRole("checkbox", { name: /沿用原来源播放位置/ });
  await carry.waitFor(); assert.equal(await carry.isChecked(), false);
  await dialog.getByRole("button", { name: "确认换源并播放", exact: true }).click();
  await page.waitForFunction(() => document.querySelector("video")?.currentTime > 0 && document.querySelector("video")?.currentTime < 1.5);
  assert.equal((await inspect(ids[1])).playback.episodeId, "b:actual");
  assert.equal((await associations()).length, 1);
  const original = await operation("library.get", { pluginId: ids[0], mediaId: "same" });
  assert(original.favorite && original.tracking && original.lastProgress.positionSec >= 2);
  await pauseAt(1);
  await playerPanel(page, '播放设置'); await page.getByRole('button', { name: '换源', exact: true }).click();
  dialog = page.getByRole("dialog", { name: "换源 · 跨源合成作品", exact: true });
  await dialog.locator(".hpp-source-candidates button").filter({ hasText: "原始虚构源" }).click();
  await dialog.getByRole("status").filter({ hasText: "优先使用新来源进度" }).waitFor();
  assert.equal(await dialog.getByRole("checkbox", { name: /沿用原来源播放位置/ }).count(), 0);
  await dialog.getByRole("button", { name: "确认换源并播放", exact: true }).click();
  await page.waitForFunction(() => document.querySelector("video")?.currentTime >= 2);
  await pauseAt(2.5);
  // No old-position inference from an ambiguous season/episode match.
  await choose("ambiguous");
  assert.equal(await dialog.getByLabel("新来源分集", { exact: true }).inputValue(), "");
  await dialog.getByLabel("新来源分集", { exact: true }).selectOption(JSON.stringify(["b:list", "b:ambiguous:two"]));
  await dialog.getByRole("checkbox", { name: /沿用原来源播放位置/ }).check();
  await page.screenshot({ path: join(output, "source-switch-confirmation.png") });
  const oldSource = await page.locator("video").evaluate(v => v.currentSrc);
  await dialog.getByRole("button", { name: "确认换源并播放", exact: true }).click();
  await dialog.waitFor({ state: "hidden" });
  await page.waitForFunction(old => { const v = document.querySelector("video"); return v?.currentSrc && v.currentSrc !== old && v.currentTime >= 2.5; }, oldSource);
  assert.equal((await inspect(ids[1])).playback.episodeId, "b:ambiguous:two");
  // Return to original via the local library and select a paged candidate.
  await rail.getByRole("button", { name: "我的", exact: true }).click();
  const originalCard = page.locator(".hpp-local-entry").filter({ hasText: "原始虚构源" });
  await originalCard.locator("summary").click();
  await originalCard.getByRole("button", { name: "关联来源", exact: true }).click();
  await originalCard.getByRole("button", { name: /候选虚构源/ }).first().waitFor();
  await originalCard.getByRole("button", { name: "查看作品", exact: true }).click();
  await chooseEpisode(page, 'a:actual');
  await page.waitForFunction(() => document.querySelector("video")?.currentTime >= 2.5);
  await page.evaluate(() => document.querySelector("video").pause());
  await choose("paged");
  assert.equal(await dialog.getByLabel("新来源分集", { exact: true }).inputValue(), "");
  await dialog.getByRole("button", { name: "加载候选分集 · 候选正片", exact: true }).click();
  await dialog.getByRole("option", { name: /b:paged:actual/ }).waitFor({ state: "attached" });
  await dialog.getByLabel("新来源分集", { exact: true }).selectOption(JSON.stringify(["b:list", "b:paged:actual"]));
  await dialog.getByRole("checkbox", { name: /沿用原来源播放位置/ }).waitFor();
  const oldPagedSource = await page.locator("video").evaluate(v => v.currentSrc);
  await dialog.getByRole("button", { name: "确认换源并播放", exact: true }).click();
  await dialog.waitFor({ state: "hidden" });
  await page.waitForFunction(old => { const v = document.querySelector("video"); return v?.currentSrc && v.currentSrc !== old && v.currentTime > 0.2; }, oldPagedSource);
  assert.equal((await inspect(ids[1])).playback.episodeId, "b:paged:actual");
  const rules = await page.evaluate(async () => {
    const { matchedSourceEpisode } = await import("/src/plugins/sourceMatching.ts");
    const previous = { id: "old", index: 1, episodeNumber: 1 };
    const detail = episodes => ({ id: "new", title: "New", playlists: [{ id: "p", episodes }] });
    return {
      noSeason: matchedSourceEpisode(previous, detail([{ id: "new", index: 99, episodeNumber: 1 }]))?.episode.id,
      indexOnly: !matchedSourceEpisode({ id: "old", index: 1 }, detail([{ id: "new", index: 1 }])),
      conflictingSeason: !matchedSourceEpisode(previous, detail([{ id: "new", index: 1, season: 1, episodeNumber: 1 }])),
      incomplete: !matchedSourceEpisode(previous, { id: "new", title: "New", playlists: [{ id: "p", next: "more", episodes: [{ id: "new", index: 1, episodeNumber: 1 }] }] }),
    };
  });
  assert.deepEqual(rules, { noSeason: "new", indexOnly: true, conflictingSeason: true, incomplete: true });
  await rail.getByRole("button", { name: "首页", exact: true }).click();
  for (const id of ids) await operation("remove", { id });
  assert((await associations()).length >= 2, "Uninstall retains local source associations");
  await page.reload();
  report.passed.push("Cross-source search ranks exact compatible titles, isolates failures, requires explicit confirmation, persists associations without moving favorites/tracking/progress, uses target progress first, carries seconds only when checked, prompts for ambiguous/index-only identities and resolves manually selected paged episodes");
}
