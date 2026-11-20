// Synthetic test fixture only. No real content source or credentials.
export const meta = {
  id: "fixture.source", name: "虚构影视源", version: "1.0.0", protocol: 1,
  allowInsecureHttp: true,
  settings: [{key:"label",type:"text",label:"展示名称",default:"虚构作品"}]
};
export default function(hg) {
  const shared = Promise.resolve("shared");
  return {
    media: {
      getDetail({id}) {
        return {id,title:hg.settings.get("label"),playlists:[{id:"main",episodes:[{id:"ep1",index:1,season:1,episodeNumber:1}]}]};
      },
      resolvePlayback() {return {streams:[{url:"https://media.example/movie.mp4",format:"progressive"}]};}
    },
    search: {
      async search({query},ctx) {
        if(query==="retry-hint") throw new hg.PluginError("RATE_LIMITED","请稍后再试",{retryable:true,retryAfterSec:3});
        if(query==="shared") {
          await shared;
          await shared.then(async () => { await new Promise(r=>setTimeout(r,5)); await hg.storage.set(ctx.requestId,ctx.requestId); });
          query=await hg.storage.get(ctx.requestId);
        }
        if(query==="late") { setTimeout(async()=>{await hg.storage.set("late-write",true);},100); }
        if(query==="late-check") query=String(await hg.storage.get("late-write"));
        if(query==="spin") {while(true) {}}
        if(query==="slow") await new Promise(r=>setTimeout(r,200));
        if(query==="fast") await new Promise(r=>setTimeout(r,10));
        if(query==="bytes") {
          await hg.storage.set("bytes",new Uint8Array([0,1,255]));
          const data=await hg.storage.get("bytes");
          query=hg.codec.hexEncode(data);
        }
        if(query==="crypto") {
          const p={mode:"cbc",key:hg.codec.utf8Encode("0123456789abcdef"),iv:hg.codec.utf8Encode("fedcba9876543210")};
          const cipher=hg.crypto.aesEncrypt(p,hg.codec.utf8Encode("hello"));
          query=hg.codec.utf8Decode(hg.crypto.aesDecrypt(p,cipher));
        }
        if(query==="html") {
          const doc=hg.html.parse('<main><article data-id="a"><h2>测试 标题</h2><a href="/play">播放</a></article></main>');
          query=doc.first("main:has(article) h2").text();
        }
        if(query==="scope") {
          await Promise.resolve();
          await new Promise(r=>setTimeout(r,5));
          await hg.storage.set(ctx.requestId,"owned");
          query=(await hg.storage.get(ctx.requestId))+":"+ctx.requestId;
        }
        return {items:[{id:"one",title:query,unknown:"discard"},{id:1,title:"invalid item"}],next:null};
      }
    }
  };
}
