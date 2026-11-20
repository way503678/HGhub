import { IPlayF } from "../icons";
import { useEffect, useState } from "react";
import type { ImageRef, MediaItem } from "./protocol";
import { pluginCommand, releaseMedia } from "./client";

export function useAssetUrls(pluginId: string, assets: ImageRef[]) {
  const [urls,setUrls]=useState<string[]>([]);
  const encoded=JSON.stringify(assets);
  useEffect(()=>{
    let disposed=false;const sessions:string[]=[];setUrls([]);
    if(assets.length) void (async()=>{
      const result:string[]=[];
      for(let start=0;start<assets.length;start+=200){
        const r=await pluginCommand<{sessionId:string;urls:string[]}>("assets.prepare",{id:pluginId,assets:assets.slice(start,start+200)});
        if(disposed){releaseMedia(r.sessionId);return;}sessions.push(r.sessionId);result.push(...r.urls);
      }
      if(!disposed)setUrls(result);
    })().catch(()=>{for(const id of sessions)releaseMedia(id);});
    return ()=>{disposed=true;for(const id of sessions)releaseMedia(id);};
  },[pluginId,encoded]);return urls;
}
export default function MediaCards({ pluginId, items, open, landscape=false }: {pluginId:string;items:MediaItem[];open:(item:MediaItem)=>void;landscape?:boolean}) {
  const assets=items.flatMap(i=>(landscape?i.backdrop||i.cover:i.cover)?[landscape?i.backdrop||i.cover!:i.cover!]:[]);
  const urls=useAssetUrls(pluginId,assets);let n=0;
  return <div className={`hpp-cards ${landscape?"hpp-landscape":""}`}>{items.map((item,index)=>{
    const cover=(landscape?item.backdrop||item.cover:item.cover)?urls[n++]:undefined;
    return <button className="hpp-media-card" key={`${item.id}-${index}`} onClick={()=>open(item)}><div className="hpp-poster">{cover?<img src={cover} alt="" loading="lazy"/>:<span>{item.title.slice(0,2)}</span>}{item.badge&&<small>{item.badge}</small>}</div><strong>{item.title}</strong><p>{[item.year,item.type,item.episodeCount?`${item.episodeCount} 集`:null].filter(Boolean).join(" · ")}</p></button>;
  })}</div>;
}

export function RowCard({ v, onOpen, cover }: { v: MediaItem; onOpen: () => void; cover?: string }) {
  return (
    <button onClick={onOpen} className="legacy-row-card w-[104px] shrink-0 text-left snap-start pressable group">
      <div className="relative w-full rounded-(--r-sm) overflow-hidden bg-s1 elev-1" style={{ paddingTop: "133%" }}>
        {cover ? <img src={cover} alt="" loading="lazy" className="absolute inset-0 w-full h-full object-cover"/> : <span className="absolute inset-0 grid place-items-center text-mut">{v.title.slice(0,2)}</span>}
        <div className="absolute inset-x-0 bottom-0 h-1/2 bg-gradient-to-t from-black/75 to-transparent" />
        <span className="absolute top-1.5 left-1.5 px-1.5 py-0.5 rounded-md text-[8.5px] font-bold brand-grad">
          {v.badge || (v.status === "finished" ? "完结" : v.episodeCount ? `${v.episodeCount}集` : v.type || "")}
        </span>
        <span className="absolute bottom-1.5 right-1.5 flex items-center gap-0.5 text-[9px] font-semibold text-white/90 tsh"><IPlayF size={9} />{v.stats?.plays ?? ""}</span>
      </div>
      <div className="mt-1.5 text-[11.5px] font-medium text-on clamp-1 leading-tight">{v.title}</div>
      <div className="text-[10px] text-mut clamp-1 mt-0.5">{v.tags?.slice(0, 2).join(" · ")}</div>
    </button>
  );
}


export function MediaRow({pluginId,items,open}:{pluginId:string;items:MediaItem[];open:(item:MediaItem)=>void}) {const urls=useAssetUrls(pluginId,items.flatMap(i=>i.cover?[i.cover]:[]));let index=0;return <>{items.map(item=><RowCard key={item.id} v={item} cover={item.cover?urls[index++]:undefined} onOpen={()=>open(item)}/>)}</>;}
