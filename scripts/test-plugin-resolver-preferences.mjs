import { openRecommendations, selectCategorySource, playerPanel, chooseEpisode } from "./frontend-navigation.mjs";
import assert from "node:assert/strict";
import { join } from "node:path";

export async function testResolverPreferences({ page, dispatch, base, output, report, rail }) {
  const sourceId = "fixture.routing-ui", ids = ["fixture.remember-a", "fixture.remember-b"], names = ["记忆解析器甲", "记忆解析器乙"];
  const source = `export const meta={id:"${sourceId}",name:"站点路由影院",version:"1.0.0",protocol:1};
    export default ()=>({
      home:{getSections(){return {items:[{id:"route",layout:"row",title:"解析器站点测试",items:[{id:"route",title:"站点路由作品"}]}]};}},
      media:{getDetail(){return {id:"route",title:"站点路由作品",playlists:[{id:"main",episodes:[{id:"a",index:1,label:"站点甲"},{id:"b",index:2,label:"站点乙"},{id:"pinned",index:3,label:"指定解析器"}]}]};},
        resolvePlayback(p){return {streams:[{kind:"page",url:"https://playback-"+(p.episodeId==="b"?"b":"a")+".example/private-watch?token=secret-page-token",...(p.episodeId==="pinned"?{resolverId:"${ids[0]}"}:{})}]};}},
      live:{getGroups(){return [{id:"group",label:"路由频道组"}];},listChannels(){return {items:[{id:"channel",name:"站点甲直播"}]};},resolve(){return {streams:[{kind:"page",url:"https://playback-a.example/live-private?token=secret-page-token"}]};}}
    });`;
  assert((await dispatch({ action: "import", params: { code: source } })).ok);
  for (let i = 0; i < ids.length; i++) {
    const code = `export const meta={id:"${ids[i]}",name:"${names[i]}",version:"1.0.0",protocol:1,kind:"resolver",matches:["playback-a.example","playback-b.example"],allowInsecureHttp:true};
      export default ()=>({resolver:{resolve(){return {streams:[{url:"${base}/clip.mp4",format:"progressive",headers:{"X-Fixture-Token":"local-fixture"}}]};}}});`;
    assert((await dispatch({ action: "import", params: { code } })).ok);
  }
  const failingId = "fixture.remember-failure";
  assert((await dispatch({ action: "import", params: { code: `export const meta={id:"${failingId}",name:"记忆解析器失败",version:"1.0.0",protocol:1,kind:"resolver",matches:["playback-a.example"]};export default hg=>({resolver:{resolve(){throw new hg.PluginError("NETWORK","虚构解析失败");}}});` } })).ok);
  const preferences = async () => { const result = await dispatch({ action: "resolver.preferences.get", params: {} }); assert(result.ok); return result.value; };
  const chooser = page.getByRole("region", { name: "解析器选择", exact: true });
  const choose = name => chooser.getByRole("button", { name: `使用解析器 · ${name}`, exact: true });
  const selected = async name => { await choose(name).and(page.locator(".selected")).waitFor(); assert.match(await choose(name).getAttribute("class"), /selected/); };
  const playing = () => page.waitForFunction(() => document.querySelector("video")?.currentTime > 0.3);
  const openDetail = async () => {
    await openRecommendations(page,"站点路由影院");
    await page.getByRole("button", { name: /站点路由作品/ }).click();
    await playerPanel(page, '详情');
    await page.getByRole("heading", { name: "站点路由作品", exact: true }).last().waitFor();
  };
  await page.reload(); await openDetail();
  await chooseEpisode(page, '站点甲');
  await chooser.getByRole("heading", { name: "解析器 · playback-a.example", exact: true }).waitFor();
  assert.equal(await chooser.getByRole("checkbox", { name: "记住这个站点的选择", exact: true }).isChecked(), false);
  await choose(names[0]).click(); await playing();
  assert.deepEqual(await preferences(), {}, "Manual choice is not remembered implicitly");
  await chooser.getByRole("checkbox", { name: "记住这个站点的选择", exact: true }).check();
  await choose(names[1]).click(); await playing();
  await chooser.getByRole("status").filter({ hasText: "已记住" }).waitFor();
  assert.equal((await preferences())["playback-a.example"], ids[1]);
  assert(!(await chooser.textContent()).includes("secret-page-token"));
  await page.screenshot({ path: join(output, "resolver-site-preference.png") });
  await choose("记忆解析器失败").click();
  await page.getByRole("alert").filter({ hasText: "虚构解析失败" }).waitFor();
  assert.equal((await preferences())["playback-a.example"], ids[1], "Failed resolver choice cannot replace a working remembered choice");
  await chooseEpisode(page, '站点乙');
  await chooser.getByRole("heading", { name: "解析器 · playback-b.example", exact: true }).waitFor();
  assert.equal(await page.locator("video").count(), 0, "Prior manual selection must not follow a source to another page host");
  assert.equal(await chooser.getByRole("checkbox", { name: "记住这个站点的选择", exact: true }).isChecked(), false);
  await choose(names[0]).click(); await playing();
  assert.equal((await preferences())["playback-b.example"], undefined);
  await chooseEpisode(page, '站点甲'); await selected(names[1]); await playing();
  await chooseEpisode(page, '指定解析器'); await chooser.waitFor({ state: "hidden" }); await playing();
  assert.equal(await chooser.count(), 0, "Source-pinned resolver cannot be overridden or remembered");
  // Reopening after a full reload uses host persistence rather than the component selection ref.
  await page.reload(); await openDetail();
  await chooseEpisode(page, '站点甲'); await selected(names[1]); await playing();
  // ContentPlayer (live) shares exactly the same selection and remember flow.
  await rail.getByRole("button", { name: "首页", exact: true }).click();
  await selectCategorySource(page,"站点路由影院");
  await page.getByRole("button", { name: "直播 · 站点路由影院", exact: true }).click();
  await page.getByRole("button", { name: /站点甲直播/ }).click(); await page.locator(".hpp-content-settings summary").click(); await selected(names[1]); await playing();
  await chooser.getByRole("checkbox", { name: "记住这个站点的选择", exact: true }).check();
  await choose(names[0]).click(); await playing();
  await chooser.getByRole("status").filter({ hasText: "已记住" }).waitFor();
  assert.equal((await preferences())["playback-a.example"], ids[0]);
  assert((await dispatch({ action: "resolver.preferences.set", params: { host: "playback-b.example", pluginId: ids[1] } })).ok);
  await rail.getByRole("button", { name: "设置", exact: true }).click();
  const settings = page.locator('.hpp-resolver-preferences');
  await settings.locator('summary').click();
  await settings.getByRole('button', {name:'忘记 playback-a.example 的解析器',exact:true}).waitFor();
  await settings.getByRole("button", { name: "忘记 playback-a.example 的解析器", exact: true }).click();
  await page.waitForFunction(() => !document.querySelector('.hpp-resolver-preferences')?.textContent.includes("playback-a.example"));
  assert.deepEqual(await preferences(), { "playback-b.example": ids[1] });
  await page.screenshot({ path: join(output, "resolver-preference-settings.png") });
  await settings.getByRole("button", { name: "清空解析器偏好", exact: true }).click();
  await settings.getByText(/暂无记住的站点/).waitFor(); assert.deepEqual(await preferences(), {});
  await page.reload(); await rail.getByRole("button", { name: "设置", exact: true }).click();
  await settings.locator('summary').click();
  await settings.getByText(/暂无记住的站点/).waitFor();
  for (const id of [sourceId, ...ids, failingId]) assert((await dispatch({ action: "remove", params: { id } })).ok);
  await page.reload();
  report.passed.push("Resolver selection is host scoped, defaults to unremembered, persists only after explicit successful choice, survives reload, ignores a prior site's manual selection, honors a source-pinned resolver, works in VOD and live players, and supports individual/all preference removal in settings without exposing page paths or tokens");
}
