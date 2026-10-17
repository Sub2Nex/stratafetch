import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
import {createHash,webcrypto} from 'node:crypto';
const read = (folder,file) => readFileSync(new URL('../extensions/'+folder+'/'+file,import.meta.url),'utf8');
function background(folder, options={}) {
  const calls=[], hooks={}, stored={...(options.stored || {})};
  let enabled=options.enabled ?? true;
  const item={id:7,url:'https://example.org/file.zip',filename:'C:\\Downloads\\file.zip',state:'in_progress',startTime:'2026-10-16T12:00:00Z',danger:'safe',totalBytes:100,mime:'application/zip',...options.item};
  for (const value of (options.items || [])) {
    if (value.filename === undefined) value.filename = 'file.zip';
    if (value.mime === undefined) value.mime = 'application/zip';
  }
  const items = new Map([item, ...(options.items || [])].map(value => [value.id, value]));
  const searches = new Map();
  const api={
    runtime:{onStartup:{addListener(fn){hooks.startup=fn;}},onMessage:{addListener(fn){hooks.message=fn;}},onInstalled:{addListener(fn){hooks.installed=fn;}},async sendNativeMessage(host,payload){
      if(payload.action==='probeReplay'){calls.push({op:'probe',payload});return {ok:true,replayable:Boolean(options.replayable)};}
      calls.push({op:'native',host,payload});
      if(options.native) return options.native(payload);
      return payload.action==='cancel' ? {ok:true} : {ok:true,download:{id:'native-id'}};
    }},
    storage:{local:{async get(keys){return keys===null?{capture:enabled,...stored}:{...keys,...stored,capture:enabled};},async set(value){if ("capture" in value) enabled=value.capture;Object.assign(stored,value);},async remove(key){delete stored[key];}}},
    contextMenus:{async removeAll(){},create(value){calls.push({op:'menu',value});},onClicked:{addListener(fn){hooks.clicked=fn;}}},
    downloads:{
      onCreated:{addListener(fn){hooks.created=fn;}},
      onChanged:{addListener(fn){hooks.changed=fn;}},
      onDeterminingFilename:{addListener(fn){hooks.filename=fn;}},
      async pause(id){calls.push({op:'pause',id});if(options.pauseWait)await options.pauseWait;if(options.pauseError)throw Error('pause failed');items.get(id).paused=true;},
      async search({id}){calls.push({op:'search',id});const count=(searches.get(id)||0)+1;searches.set(id,count);const changes=typeof options.searchItem==='function'?options.searchItem(id,count):options.searchItem;return items.has(id)?[{...items.get(id),...changes}]:[];},
      async resume(id){calls.push({op:'resume',id});if(options.resumeError)throw Error('resume failed');items.get(id).paused=false;},
      async erase({id}){calls.push({op:'erase',id});items.delete(id);return [id];},
      async download(options){calls.push({op:'browser-download',options});if(options.restartError)throw Error('restart failed');return 99;},
      async cancel(id){calls.push({op:'cancel',id});if(options.cancelError)throw Error('cancel failed');if(!options.cancelNoEffect)Object.assign(items.get(id),{state:'interrupted',error:'USER_CANCELED'});},
    },
    action:Object.fromEntries(['setBadgeText','setBadgeBackgroundColor','setTitle'].map(name=>[name,async value=>{calls.push({op:name,...value});if(options.badgeError)throw Error('badge failed');}])),
  };
  runInNewContext(read(folder,'background.js'),{URL,setTimeout,crypto:webcrypto,TextEncoder,[folder==='firefox'?'browser':'chrome']:api});
  return {api,calls,hooks,item,items,stored,enabled:()=>enabled};
}
for(const folder of ['chromium','firefox']) {
  for(const accept of [false,true]) {
    test(folder+': verified public request stops before choice; accept='+accept,async()=>{
      const b=background(folder,{replayable:true});let receive,disconnect;const sent=[];
      b.api.runtime.connectNative=()=>({onMessage:{addListener(f){receive=f;}},onDisconnect:{addListener(f){disconnect=f;}},postMessage(p){sent.push(p);},disconnect(){disconnect();}});
      const capture=b.hooks.created(b.item);
      while(!sent.length||b.item.state!=='interrupted')await new Promise(r=>setImmediate(r));
      assert.equal(b.calls.filter(c=>c.op==='cancel').length,1);
      assert.equal(sent.some(p=>p.action==='commitCapture'),false);
      assert.equal(b.stored.capture_pending_7.exclusive.url,b.item.url);
      if(accept){
        receive({ok:true,confirmation:'accepted'});
        while(!sent.some(p=>p.action==='commitCapture'))await new Promise(r=>setImmediate(r));
        assert.equal(b.item.state,'interrupted');
        receive({ok:true,download:{id:'native-id'}});
      }else receive({ok:false,declined:true});
      await capture;
      assert.equal(b.calls.filter(c=>c.op==='browser-download').length,accept?0:1);
      assert.equal(b.calls.some(c=>c.op==='resume'),false);
      assert.equal('capture_pending_7' in b.stored,false);
    });
  }
  test(folder+': cancellation that does not stop the browser never authorizes native commit',async()=>{
    const b=background(folder,{replayable:true,cancelNoEffect:true});let receive,disconnect;const sent=[];
    b.api.runtime.connectNative=()=>({onMessage:{addListener(f){receive=f;}},onDisconnect:{addListener(f){disconnect=f;}},postMessage(p){sent.push(p);},disconnect(){disconnect();}});
    const capture=b.hooks.created(b.item);
    while(!sent.length)await new Promise(r=>setImmediate(r));
    receive({ok:true,confirmation:'accepted'});await capture;
    assert.equal(sent.some(p=>p.action==='commitCapture'),false);
    assert.match(b.stored.lastHandoffError.message,/cancellation was not confirmed/);
  });
  test(folder+': restart recovers a stopped public download through normal browser download API',async()=>{
    const b=background(folder,{item:{state:'interrupted',error:'USER_CANCELED'},stored:{capture_pending_7:{startTime:'2026-10-16T12:00:00Z',exclusive:{url:'https://example.org/file.zip',name:'file.zip',size:100}}}});
    await b.hooks.startup();
    assert.equal(b.calls.filter(c=>c.op==='browser-download').length,1);
    assert.equal(b.calls.some(c=>c.op==='resume'),false);
    assert.equal('capture_pending_7' in b.stored,false);
  });
  test(folder+': recovery never restarts a stopped file subsequently marked unsafe',async()=>{
    const b=background(folder,{item:{state:'interrupted',error:'USER_CANCELED',danger:'dangerous'},stored:{capture_pending_7:{startTime:'2026-10-16T12:00:00Z',exclusive:{url:'https://example.org/file.zip',name:'file.zip',size:100}}}});
    await b.hooks.startup();
    assert.equal(b.calls.some(c=>c.op==='browser-download'||c.op==='resume'),false);
    assert.equal('capture_pending_7' in b.stored,false);
  });
  test(folder+': confirmation is sent while pause API remains unresolved',async()=>{
    let release;const pauseWait=new Promise(r=>{release=r;});
    const b=background(folder,{pauseWait});let receive,disconnect;const sent=[];
    b.api.runtime.connectNative=()=>({onMessage:{addListener(f){receive=f;}},onDisconnect:{addListener(f){disconnect=f;}},postMessage(p){sent.push(p);},disconnect(){disconnect();}});
    const capture=b.hooks.created(b.item);
    while(!sent.length)await new Promise(r=>setImmediate(r));
    assert.equal(sent[0].action,'capture');
    assert.equal(Boolean(b.item.paused),false);
    release();receive({ok:false,declined:true});await capture;
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
  });
  for(const accept of [false,true]) {
    test(folder+': fast completed file remains available during confirmation, accept='+accept,async()=>{
      const b=background(folder);let receive,disconnect;const sent=[];
      b.api.runtime.connectNative=()=>({onMessage:{addListener(f){receive=f;}},onDisconnect:{addListener(f){disconnect=f;}},postMessage(p){sent.push(p);},disconnect(){disconnect();}});
      const capture=b.hooks.created(b.item);
      while(!sent.length)await new Promise(r=>setImmediate(r));
      Object.assign(b.item,{state:'complete',paused:false,fileSize:100});
      await b.hooks.changed({id:7,state:{current:'complete'},paused:{current:false}});
      assert.equal(b.calls.some(c=>c.op==='erase'||c.op==='cancel'),false);
      if(accept){
        receive({ok:true,confirmation:'accepted'});
        while(!sent.some(p=>p.action==='adoptCapture'))await new Promise(r=>setImmediate(r));
        const adopt=sent.find(p=>p.action==='adoptCapture');
        assert.equal(adopt.path,b.item.filename);assert.equal(adopt.size,100);
        receive({ok:true,adopted:true,download:{id:'adopted-id'}});
      }else receive({ok:false,declined:true});
      await capture;
      assert.equal(b.calls.filter(c=>c.op==='erase').length,accept?1:0);
      assert.equal(b.calls.some(c=>c.op==='cancel'||c.op==='resume'),false);
    });
  }
  for(const unsafe of [false,true]) {
    test(folder+': final native commit rechecks safety and holds an externally resumed download '+unsafe,async()=>{
      const b=background(folder);let receive,disconnect;const sent=[];
      b.api.runtime.connectNative=()=>({
        onMessage:{addListener(fn){receive=fn;}},onDisconnect:{addListener(fn){disconnect=fn;}},
        postMessage(payload){sent.push(payload);},disconnect(){disconnect();}
      });
      const handoff=b.hooks.created(b.item);
      while(!sent.length)await new Promise(r=>setImmediate(r));
      assert.equal(b.item.paused,true);
      b.item.paused=false;
      if(unsafe)b.item.danger='dangerous';
      await b.hooks.changed({id:7,paused:{current:false}});
      receive({ok:true,confirmation:'accepted'});
      if(unsafe) {
        await handoff;
        assert.equal(sent.some(p=>p.action==='commitCapture'),false);
        assert.equal(b.calls.some(c=>c.op==='cancel'),false);
      } else {
        while(!sent.some(p=>p.action==='commitCapture'))await new Promise(r=>setImmediate(r));
        assert.equal(b.item.paused,true);
        assert.equal(sent.filter(p=>p.action==='capture').length,1);
        receive({ok:true,download:{id:'native-id'}});
        await handoff;
        assert.equal(b.item.state,'interrupted');
        assert.equal(b.calls.some(c=>c.op==='resume'),false);
      }
    });
  }
  test(folder+': onCreated and filename events open one prompt without waiting for metadata',async()=>{
    const b=background(folder,{item:{filename:'',mime:''}});
    const capture=b.hooks.created(b.item);
    let suggested=false;
    b.hooks.filename({...b.item,filename:'file.zip'},()=>{suggested=true;});
    await capture;
    assert(suggested);
    assert.equal(b.calls.filter(c=>c.payload?.action==='capture').length,1);
    assert.equal(b.calls.some(c=>c.op==='pause'),true);
    assert.equal(b.calls.find(c=>c.payload?.action==='capture').payload.mime,undefined);
  });
  test(folder+': legacy metadata-wait records are cleaned without recapturing old downloads',async()=>{
    const b=background(folder,{stored:{capture_candidate_7:{observedAt:Date.now(),urlHash:'old'}}});
    await b.hooks.startup();
    await b.hooks.changed({id:7});
    assert.equal(b.calls.some(c=>c.op==='native'),false);
    assert.equal('capture_candidate_7' in b.stored,false);
  });
  test(folder+': decline is not recaptured by resume notifications',async()=>{
    const b=background(folder,{native:async()=>({ok:false,declined:true})});
    await b.hooks.created(b.item);
    await b.hooks.changed({id:7,paused:{current:false}});
    await b.hooks.created(b.item);
    assert.equal(b.calls.filter(c=>c.payload?.action==='capture').length,1);
    assert.equal(b.item.paused,false);
  });
  test(folder+': unconfirmed native rollback preserves pause and recovery record',async()=>{
    let rollbackFails=true;
    const b=background(folder,{cancelError:true,native:async p=>p.action==='capture'?{ok:true,download:{id:'native-id'}}:{ok:!rollbackFails}});
    await b.hooks.created(b.item);
    assert.equal(b.item.paused,true);
    assert.equal(b.stored.capture_pending_7.nativeId,'native-id');
    await b.hooks.startup();
    assert.equal(b.calls.some(c=>c.op==='resume'),false);
    assert.equal(b.stored.capture_pending_7.nativeId,'native-id');
    rollbackFails=false;
    await b.hooks.startup();
    assert.equal(b.item.paused,false);
    assert.equal('capture_pending_7' in b.stored,false);
  });
  test(folder+': resolved cancellation that leaves Chrome active rolls back before resume',async()=>{
    const b=background(folder,{cancelNoEffect:true});
    await b.hooks.created(b.item);
    const rollback=b.calls.findIndex(c=>c.payload?.action==='cancel');
    assert(rollback>=0);
    assert(b.calls.findIndex(c=>c.op==='resume')>rollback);
    assert.match(b.stored.lastHandoffError.message,/cancellation was not confirmed/);
  });
  test(folder+': bounded local state trace links acknowledgment to browser cancellation',async()=>{
    const b=background(folder,{stored:{handoffEvents:Array.from({length:20},()=>({event:'old'}))}});
    await b.hooks.created(b.item);
    for(let i=0;i<30;i++)await new Promise(r=>setImmediate(r));
    const events=b.stored.handoffEvents;
    assert.equal(events.length,20);
    const ack=events.find(e=>e.event==='native_acknowledged');
    const cancelled=events.find(e=>e.event==='browser_cancelled');
    assert.equal(ack.downloadId,cancelled.downloadId);
    assert.equal(ack.nativeId,cancelled.nativeId);
    assert.equal(/example.org|file.zip|Downloads/.test(JSON.stringify(events)),false);
  });
  for (const startTime of ['2026-10-16T14:00:00+02:00','2026-10-16T12:00:00.500Z','2026-10-16T11:59:59.500Z']) {
    test(folder+': Chrome timestamp updates do not reject the same live download '+startTime,async()=>{
      let record;
      const b=background(folder,{searchItem:{startTime},native:async()=>{record={...b.stored.capture_pending_7};return {ok:true,download:{id:'native-id'}};}});
      await b.hooks.created(b.item);
      assert.equal(b.calls.some(c=>c.op==='cancel'),true);
      assert.equal(record.startTime,startTime);
      assert.equal(b.calls.some(c=>c.op==='resume'),false);
    });
  }
  test(folder+': delayed pause snapshots settle before native handoff',async()=>{
    const b=background(folder,{searchItem:(_id,count)=>count<3?{paused:false}:{}});
    await b.hooks.created(b.item);
    const native=b.calls.findIndex(c=>c.op==='native');
    assert(native>=0);
    assert(b.calls.filter(c=>c.op==='pause').length>=1);
    assert.equal(b.calls.some(c=>c.op==='cancel'),true);
  });
  test(folder+': an unconfirmed pause times out without taking the download',async()=>{
    const b=background(folder,{searchItem:{paused:false}});
    await b.hooks.created(b.item);
    assert.equal(b.calls.some(c=>c.payload?.action==='capture'),true);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
    assert.match(b.stored.lastHandoffError.message,/did not confirm its pause/);
  });
  test(folder+': a pause that cannot be held prevents native handoff',async()=>{
    const b=background(folder,{searchItem:(_id,count)=>count>=3?{paused:false}:{}});
    await b.hooks.created(b.item);
    assert.equal(b.calls.some(c=>c.payload?.action==='capture'),true);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
    assert(b.calls.filter(c=>c.op==='pause').length>=1);
  });
  test(folder+': safety changes during pause settling never reach the host',async()=>{
    const b=background(folder,{searchItem:(_id,count)=>count===1?{paused:false}:{danger:'dangerous'}});
    await b.hooks.created(b.item);
    assert.equal(b.calls.some(c=>c.op==='native'||c.op==='cancel'),false);
    assert.match(b.stored.lastHandoffError.message,/safety status is dangerous/);
  });
  test(folder+': the same ID cannot substitute a different original URL',async()=>{
    const b=background(folder,{searchItem:{url:'https://other.example.org/file.zip'}});
    await b.hooks.created(b.item);
    assert.equal(b.calls.some(c=>c.op==='native'||c.op==='cancel'),false);
  });
  for (const fileSize of [-1,0]) {
    test(folder+': initial zero total size is trusted only when file size confirms an empty file '+fileSize,async()=>{
      const b=background(folder,{item:{totalBytes:0,fileSize}});
      await b.hooks.created(b.item);
      assert.equal(b.calls.find(c=>c.op==='native').payload.size,fileSize===0?0:undefined);
    });
  }
  test(folder+': a safety change after acknowledgment rolls back native work first',async()=>{
    const b=background(folder,{native:async payload=>{
      if(payload.action==='capture'){b.items.get(7).danger='dangerous';return {ok:true,download:{id:'native-id'}};}
      return {ok:true};
    }});
    await b.hooks.created(b.item);
    const rollback=b.calls.findIndex(c=>c.payload?.action==='cancel');
    assert(rollback>=0);
    assert(rollback<b.calls.findIndex(c=>c.op==='resume'));
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
  });
  test(folder+': native launch failures retain the reason without signed URLs',async()=>{
    const b=background(folder,{native:async()=>{throw Error('Native host exited https://example.org/private?token=secret');}});
    await b.hooks.created(b.item);
    assert.match(b.stored.lastHandoffError.message,/Opening browser bridge: Native host exited/);
    assert.equal(b.stored.lastHandoffError.message.includes('secret'),false);
    assert.equal(b.calls.some(c=>c.op==='resume'),true);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
  });
  test(folder+': successful capture clears the previous handoff issue',async()=>{
    const b=background(folder,{stored:{lastHandoffError:{message:'old failure'}}});
    await b.hooks.created(b.item);
    assert.equal('lastHandoffError' in b.stored,false);
  });
  test(folder+': popup retains the last failure even when a new connection ping succeeds',async()=>{
    const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded','last-issue'].map(id=>[id,{}]));
    const api={runtime:{async sendMessage(){return {ok:true,active:0};}},storage:{local:{async get(){return {capture:true,lastHandoffError:{message:'Opening browser bridge: Host exited'}};}}}};
    runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
    await new Promise(resolve=>setImmediate(resolve));
    assert.match(elements.status.textContent,/Connected/);
    assert.match(elements['last-issue'].textContent,/Last handoff issue: Opening browser bridge: Host exited/);
  });
  test(folder+': only native/download/storage permissions, no cookies or broad page access',()=>{
    const m=JSON.parse(read(folder,'manifest.json'));
    assert.deepEqual(m.permissions.slice().sort(),['activeTab','contextMenus','downloads','nativeMessaging','storage']);
    assert.equal(m.host_permissions,undefined);assert.equal(m.content_scripts,undefined);
  });
  test(folder+': installation preserves disabled capture',async()=>{
    const b=background(folder,{enabled:false});await b.hooks.installed();assert.equal(b.enabled(),false);
    assert.deepEqual(Array.from(b.calls[0].value.contexts),['link']);
  });
  for(const result of ['success','declined','rejected','unavailable']) {
    test(folder+': context menu '+result,async()=>{
      const b=background(folder,{native:async()=>{if(result==='unavailable')throw Error();return result==='success'?{ok:true}:result==='declined'?{ok:false,declined:true}:{ok:false,error:'Unsupported URL'};}});
      await b.hooks.clicked({menuItemId:'stratafetch',linkUrl:'https://example.org/a.zip'});
      assert.equal(b.calls[0].payload.action,'capture');
      assert.deepEqual(Object.keys(b.calls[0].payload).sort(),['action','url']);
      const count=b.calls.length;await b.hooks.clicked({menuItemId:'other'});assert.equal(b.calls.length,count);
    });
  }
  test(folder+': browser is cancelled only after native acknowledgement',async()=>{
    const b=background(folder);await b.hooks.created(b.item);
    assert(b.calls.findIndex(c=>c.op==='pause')>=0);
    const native=b.calls.findIndex(c=>c.op==='native');
    assert(native>b.calls.findIndex(c=>c.op==='pause'));assert(b.calls.findIndex(c=>c.op==='cancel')>native);
    assert.equal(b.calls.some(c=>c.op==='resume'),false);
    const payload=b.calls[native].payload;
    assert.deepEqual(Object.keys(payload).sort(),['action','mime','name','size','url']);
    assert.equal(payload.name,'file.zip');assert.equal(payload.size,100);
  });
  for(const failure of ['decline','reject','unavailable','missing-id','pause','cancel','changed','badge']) {
    test(folder+': handoff '+failure+' preserves the browser download',async()=>{
      const b=background(folder,{
        pauseError:failure==='pause',cancelError:failure==='cancel',badgeError:failure==='badge',
        searchItem:failure==='changed'?{danger:'dangerous'}:undefined,
        native:async payload=>{
          if(payload.action==='cancel')return {ok:true};
          if(failure==='unavailable')throw Error('host missing');
          if(failure==='decline')return {ok:false,declined:true};
          if(failure==='reject')return {ok:false,error:'server rejected request'};
          return {ok:true,download:failure==='missing-id'?{}:{id:'native-id'}};
        }
      });
      await b.hooks.created(b.item);
      if(failure==='badge')assert.equal(b.calls.some(c=>c.op==='cancel'),true);
      else if(failure==='pause')assert.equal(b.calls.some(c=>c.op==='cancel'),false);
      else if(failure==='changed')assert.equal(b.calls.some(c=>c.op==='native'),false);
      else {
        assert.equal(b.calls.some(c=>c.op==='resume'),true);
        if(failure==='cancel')assert.equal(b.calls.some(c=>c.payload?.action==='cancel'),true);
      }
    });
  }
  for(const item of [{incognito:true},{byExtensionId:'another'},{state:'complete'},{paused:true},{url:'blob:secret'},{url:'data:text/plain,hi'},{url:'https://user:secret@example.org/file.zip'},{url:'http://['},{mime:'application/x-chrome-extension'},{danger:'dangerous'},{mime:'text/html'},{url:'https://example.org/extension.xpi'},{url:'https://example.org/extension.crx'}]) {
    test(folder+': does not capture '+JSON.stringify(item),async()=>{
      const b=background(folder,{item});await b.hooks.created(b.item);assert.equal(b.calls.length,0);
    });
  }
  test(folder+': disabled capture does not pause downloads',async()=>{
    const b=background(folder,{enabled:false});await b.hooks.created(b.item);assert.equal(b.calls.length,0);
  });
  test(folder+': duplicate events cannot create duplicate native transfers',async()=>{
    let reply;const b=background(folder,{native:()=>new Promise(resolve=>{reply=resolve;})});
    const pending=b.hooks.created(b.item);
    while(!reply)await new Promise(resolve=>setImmediate(resolve));
    await b.hooks.created(b.item);
    assert.equal(b.calls.filter(c=>c.op==='native').length,1);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
    reply({ok:true,download:{id:'native-id'}});await pending;
    assert.equal(b.calls.filter(c=>c.op==='cancel').length,1);
  });
  test(folder+': native port remains connected while awaiting confirmation',async()=>{
    const b=background(folder);let message,disconnect;let closed=false;
    b.api.runtime.connectNative=()=>({
      onMessage:{addListener(fn){message=fn;}},onDisconnect:{addListener(fn){disconnect=fn;}},
      postMessage(){},disconnect(){closed=true;disconnect();}
    });
    const pending=b.hooks.created(b.item);
    while(!message)await new Promise(resolve=>setImmediate(resolve));
    assert.equal(closed,false);assert.equal(b.calls.some(c=>c.op==='cancel'),false);
    message({ok:false,declined:true});await pending;
    assert.equal(closed,true);assert.equal(b.calls.some(c=>c.op==='resume'),true);
  });
  test(folder+': a failed native send closes its port and resumes the browser',async()=>{
    const b=background(folder);let closed=false;
    b.api.runtime.connectNative=()=>({onMessage:{addListener(){}},onDisconnect:{addListener(){}},postMessage(){throw Error('send failed');},disconnect(){closed=true;}});
    await b.hooks.created(b.item);
    assert(closed);assert.equal(b.calls.some(c=>c.op==='resume'),true);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
  });
  for(const result of ['empty','restricted','error']) {
    test(folder+': current page '+result+' has an actionable error',async()=>{
      const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{value:'previous URL'}]));
      const api={runtime:{async sendMessage(){return {ok:true,active:0};}},storage:{local:{async get(){return {capture:true};},async set(){}}},
        tabs:{async query(){if(result==='error')throw Error();return result==='empty'?[]:[{id:1}];}}};
      runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
      await new Promise(resolve=>setImmediate(resolve));await elements.current.onclick();
      assert.equal(elements.url.value,'');assert.match(elements.status.textContent,/cannot read.*paste/i);
    });
  }
  test(folder+': late startup ping does not overwrite a current-tab error',async()=>{
    const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{value:'previous'}]));let ping;
    const api={runtime:{sendMessage(){return new Promise(resolve=>{ping=resolve;});}},
      storage:{local:{async get(){return {capture:false};},async set(){}}},tabs:{async query(){throw Error();}}};
    runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
    await elements.current.onclick();const error=elements.status.textContent;ping({ok:true,active:0});await new Promise(resolve=>setImmediate(resolve));await new Promise(resolve=>setImmediate(resolve));
    assert.equal(elements.status.textContent,error);
  });
}
test('local Chromium ID derives from the bundled public key and is independent of install path',()=>{
  const m=JSON.parse(read('chromium','manifest.json'));
  const hash=createHash('sha256').update(Buffer.from(m.key,'base64')).digest().subarray(0,16);
  const id=[...hash].flatMap(b=>[b>>4,b&15]).map(n=>String.fromCharCode(97+n)).join('');
  const ids=JSON.parse(readFileSync(new URL('../extensions/local-ids.json',import.meta.url)));
  assert.equal(id,ids.chromium);
  assert.equal(JSON.parse(read('firefox','manifest.json')).browser_specific_settings.gecko.id,ids.firefox);
});

for (const folder of ['chromium','firefox']) {
  test(folder+': popup relay keeps native work alive if its response view closes',async()=>{
    const b=background(folder);let finish;
    b.api.runtime.sendNativeMessage=()=>new Promise(resolve=>{finish=resolve;});
    const kept=b.hooks.message({action:'capture',url:'https://example.org/file.zip'},{},()=>{throw Error('popup closed');});
    assert.equal(kept,true);
    assert.equal(b.hooks.message({action:'unknown'},{},()=>{}),false);
    while(!finish)await new Promise(resolve=>setImmediate(resolve));
    finish({ok:true,download:{id:'native-id'}});
    await new Promise(resolve=>setImmediate(resolve));await new Promise(resolve=>setImmediate(resolve));
  });
}


for (const folder of ['chromium','firefox']) {
  for(const url of ['https://example.org/file.zip','https://cdn.example.org/file.zip']) {
    test(folder+': excluded domain remains in browser '+url,async()=>{
      const b=background(folder,{item:{url},stored:{excludedDomains:['example.org']}});
      await b.hooks.created(b.item);assert.equal(b.calls.length,0);
    });
  }
  test(folder+': unrelated domains are still captured',async()=>{
    const b=background(folder,{stored:{excludedDomains:['other.example.org']}});
    await b.hooks.created(b.item);assert(b.calls.some(call=>call.op==='cancel'));
  });
  test(folder+': exclusions added while waiting prevent a queued native prompt',async()=>{
    const next={id:8,url:'https://second.example.org/file.zip',state:'in_progress',startTime:'second'};
    let finish;const b=background(folder,{items:[next],native:()=>new Promise(resolve=>{finish=resolve;})});
    const first=b.hooks.created(b.item),second=b.hooks.created(next);
    while(!finish)await new Promise(resolve=>setImmediate(resolve));
    while(!next.paused)await new Promise(resolve=>setImmediate(resolve));
    await b.api.storage.local.set({excludedDomains:['second.example.org']});
    finish({ok:false,declined:true});await Promise.all([first,second]);
    assert.equal(b.calls.filter(call=>call.op==='native').length,1);
    assert(b.calls.some(call=>call.op==='resume'&&call.id===8));
  });
  for(const invalid of [false,true]) {
    test(folder+': popup validates domain exclusions '+invalid,async()=>{
      const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{}]));
      let saved;
      const api={runtime:{async sendMessage(){return {ok:true};}},storage:{local:{async get(){return {capture:true};},async set(value){saved=value;}}}};
      runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
      await new Promise(resolve=>setImmediate(resolve));
      elements.excluded.value=invalid?'example.org/path':'EXAMPLE.ORG\n*.cdn.example.org\nexample.org';
      await elements['save-excluded'].onclick();
      if(invalid) {assert.equal(saved,undefined);assert.match(elements.status.textContent,/without URLs/);}
      else {assert.deepEqual(Array.from(saved.excludedDomains),['example.org','cdn.example.org']);assert.match(elements.status.textContent,/Exclusions saved/);}
    });
  }
  test(folder+': simultaneous downloads pause immediately and prompt one at a time', async()=>{
    const next={id:8,url:'https://example.org/second.zip',filename:'second.zip',state:'in_progress',startTime:'second'};
    const finishes=[];
    const b=background(folder,{items:[next],native:()=>new Promise(resolve=>finishes.push(resolve))});
    const first=b.hooks.created(b.item), second=b.hooks.created(next);
    while(!finishes.length)await new Promise(resolve=>setImmediate(resolve));
    while(!next.paused)await new Promise(resolve=>setImmediate(resolve));
    assert.equal(b.calls.filter(call=>call.op==='pause').length,2);
    assert.equal(finishes.length,1);
    finishes[0]({ok:false,declined:true});
    while(finishes.length<2)await new Promise(resolve=>setImmediate(resolve));
    finishes[1]({ok:false,declined:true});await Promise.all([first,second]);
    assert.equal(b.calls.filter(call=>call.op==='resume').length,2);
  });
  test(folder+': a cancelled queued download never opens a native confirmation', async()=>{
    const next={id:8,url:'https://example.org/second.zip',state:'in_progress',startTime:'second'};
    let finish;const b=background(folder,{items:[next],native:()=>new Promise(resolve=>{finish=resolve;})});
    const first=b.hooks.created(b.item),second=b.hooks.created(next);
    while(!finish)await new Promise(resolve=>setImmediate(resolve));
    next.state='interrupted';finish({ok:false,declined:true});await Promise.all([first,second]);
    assert.equal(b.calls.filter(call=>call.op==='native').length,1);
    assert.equal(b.calls.some(call=>call.op==='resume'&&call.id===8),false);
  });
  for(const searchItem of [{incognito:true},{url:'blob:private',finalUrl:'blob:private'},{mime:'text/html'},{filename:'extension.xpi'},{startTime:'different'}]) {
    test(folder+': changed download is rechecked before URL transmission '+JSON.stringify(searchItem),async()=>{
      const b=background(folder,{searchItem});await b.hooks.created(b.item);
      assert.equal(b.calls.some(call=>call.op==='native'),false);
      assert.equal(b.calls.some(call=>call.op==='cancel'),false);
    });
  }
  test(folder+': failed resume retains recovery data and retries successfully', async()=>{
    const options={resumeError:true,native:async()=>({ok:false,declined:true})};
    const b=background(folder,options);await b.hooks.created(b.item);
    assert.equal(b.item.paused,true);assert('capture_pending_7' in b.stored);
    await b.hooks.startup();assert('capture_pending_7' in b.stored);
    options.resumeError=false;await b.hooks.startup();
    assert.equal(b.item.paused,false);assert.equal('capture_pending_7' in b.stored,false);
  });
  test(folder+': recovery rolls back an acknowledged native copy before browser resume',async()=>{
    const b=background(folder,{item:{paused:true},stored:{capture_pending_7:{startTime:'2026-10-16T12:00:00Z',nativeId:'native-id'}}});
    await b.hooks.startup();
    const cancel=b.calls.findIndex(call=>call.payload?.action==='cancel');
    assert(cancel>=0);assert(b.calls.findIndex(call=>call.op==='resume')>cancel);
  });
  for(const reply of [{ok:false,error:'Native host unavailable'},undefined,{ok:true,running:false}]) {
    test(folder+': popup accurately reports bridge readiness '+JSON.stringify(reply),async()=>{
      const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{}]));
      const api={runtime:{async sendMessage(){return reply;}},storage:{local:{async get(){return {capture:true};}}}};
      runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
      await new Promise(resolve=>setImmediate(resolve));
      assert.match(elements.status.textContent,reply?.ok?/Ready.*opens/:/unavailable|did not respond/i);
    });
  }
  for (const state of ['owned','different-download','already-resumed']) {
    test(folder+': restart recovery '+state+' affects only a paused owned download', async()=>{
      const b=background(folder,{
        item:{paused:state!=='already-resumed'},
        stored:{capture_pending_7:{startTime:state==='different-download'?'old-start':'2026-10-16T12:00:00Z'}}
      });
      await b.hooks.startup();
      assert.equal(b.calls.some(call=>call.op==='resume'),state==='owned');
      assert.equal('capture_pending_7' in b.stored,false);
      assert.equal(b.calls.some(call=>call.op==='native'),false);
    });
  }
}
