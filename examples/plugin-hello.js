export const meta = {
  id: 'example.hello', name: '虚构示例', version: '1.0.0', protocol: 1,
  settings: [{key:'videoUrl',type:'text',label:'HTTPS MP4 地址',default:''}],
};
export default hg => {
  const card={id:'demo',title:'示例短片',type:'short'};
  return {
    home:{getSections(){return {items:[{id:'featured',title:'示例推荐',layout:'row',items:[card]}],next:null};}},
    search:{search(){return {items:[card],next:null};}},
    media:{
      getDetail({id}){if(id!=='demo')throw new hg.PluginError('NOT_FOUND','内容不存在');return {...card,playlists:[{id:'main',episodes:[{id:'first',index:1,label:'第一集'}]}]};},
      resolvePlayback({episodeId}){
        if(episodeId!=='first')throw new hg.PluginError('NOT_FOUND','分集不存在');
        const url=hg.settings.get('videoUrl');
        if(typeof url!=='string'||!url.startsWith('https://'))throw new hg.PluginError('INVALID_ARGUMENT','请在插件设置填写自己的 HTTPS MP4 地址');
        return {streams:[{url,format:'progressive'}]};
      },
    },
  };
};
