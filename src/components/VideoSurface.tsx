import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import Hls from "hls.js";
import workerPath from 'hls.js/dist/hls.worker.js?url';
import { hlsPlaybackPath } from '../plugins/mediaCapabilities';
import { playbackTiming } from '../plugins/playbackTiming';
import { pluginCommand, type PreparedMedia } from "../plugins/client";

export interface VideoHandle {
  el: () => HTMLVideoElement | null;
  play: () => void; pause: () => void; toggle: () => void;
  seek: (time: number, source?: string) => void;
  rate: (rate: number) => void; mute: (muted: boolean) => void; volume: (volume: number) => void;
}
interface Props {
  media: PreparedMedia; position: number; active: boolean; autoPlay?: boolean;
  poster?: string; muted: boolean; volume: number; rate: number; fit: "contain" | "cover";
  scale: number; panX: number; panY: number; isGesturing: boolean;
  onClick: () => void; onTime: (t: number) => void; onDuration: (d: number) => void;
  onSize: (w: number,h: number) => void; onBuffered: (fraction: number) => void;
  onBuffer?: (seconds: number) => void;
  onPlaying: () => void; onPause: () => void; onWaiting: () => void;
  onEnded?: () => void; onError: () => void;
}

/** Same surface/handle as the original player, loading only host-prepared media URLs. */
export default forwardRef<VideoHandle, Props>(function VideoSurface(props, forwarded) {
  const ref=useRef<HTMLVideoElement>(null), latest=useRef(props), hls=useRef<Hls|null>(null);
  const playAttempt=useRef(0), wantsPlay=useRef(props.autoPlay!==false);
  latest.current=props;
  const play=()=>{
    const v=ref.current;
    if(!v || !latest.current.active || document.hidden)return;
    wantsPlay.current=true;
    const attempt=++playAttempt.current, sessionId=latest.current.media.sessionId;
    if(v.paused || v.readyState<3)latest.current.onWaiting();
    void v.play().catch((error:unknown)=>{
      if(attempt!==playAttempt.current || sessionId!==latest.current.media.sessionId || !latest.current.active || document.hidden)return;
      const name=(error as {name?:string})?.name;
      if(name==='AbortError')return; // Source changes and pause interrupt pending play promises.
      if(name==='NotAllowedError'){wantsPlay.current=false;latest.current.onPause();}
      else latest.current.onError();
    });
  };
  const pause=()=>{wantsPlay.current=false;playAttempt.current++;const v=ref.current;if(!v)return;const alreadyPaused=v.paused;v.pause();if(alreadyPaused)latest.current.onPause();};
  useImperativeHandle(forwarded,()=>({
    el:()=>ref.current, play, pause,
    toggle:()=>{const v=ref.current;if(v?.paused)play();else pause();},
    seek:(t)=>{const v=ref.current;if(!v || !Number.isFinite(t) || !Number.isFinite(v.duration))return;const target=Math.max(0,Math.min(v.duration-.15,t));hls.current?.startLoad(target);v.currentTime=target;latest.current.onTime(target);},
    rate:r=>{if(ref.current)ref.current.playbackRate=r;},mute:m=>{if(ref.current)ref.current.muted=m;},volume:n=>{if(ref.current)ref.current.volume=n;},
  }),[]);
  useEffect(()=>{
    const v=ref.current!;let disposed=false,failed=false,resume=false;
    playAttempt.current++;wantsPlay.current=latest.current.autoPlay!==false;
    const started=performance.now(); let firstFrame: number | undefined;
    const p=latest.current;
    const activity=(playing:boolean)=>void pluginCommand("media.activity",{sessionId:p.media.sessionId,playing}).then(()=>{if(!playing)window.dispatchEvent(new Event("hpp.playback.idle"));}).catch(()=>{});
    const fail=()=>{if(!disposed&&!failed){failed=true;latest.current.onError();}};
    const metadata=()=>{if(p.position>0&&Number.isFinite(v.duration))v.currentTime=Math.min(p.position,Math.max(0,v.duration-.15));if(latest.current.active&&wantsPlay.current)play();};
    const playing=()=>{if(!disposed){activity(true);latest.current.onPlaying();}};
    const paused=()=>{if(!disposed&&v.paused){activity(false);latest.current.onTime(v.currentTime);if(!v.ended){if(wantsPlay.current&&latest.current.active&&v.readyState<3)latest.current.onWaiting();else latest.current.onPause();}}};
    const visibility=()=>{if(document.hidden){resume=!v.paused;v.pause();hls.current?.stopLoad();}else if(resume){resume=false;if(latest.current.active){hls.current?.startLoad(v.currentTime);play();}}};
    v.muted=p.muted;v.volume=p.volume;v.playbackRate=p.rate;
    v.addEventListener("loadedmetadata",metadata);v.addEventListener("error",fail);v.addEventListener("playing",playing);v.addEventListener("pause",paused);document.addEventListener("visibilitychange",visibility);
    if(p.media.format==="hls"&&hlsPlaybackPath(v)==="mse"){
      const engine=new Hls({enableWorker:true,workerPath,autoStartLoad:p.active&&!document.hidden,startPosition:Math.max(0,p.position),backBufferLength:30,interstitialsController:undefined,contentSteeringController:undefined,enableInterstitialPlayback:false});
      hls.current=engine;engine.loadSource(p.media.url);engine.attachMedia(v);
      engine.on(Hls.Events.ERROR,(_,data)=>{if(data.fatal)fail();});
    }else v.src=p.media.url;
    playbackTiming(p.media.sessionId,'media-load');
    if(v.requestVideoFrameCallback)firstFrame=v.requestVideoFrameCallback(()=>{if(!disposed){playbackTiming(p.media.sessionId,'first-frame',started);if(p.media.requestedAt!==undefined)playbackTiming(p.media.sessionId,'intent-to-first-frame',p.media.requestedAt);}});
    const expiry=p.media.playback.expiresAt;
    const timer=expiry?window.setTimeout(fail,Math.max(1000,Math.min(2_147_483_647,expiry-Date.now()-5000))):undefined;
    return()=>{disposed=true;playAttempt.current++;if(firstFrame!==undefined)v.cancelVideoFrameCallback(firstFrame);clearTimeout(timer);activity(false);document.removeEventListener("visibilitychange",visibility);v.removeEventListener("loadedmetadata",metadata);v.removeEventListener("error",fail);v.removeEventListener("playing",playing);v.removeEventListener("pause",paused);hls.current?.destroy();hls.current=null;v.pause();v.removeAttribute("src");v.load();};
  },[props.media.sessionId]);
  useEffect(()=>{const v=ref.current;if(v){v.muted=props.muted;v.volume=props.volume;v.playbackRate=props.rate;}},[props.muted,props.volume,props.rate]);
  useEffect(()=>{if(!props.active||document.hidden){ref.current?.pause();hls.current?.stopLoad();}else if(hls.current)hls.current.startLoad(ref.current?.currentTime||props.position);},[props.active,props.media.sessionId]);
  const dimensions=()=>{const v=ref.current!;if(Number.isFinite(v.duration))latest.current.onDuration(v.duration);if(v.videoWidth)latest.current.onSize(v.videoWidth,v.videoHeight);};
  const buffered=(v:HTMLVideoElement)=>{props.onBuffered(v.duration&&v.buffered.length?v.buffered.end(v.buffered.length-1)/v.duration:0);let ahead=0;for(let i=0;i<v.buffered.length;i++)if(v.buffered.start(i)<=v.currentTime+.1&&v.buffered.end(i)>v.currentTime)ahead=v.buffered.end(i)-v.currentTime;props.onBuffer?.(ahead);};
  return <div className="absolute inset-0 overflow-hidden" onClick={props.onClick}>
    <video ref={ref} playsInline preload="auto" autoPlay={props.autoPlay && props.active} crossOrigin="anonymous" poster={props.poster} className="w-full h-full"
      style={{objectFit:props.fit,transform:`translate(${props.panX}px, ${props.panY}px) scale(${props.scale})`,transition:props.isGesturing?"none":"transform .18s ease"}}
      onLoadedMetadata={dimensions} onLoadedData={dimensions} onTimeUpdate={e=>{props.onTime(e.currentTarget.currentTime);buffered(e.currentTarget);}}
      onProgress={e=>buffered(e.currentTarget)}
      onPlay={()=>{if(!ref.current?.paused){wantsPlay.current=true;props.onWaiting();}}}
      onWaiting={()=>{if(!ref.current?.paused)props.onWaiting();}} onSeeking={()=>{if(!ref.current?.paused)props.onWaiting();}} onSeeked={()=>{const v=ref.current;if(v&&!v.paused&&v.readyState>=3)props.onPlaying();}}
      onEnded={props.onEnded}>
      {props.media.playback.subtitles?.filter(t=>t.format==="vtt").map((t,i)=><track key={t.url} kind="subtitles" src={t.url} label={t.label||t.language||`字幕 ${i+1}`} srcLang={t.language} default={t.default}/>)}
    </video>
  </div>;
});
