// Synthetic fixture. The integration runner substitutes its loopback server URL.
export const meta = {
  id:"fixture.local",name:"本地虚构影院",version:"1.0.0",protocol:1,allowInsecureHttp:true,
  description:"仅含本地生成的测试画面，用于验证 HGhub 插件协议。",
  defaultHeaders:{"X-Fixture-Token":"local-fixture"},
  settings:[{key:"label",type:"text",label:"展示名称",default:"本地虚构影院"}]
};
export default function(hg) {
  const base="__FIXTURE_BASE__";
  return {
    endpoints:{list(){return [{id:"local",label:"本机测试线路",url:base}];}},
    home:{getSections(){return {items:[{id:"recommended",title:"虚构推荐",layout:"row",items:[{id:"one",title:"合成影像 · HGhub",type:"short",year:2026,cover:base+"/cover.svg"}],more:true}],next:null};},listSection(){return {items:[{id:"one",title:"合成影像 · HGhub",type:"short"}],next:null};}},
    catalog:{getCategories(){return [{id:"shorts",label:"虚构短剧",filters:[{id:"year",label:"年份",options:[{id:"2026",label:"2026"}]}],sorts:[{id:"latest",label:"最新"}]}];},list({filters,sort}){return {items:[{id:"one",title:filters?.year?.[0]?"筛选后的虚构短剧":"分类中的虚构短剧",badge:sort==="latest"?"最新":undefined}],next:null};}},
    feed:{async list(){return (await hg.http.fetch({url:base+"/search",headers:{"X-Fixture-Token":"local-fixture"},responseType:"json"})).data;}},
    search:{async search({query}){if(query==="fail")throw new hg.PluginError("UPSTREAM_ERROR","虚构源暂时不可用");return (await hg.http.fetch({url:base+"/search",query:{query},headers:{"X-Fixture-Token":"local-fixture"},responseType:"json"})).data;}},
    media:{
      getDetail({id}){return {id,title:hg.settings.get("label"),description:"使用真实 QuickJS 运行时、HTTP 与媒体代理播放的本地合成画面。",playlists:[{id:"main",label:"正片",episodes:[{id:"progressive",index:1,season:1,episodeNumber:1,label:"普通视频"},{id:"hls",index:2,season:1,episodeNumber:2,label:"HLS 清单"},{id:"page",index:3,season:1,episodeNumber:3,label:"解析器播放"},{id:"unsupported",index:4,season:1,episodeNumber:4,label:"备用流测试"}]}]};},
      async resolvePlayback({episodeId,serverId,reason}){
        if(serverId&&!["source-main","source-backup"].includes(serverId))throw new Error("解析器线路误传给源");
        await hg.storage.set("lastPlayback",{episodeId,serverId,reason});
        if(episodeId==="unsupported")return {streams:[{url:base+"/unsupported.mpd",format:"dash",label:"不可用格式"},{url:base+"/clip.mp4",format:"progressive",label:"可用备用流"}]};
        if(episodeId==="page")return {streams:[{kind:"page",url:base+"/watch/one",serverId:serverId||"source-main"}],servers:[{id:"source-main",label:"源主线"},{id:"source-backup",label:"源备用"}]};
        const url=base+(episodeId==="hls"?"/hls/list.m3u8":"/clip.mp4"),format=episodeId==="hls"?"hls":"progressive";
        return {servers:[{id:"source-main",label:"源主线"},{id:"source-backup",label:"源备用"}],streams:[
          {url,format,serverId:serverId||"source-main",label:"720p",height:720,codec:"h264"},
          {url,format,serverId:serverId||"source-main",label:"1080p",height:1080,codec:"h264"}
        ]};
      }
    }
  };
}
