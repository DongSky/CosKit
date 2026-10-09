const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const fs=require('node:fs');
const context={document:{body:{classList:{add(){},remove(){}}}},console};
vm.createContext(context);
vm.runInContext(fs.readFileSync('src/studio.js','utf8')+';this.Studio=CosKitStudio;',context);
function studio(){
  const s=Object.create(context.Studio.prototype);s.context={session:{session_id:'session'},node:{id:'child',parent_id:'root',children:['branch'],status:'done'}};s.redo=[];s.layerId='base';s.hint=()=>{};s.clearOverlay=()=>{};
  return s;
}
test('pointer coordinates use document scale, not viewport pixels',()=>{
  const s=studio();s.scale=0.5;s.overlay={getBoundingClientRect:()=>({left:100,top:200})};
  assert.deepEqual([...s.point({clientX:150,clientY:225})],[100,50]);
});
test('undo and redo return to the exact node on the selected branch',async()=>{
  const s=studio(),visited=[];s.bridge={navigate:async id=>visited.push(id)};
  await s.navigate();assert.deepEqual(visited,['root']);assert.deepEqual(s.redo,['child']);
  s.context.node={id:'root',children:['other','child']};await s.navigate(true);assert.deepEqual(visited,['root','child']);
});
test('failed local edit never accepts a new history node',async()=>{
  const s=studio();let accepted=false,error='';s.bridge={mask:()=>null,invoke:async()=>{throw Error('disk full');},accept:()=>accepted=true,toast:e=>error=e};
  await s.apply('brush',{},[[1,2]]);assert.equal(accepted,false);assert.match(error,/disk full/);assert.equal(s.busy,false);
});
test('successful geometry invalidates selection and clears redo',async()=>{
  const s=studio();let args,accept;s.redo=['old'];s.bridge={mask:()=> 'mask',invoke:async(cmd,p)=>{args=p;return {id:'new'};},accept:async(...a)=>accept=a,toast(){}};
  await s.apply('crop',{width:10,height:10});assert.equal(args.request.operation,'crop');assert.equal(args.node_id,'child');assert.equal(accept[2],false);assert.equal(s.redo.length,0);
});
test('new layer becomes the editing target; pixel operations keep selection',async()=>{
  const s=studio();let keep;s.bridge={mask:()=> 'mask',invoke:async()=>({id:'new',layers:[{id:'base'},{id:'paint'}]}),accept:async(id,sid,p)=>keep=p,toast(){}};
  await s.apply('layer_new');assert.equal(s.layerId,'paint');assert.equal(keep,true);
});
test('busy editor rejects overlapping strokes',async()=>{
  const s=studio();s.busy=true;s.bridge={invoke:()=>{throw Error('should not run');}};await s.apply('brush');
});
