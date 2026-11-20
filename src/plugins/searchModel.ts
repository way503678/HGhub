import type { MediaItem } from "./protocol";
export interface SearchHit { pluginId: string; item: MediaItem }
export function normalizedTitle(title: string) { return title.normalize("NFKC").replace(/[\p{P}\p{Z}\s]/gu, "").toLowerCase(); }
export function mergeSearch(hits: SearchHit[]) {
  const groups: SearchHit[][]=[];
  for(const hit of hits){
    const title=normalizedTitle(hit.item.title);
    const group=groups.find(g=>normalizedTitle(g[0].item.title)===title&&g.every(h=>(h.item.year===undefined||hit.item.year===undefined||h.item.year===hit.item.year)&&(h.item.type===undefined||hit.item.type===undefined||h.item.type===hit.item.type)));
    if(group) {if(!group.some(h=>h.pluginId===hit.pluginId&&h.item.id===hit.item.id))group.push(hit);}else groups.push([hit]);
  }return groups;
}
