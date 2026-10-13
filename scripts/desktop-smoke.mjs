// Genuine native framebuffers backed by public engine APIs, never synthetic images.
import {spawn} from 'node:child_process';
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs';
import {join,resolve} from 'node:path';
import assert from 'node:assert/strict';
const root=resolve(import.meta.dirname,'..');
const directory=join(root,'.smoke',`phase3-${Date.now()}`);
mkdirSync(directory,{recursive:true});
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const fixture=spawn(join(root,'target/release/examples/fixture_server.exe'),[],{windowsHide:true});
let serverError='';fixture.stderr.on('data',b=>serverError+=b);
let service;
const report=[];
async function seed(label,count,mode) {
  service?.kill();
  const state=join(directory,label,'state'),downloads=join(directory,label,'downloads');
  mkdirSync(state,{recursive:true});mkdirSync(downloads,{recursive:true});
  const env={...process.env,STRATAFETCH_STATE_DIR:state,STRATAFETCH_DOWNLOAD_DIR:downloads};
  if(count!==null) {
    service=spawn(join(root,'target/release/examples/ui_fixture.exe'),[String(count),mode??'mixed'],{env,windowsHide:true});
    let stdout='',stderr='';service.stdout.on('data',b=>stdout+=b);service.stderr.on('data',b=>stderr+=b);
    for(let i=0;i<600;i++) {if(stdout.includes('Ready:'))break;if(service.exitCode!==null)throw Error(stderr||'Fixture exited');await sleep(100);if(i===599)throw Error('Fixture timed out');}
  }
  return env;
}
async function capture(env,label,width=1280,height=720,theme='dark',scale=1,view='') {
  const path=join(root,`docs/screenshot-phase3-${label}.png`);
  const child=spawn(join(root,'target/release/stratafetch-desktop.exe'),[],{windowsHide:false,env:{...env,STRATAFETCH_SMOKE_OUTPUT:path,STRATAFETCH_SMOKE_WIDTH:String(width),STRATAFETCH_SMOKE_HEIGHT:String(height),STRATAFETCH_SMOKE_THEME:theme,STRATAFETCH_SMOKE_SCALE:String(scale),STRATAFETCH_SMOKE_VIEW:view}});
  await new Promise((resolve,reject)=>{const timeout=setTimeout(()=>{child.kill();reject(Error('Native screenshot timeout'));},30000);child.on('error',reject);child.on('exit',code=>{clearTimeout(timeout);code===0?resolve():reject(Error(`Native desktop exit ${code}`));});});
  const png=readFileSync(path);assert.equal(png.readUInt32BE(16),width);assert.equal(png.readUInt32BE(20),height);
  report.push({label,width,height,theme,scale,view,path});console.log(`Captured ${label}: ${width} × ${height}, scale ${scale}`);
}
try {
  for(let i=0;i<100;i++) {if(fixture.exitCode!==null)throw Error(serverError||'HTTP fixture exited');try{const r=await fetch('http://127.0.0.1:18765/fixture.bin?bytes=1');await r.arrayBuffer();break;}catch{await sleep(50);if(i===99)throw Error('HTTP fixture unavailable');}}
  await capture(await seed('empty',null),'empty');
  await capture(await seed('one',1,'one'),'one');
  const mixed=await seed('mixed',24);
  for(const spec of [['dark',1280,720,'dark',1,''],['light',1366,768,'light',1,''],['1920',1920,1080,'dark',1,''],['2560',2560,1440,'dark',1,''],['scale150',1280,720,'dark',1.5,''],['scale200',1280,720,'light',2,''],['transfer',1280,720,'dark',1,'transfer'],['settings',1280,720,'light',1,'settings'],['add',1280,720,'dark',1,'add'],['scheduler',1280,720,'dark',1,'scheduler']]) await capture(mixed,...spec);
  await capture(await seed('many',1000),'many',1920,1080);
  writeFileSync(join(root,'docs/PHASE3-CAPTURES.json'),JSON.stringify({date:new Date().toISOString(),fixtureRecords:{empty:0,one:1,mixed:29,many:1005},scaling:'egui pixels-per-point simulation; Windows system display scaling was not changed',captures:report},null,2));
} finally {service?.kill();fixture.kill();}
