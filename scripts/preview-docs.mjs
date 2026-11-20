import {createServer} from 'node:http';
import {readFile,stat} from 'node:fs/promises';
import {resolve,sep,extname} from 'node:path';
const root=resolve('dist-docs');const {base}=JSON.parse(await readFile(resolve(root,'build.json'),'utf8'));
const mime={'.html':'text/html; charset=utf-8','.css':'text/css','.js':'text/javascript','.json':'application/json','.zip':'application/zip','.py':'text/plain'};
createServer(async(req,res)=>{try{let path=decodeURIComponent(new URL(req.url,'http://localhost').pathname);if(path==='/'&&base!=='/'){res.writeHead(302,{Location:base});return res.end();}if(!path.startsWith(base))throw Error();path=path.slice(base.length)||'index.html';let file=resolve(root,path);if(!file.startsWith(root+sep))throw Error();if((await stat(file)).isDirectory())file=resolve(file,'index.html');res.setHeader('Content-Type',mime[extname(file)]||'application/octet-stream');res.end(await readFile(file));}catch{res.writeHead(404);res.end('Not found');}}).listen(4174,'127.0.0.1',()=>console.log('http://127.0.0.1:4174'+base));
