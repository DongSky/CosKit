/* CosKit Studio: local tools and AI share the same session snapshots. */
class CosKitStudio {
  constructor(bridge) {
    this.bridge = bridge; this.tool = 'hand'; this.scale = 1; this.pan = [0, 0];
    this.layerId = ''; this.busy = false; this.redo = []; this.context = null;
    this.image = document.getElementById('canvas-image');
    this.wrap = document.getElementById('canvas-image-wrap');
    this.tools = [
      ['hand', '平移', 'H', 'M7 10V5a1 1 0 0 1 2 0v5-7a1 1 0 0 1 2 0v7-6a1 1 0 0 1 2 0v6-4a1 1 0 0 1 2 0v7c0 6-7 6-9 2l-3-4c-1-2 1-3 2-1l2 2'],
      ['select', '选区', 'M', 'M3 7V3h4m6 0h4v4m0 6v4h-4m-6 0H3v-4'],
      ['crop', '裁剪', 'C', 'M6 2v12h12M2 6h12v12'],
      ['brush', '画笔', 'B', 'm6 12 8-9 3 3-9 8M7 12c-4-1-2 4-5 5 5 1 7-1 5-5'],
      ['erase', '橡皮', 'E', 'm3 11 8-8 6 6-8 8H7zM7 7l6 6M9 17h9'],
      ['spot', '污点修复', 'J', 'm3 13 10-10a3 3 0 0 1 4 4L7 17a3 3 0 0 1-4-4ZM7 7l6 6M9 9h.01M11 11h.01'],
      ['clone', '仿制图章', 'S', 'M5 17h10M4 14h12v-3H4zM8 11V8c-4-6 8-6 4 0v3'],
      ['heal', '修复画笔', 'R', 'M10 3v14M3 10h14M5 5l10 10M15 5 5 15'],
      ['dodge', '减淡', 'O', 'M12 12 5 19M16 7a5 5 0 1 1-10 0 5 5 0 0 1 10 0'],
      ['burn', '加深', 'U', 'M10 2c4 5-2 5 3 8 1-3 2-3 2-3 6 10-12 14-11 3 0-3 4-5 6-8'],
      ['sponge', '海绵', 'G', 'M3 5h14v10H3zM6 8h.01M10 7h.01M14 9h.01M8 12h.01M13 12h.01'],
      ['picker', '吸管', 'I', 'm4 12 9-9 4 4-9 9H4zM11 5l4 4M4 16l-2 2'],
    ];
    const svg = path => `<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="${path}"/></svg>`;
    const rail = document.createElement('nav'); rail.id = 'studio-tools'; rail.setAttribute('aria-label', '图像工具');
    rail.innerHTML = this.tools.map(([id, label, key, path]) => `<button data-tool="${id}" title="${label} (${key})" aria-label="${label}" aria-pressed="${id === 'hand'}">${svg(path)}</button>`).join('') + '<input id="studio-color" type="color" value="#e9b5ff" title="画笔颜色" aria-label="画笔颜色">';
    document.getElementById('app-shell').prepend(rail);
    rail.querySelectorAll('button').forEach(b => b.onclick = () => this.setTool(b.dataset.tool));
    const options = document.createElement('div'); options.id = 'studio-options';
    options.innerHTML = `<strong id="studio-tool-name">平移</strong><label>大小 <input id="studio-size" type="number" min="1" max="1000" value="35"> px</label><label>强度 <input id="studio-strength" type="range" min="1" max="100" value="100"></label><label>硬度 <input id="studio-hardness" type="range" min="0" max="100" value="60"></label><span id="studio-tool-hint">拖动画布平移 · 滚轮缩放</span>`;
    document.getElementById('workspace').prepend(options);
    const bar = document.createElement('div'); bar.id = 'studio-document';
    bar.innerHTML = '<span class="studio-document-dot"></span><span id="studio-document-name">未打开图像</span><span class="studio-spacer"></span><button id="studio-undo" title="撤销 Ctrl+Z">↶ 撤销</button><button id="studio-redo" title="重做 Ctrl+Shift+Z">↷</button><button id="studio-compare">按住对比</button><button id="studio-export" class="studio-primary">导出图片 ↗</button>';
    options.after(bar);
    const status = document.createElement('div'); status.id = 'studio-status';
    status.innerHTML = '<span id="studio-status-text">准备就绪 · 所有本地编辑均无需联网</span><span class="studio-spacer"></span><button id="studio-fit">适合画布</button><button id="studio-zoom">100%</button>';
    document.getElementById('canvas-stage').after(status);
    const panel = document.createElement('aside'); panel.id = 'studio-inspector'; panel.setAttribute('aria-label', '属性和图层');
    panel.innerHTML = `<div class="studio-panel-title">属性 <span>LOCAL STUDIO</span></div>
      <div class="studio-section"><h3>光线与色彩</h3><p>作用于所选图层，可限定选区</p>
      ${[['brightness','亮度',-100,100,1],['contrast','对比度',-100,100,1],['saturation','饱和度',-100,100,1],['exposure','曝光',-4,4,0.1]].map(([id,label,min,max,step])=>`<label class="studio-slider">${label}<output id="${id}-value">0</output><input id="studio-${id}" type="range" min="${min}" max="${max}" step="${step}" value="0"></label>`).join('')}
      <button id="studio-adjust" class="studio-wide">应用调整</button></div>
      <details class="studio-section"><summary>画布与滤镜</summary><div class="studio-grid"><button data-op="flip_h">水平翻转</button><button data-op="flip_v">垂直翻转</button><button data-op="rotate">旋转 90°</button><button id="studio-resize">调整尺寸</button><button data-op="blur">高斯模糊</button><button data-op="sharpen">锐化</button></div><label>滤镜半径 <input id="studio-radius" type="number" min="0.1" max="30" value="2" step="0.1"> px</label></details>
      <div class="studio-section studio-selection"><h3>选区</h3><p id="studio-selection-state">未设置 · 编辑整个图层</p><div class="studio-grid"><button id="studio-select">编辑选区</button><button id="studio-clear-selection">取消选区</button></div><button id="studio-fill" class="studio-wide">内容感知填充</button></div>
      <div class="studio-panel-title">图层 <span id="studio-layer-count">0</span></div><div class="studio-layer-actions"><button data-layer-op="layer_new" title="新建空白图层">＋</button><button data-layer-op="layer_duplicate" title="复制图层">复制</button><button data-layer-op="up" title="上移">↑</button><button data-layer-op="down" title="下移">↓</button><button data-layer-op="layer_delete" title="删除图层">删除</button></div>
      <div id="studio-layers"><p class="studio-empty">打开图片后开始编辑</p></div><div id="studio-layer-properties" class="studio-section" hidden><input id="studio-layer-name" aria-label="图层名称" maxlength="80"><label>混合 <select id="studio-blend"><option value="normal">正常</option><option value="multiply">正片叠底</option><option value="screen">滤色</option><option value="overlay">叠加</option></select></label><label>不透明度 <input id="studio-opacity" type="range" min="0" max="100" value="100"></label><label><input id="studio-lock" type="checkbox"> 锁定图层</label></div>`;
    document.getElementById('app-shell').append(panel);
    this.surface = document.createElement('div'); this.surface.id = 'studio-surface';
    this.image.before(this.surface); this.surface.append(this.image);
    this.overlay = document.createElement('canvas'); this.overlay.id = 'studio-overlay'; this.overlay.setAttribute('aria-label', '编辑画布'); this.surface.append(this.overlay);
    this.image.addEventListener('load', () => this.imageLoaded());
    new ResizeObserver(() => { if (this.autoFit) this.fit(); }).observe(this.wrap);
    this.bind(); this.setTool('hand');
  }
  el(id) { return document.getElementById('studio-' + id); }
  reset() { this.context=null;this.key=null;this.redo=[];this.layerId='';this.el('layers').replaceChildren();this.el('layer-properties').hidden=true;this.el('layer-count').textContent='0';this.el('document-name').textContent='未打开图像';this.hint('准备就绪 · 所有本地编辑均无需联网'); }
  hint(text) { this.el('status-text').textContent = text; }
  setTool(tool) {
    if (tool === 'select') { document.getElementById('btn-mask').click(); return; }
    this.tool = tool; this.el('tool-name').textContent = this.tools.find(t=>t[0]===tool)[1];
    document.querySelectorAll('#studio-tools [data-tool]').forEach(b=>b.setAttribute('aria-pressed', String(b.dataset.tool===tool)));
    this.el('tool-hint').textContent = ['clone','heal'].includes(tool) ? 'Alt + 点击设置采样点，再绘制笔画' : tool==='crop' ? '拖出裁剪区域，松开后确认' : tool==='hand' ? '拖动画布平移 · 滚轮缩放' : tool==='picker' ? '点击图像取色' : '在当前图层绘制 · 空格临时平移';
    this.overlay.style.cursor = tool==='hand' ? 'grab' : 'crosshair';
  }
  async setContext(session, node) {
    if(this.context && this.context.session.session_id!==session.session_id) this.redo=[];
    const key=session.session_id+'/'+node.id; const changed=this.key!==key;
    this.context={session,node}; this.key=key;
    this.el('document-name').textContent = (session.nodes[session.root_id]?.note || '图像') + ' · RGB/8';
    if (changed) { this.source=null; this.pan=[0,0]; this.autoFit=true; this.clearOverlay(); }
    this.el('undo').disabled=!node.parent_id; this.el('redo').disabled=!this.redo.length && !node.children?.length;
    const mask=this.bridge.mask(); this.el('selection-state').textContent=mask ? '选区已启用 · AI 与本地编辑共用' : '未设置 · 编辑整个图层';
    await this.refreshLayers();
  }
  imageLoaded() {
    if (!this.image.naturalWidth) return;
    const root=this.context?.session.nodes[this.context.session.root_id];
    const name=(root?.note||'图像').split(' · ')[0];
    this.el('document-name').textContent=`${name} · ${this.image.naturalWidth}×${this.image.naturalHeight} · RGB/8`;
    this.overlay.width=this.image.naturalWidth; this.overlay.height=this.image.naturalHeight;
    if(this.autoFit!==false) this.fit(); else this.transform();
    this.hint(`${this.image.naturalWidth} × ${this.image.naturalHeight} px · 自动保存 · ${this.context?.node.metadata?.local_operation ? '本地编辑' : '就绪'}`);
  }
  fit() {
    if (!this.image.naturalWidth) return;
    this.autoFit=true; this.scale=Math.min((this.wrap.clientWidth-48)/this.image.naturalWidth,(this.wrap.clientHeight-48)/this.image.naturalHeight,1); this.scale=Math.max(this.scale,0.01); this.pan=[0,0]; this.transform();
  }
  transform() {
    this.surface.style.width=this.image.naturalWidth*this.scale+'px'; this.surface.style.height=this.image.naturalHeight*this.scale+'px';
    this.surface.style.transform=`translate(calc(-50% + ${this.pan[0]}px), calc(-50% + ${this.pan[1]}px))`;
    this.el('zoom').textContent=Math.round(this.scale*100)+'%';
  }
  point(e) { const r=this.overlay.getBoundingClientRect(); return [(e.clientX-r.left)/this.scale,(e.clientY-r.top)/this.scale]; }
  clearOverlay() { this.overlay.getContext('2d').clearRect(0,0,this.overlay.width,this.overlay.height); }
  preview(points) {
    this.clearOverlay(); if (!points.length) return;
    const ctx=this.overlay.getContext('2d'); ctx.strokeStyle=this.el('color').value; ctx.fillStyle=ctx.strokeStyle;
    ctx.globalAlpha=0.6; ctx.lineWidth=Number(this.el('size').value); ctx.lineCap='round'; ctx.lineJoin='round'; ctx.beginPath();
    if(this.tool==='crop') { const a=points[0],b=points.at(-1); ctx.strokeStyle='#ffffff'; ctx.lineWidth=1/this.scale; ctx.strokeRect(a[0],a[1],b[0]-a[0],b[1]-a[1]); }
    else { ctx.moveTo(...points[0]); for(const p of points) ctx.lineTo(...p); ctx.stroke(); if(points.length===1){ctx.beginPath();ctx.arc(...points[0],ctx.lineWidth/2,0,Math.PI*2);ctx.fill();} }
    ctx.globalAlpha=1;
  }
  async apply(operation, params={}, points=[]) {
    if(this.busy || !this.context) return;
    const {session,node}=this.context;
    const key=this.key;
    if(node.status!=='done') return this.bridge.toast('请等待图像处理完成','error');
    this.busy=true; document.body.classList.add('studio-busy'); this.hint('正在本地处理并保存…');
    try {
      const result=await this.bridge.invoke('apply_local_edit',{session_id:session.session_id,node_id:node.id,request:{operation,layer_id:this.layerId,params,points,mask:this.bridge.mask()}});
      if(!result?.id) throw new Error('本地编辑需要在 CosKit 桌面应用中运行');
      if(this.key!==key) return;
      if(operation==='layer_new'||operation==='layer_import') this.layerId=result.layers?.at(-1)?.id||this.layerId;
      if(operation==='layer_duplicate'){const index=result.layers?.findIndex(l=>l.id===this.layerId);this.layerId=result.layers?.[index+1]?.id||this.layerId;}
      this.redo=[]; await this.bridge.accept(result.id,session.session_id,!['crop','resize','rotate','flip_h','flip_v'].includes(operation)); this.hint('已保存 · Ctrl+Z 撤销');
    } catch(e) { this.bridge.toast(String(e),'error'); this.hint('操作未完成 · 原图保留'); }
    finally { this.busy=false; document.body.classList.remove('studio-busy'); this.clearOverlay(); }
  }
  async refreshLayers() {
    if(!this.context) return;
    const key=this.key, {session,node}=this.context;
    const result=await this.bridge.invoke('get_layers',{session_id:session.session_id,node_id:node.id}); if(key!==this.key) return;
    this.layers=result.layers||[];
    if(!this.layers.some(l=>l.id===this.layerId)) this.layerId=this.layers.at(-1)?.id||'';
    this.el('layer-count').textContent=this.layers.length;
    const host=this.el('layers'); host.replaceChildren();
    [...this.layers].reverse().forEach(layer=>{
      const row=document.createElement('div'); row.className='studio-layer'+(layer.id===this.layerId?' selected':''); row.tabIndex=0;
      const eye=document.createElement('button'); eye.textContent=layer.visible?'◉':'○'; eye.title=layer.visible?'隐藏图层':'显示图层'; eye.setAttribute('aria-label',eye.title); eye.onclick=e=>{e.stopPropagation();this.layerId=layer.id;this.apply('layer_props',{visible:!layer.visible});};
      const img=document.createElement('img'); img.src=layer.thumb; img.alt='';
      const name=document.createElement('span'); name.textContent=layer.name||'图层';
      const lock=document.createElement('span'); lock.textContent=layer.locked?'锁定':''; lock.className='studio-layer-tag';
      row.append(eye,img,name,lock); row.onclick=()=>{this.layerId=layer.id;this.refreshLayers();}; row.onkeydown=e=>{if(e.key==='Enter')row.click();}; host.append(row);
    });
    const l=this.layers.find(l=>l.id===this.layerId); this.el('layer-properties').hidden=!l;
    if(l){this.el('layer-name').value=l.name;this.el('blend').value=l.blend_mode;this.el('opacity').value=l.opacity*100;this.el('lock').checked=l.locked;}
  }
  async navigate(redo=false) {
    if(this.busy || !this.context) return;
    const node=this.context.node;
    const id=redo ? (this.redo.pop()||node.children?.at(-1)) : node.parent_id;
    if(!id)return;
    if(!redo)this.redo.push(node.id);
    await this.bridge.navigate(id);
  }
  bind() {
    this.el('fit').onclick=()=>this.fit(); this.el('zoom').onclick=()=>{this.autoFit=false;this.scale=1;this.pan=[0,0];this.transform();};
    this.el('undo').onclick=()=>this.navigate(); this.el('redo').onclick=()=>this.navigate(true);
    this.el('export').onclick=async()=>{if(this.context)try{await this.bridge.invoke('export_image',{session_id:this.context.session.session_id,node_id:this.context.node.id});}catch(e){this.bridge.toast(String(e),'error');}};
    this.el('select').onclick=()=>document.getElementById('btn-mask').click();
    this.el('clear-selection').onclick=()=>{this.bridge.clearMask();this.el('selection-state').textContent='未设置 · 编辑整个图层';};
    this.el('fill').onclick=()=>this.apply('inpaint');
    for(const name of ['brightness','contrast','saturation','exposure']) this.el(name).oninput=()=>{document.getElementById(name+'-value').textContent=this.el(name).value;};
    this.el('adjust').onclick=async()=>{const p={};for(const name of ['brightness','contrast','saturation','exposure'])p[name]=Number(this.el(name).value);await this.apply('adjust',p);};
    document.querySelectorAll('[data-op]').forEach(b=>b.onclick=()=>this.apply(b.dataset.op,{angle:90,radius:Number(this.el('radius').value)}));
    this.el('resize').onclick=()=>this.resizeDialog();
    document.querySelectorAll('[data-layer-op]').forEach(b=>b.onclick=()=>{
      let op=b.dataset.layerOp,params={}; if(op==='up'||op==='down'){const i=this.layers?.findIndex(l=>l.id===this.layerId);params.index=Math.max(0,Math.min(this.layers.length-1,i+(op==='up'?1:-1)));op='layer_reorder';}this.apply(op,params);
    });
    this.el('layer-name').onchange=()=>this.apply('layer_props',{name:this.el('layer-name').value});
    this.el('blend').onchange=()=>this.apply('layer_props',{blend_mode:this.el('blend').value});
    this.el('opacity').onchange=()=>this.apply('layer_props',{opacity:Number(this.el('opacity').value)/100});
    this.el('lock').onchange=()=>this.apply('layer_props',{locked:this.el('lock').checked});
    this.wrap.addEventListener('wheel',e=>{if(!this.context)return;e.preventDefault();this.autoFit=false;this.scale=Math.max(0.01,Math.min(8,this.scale*Math.exp(-e.deltaY*0.001)));this.transform();},{passive:false});
    this.overlay.onpointerdown=e=>{
      if(this.busy||!this.context||this.context.node.status!=='done'||e.button!==0)return;
      e.preventDefault();this.overlay.setPointerCapture(e.pointerId);const point=this.point(e);
      window.getSelection()?.removeAllRanges();
      if(e.altKey&&['clone','heal'].includes(this.tool)){this.source=point;this.hint('采样点已设置');return;}
      if(this.tool==='picker'){const c=document.createElement('canvas');c.width=this.image.naturalWidth;c.height=this.image.naturalHeight;const ctx=c.getContext('2d');ctx.drawImage(this.image,0,0);const pixel=ctx.getImageData(Math.max(0,Math.min(c.width-1,point[0])),Math.max(0,Math.min(c.height-1,point[1])),1,1).data;this.el('color').value='#'+[...pixel].slice(0,3).map(v=>v.toString(16).padStart(2,'0')).join('');this.setTool('brush');return;}
      if(['clone','heal'].includes(this.tool)&&!this.source){this.bridge.toast('先按住 Alt 点击图像设置采样点');return;}
      this.drag={points:[point],screen:[e.clientX,e.clientY],pan:[...this.pan],hand:this.tool==='hand'||this.space};
      if(!this.drag.hand)this.preview(this.drag.points);
    };
    this.overlay.onpointermove=e=>{
      if(!this.drag){
        if(!this.busy && !['hand','crop','picker'].includes(this.tool)){
          this.clearOverlay();const ctx=this.overlay.getContext('2d'),p=this.point(e);ctx.beginPath();ctx.arc(...p,Number(this.el('size').value)/2,0,Math.PI*2);ctx.strokeStyle='#fff';ctx.lineWidth=1/this.scale;ctx.shadowColor='#000';ctx.shadowBlur=2/this.scale;ctx.stroke();ctx.shadowBlur=0;
        }
        return;
      }
      if(this.drag.hand){this.autoFit=false;this.pan=[this.drag.pan[0]+e.clientX-this.drag.screen[0],this.drag.pan[1]+e.clientY-this.drag.screen[1]];this.transform();}
      else {const p=this.point(e),last=this.drag.points.at(-1);if(Math.hypot(p[0]-last[0],p[1]-last[1])>=1&&this.drag.points.length<20000)this.drag.points.push(p);this.preview(this.drag.points);}
    };
    this.overlay.onpointerup=async()=>{
      const drag=this.drag;this.drag=null;if(!drag||drag.hand)return;
      if(this.tool==='crop'){
        const a=drag.points[0],b=drag.points.at(-1),w=this.image.naturalWidth,h=this.image.naturalHeight;
        const x=Math.floor(Math.max(0,Math.min(a[0],b[0]))),y=Math.floor(Math.max(0,Math.min(a[1],b[1])));
        const width=Math.floor(Math.min(w,Math.max(a[0],b[0]))-x),height=Math.floor(Math.min(h,Math.max(a[1],b[1]))-y);
        if(width<1||height<1){this.clearOverlay();return;}
        this.confirmCrop({x,y,width,height});return;
      }
      const p={size:Number(this.el('size').value),strength:Number(this.el('strength').value)/100,hardness:Number(this.el('hardness').value)/100,color:this.el('color').value};
      if(this.source){p.source_x=this.source[0];p.source_y=this.source[1];}
      await this.apply(this.tool,p,drag.points);
    };
    this.overlay.onpointercancel=()=>{this.drag=null;this.clearOverlay();};
    this.overlay.onpointerleave=()=>{if(!this.drag&&!this.busy)this.clearOverlay();};
    this.overlay.onclick=e=>e.stopPropagation(); this.image.onclick=e=>e.stopPropagation();
    document.addEventListener('keydown',e=>{
      if(/INPUT|TEXTAREA|SELECT/.test(e.target.tagName)||e.target.isContentEditable||document.querySelector('dialog[open]')||[...document.querySelectorAll('.modal-overlay')].some(el=>getComputedStyle(el).display!=='none'))return;
      if(e.code==='Space'){e.preventDefault();this.space=true;}
      if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='z'){e.preventDefault();this.navigate(e.shiftKey);return;}
      if(e.ctrlKey||e.metaKey||e.altKey)return;
      const t=this.tools.find(t=>t[2].toLowerCase()===e.key.toLowerCase());if(t)this.setTool(t[0]);
      if(e.key==='Escape'){this.drag=null;this.clearOverlay();this.setTool('hand');}
    });
    document.addEventListener('keyup',e=>{if(e.code==='Space')this.space=false;}); window.addEventListener('blur',()=>{this.space=false;this.drag=null;this.clearOverlay();});
    const compare=this.el('compare');
    compare.onpointerdown=async e=>{if(!this.context?.node.parent_id||this.busy)return;compare.setPointerCapture(e.pointerId);this.comparing=true;const key=this.key;const url=await this.bridge.invoke('get_image',{session_id:this.context.session.session_id,node_id:this.context.node.parent_id,thumbnail:false});if(this.comparing&&key===this.key){this.beforeSrc=this.image.src;this.image.src=url;}};
    const restore=()=>{this.comparing=false;if(this.beforeSrc){this.image.src=this.beforeSrc;this.beforeSrc=null;}};compare.onpointerup=restore;compare.onpointercancel=restore;compare.onlostpointercapture=restore;
  }
  dialog(title,content,apply) {
    const d=document.createElement('dialog');d.className='studio-dialog';d.innerHTML=`<form method="dialog"><h2>${title}</h2>${content}<div class="studio-dialog-actions"><button value="cancel">取消</button><button value="apply" class="studio-primary">应用</button></div></form>`;
    document.body.append(d);d.addEventListener('close',()=>{if(d.returnValue==='apply')apply(d);else this.clearOverlay();d.remove();});d.showModal();
  }
  confirmCrop(p){this.dialog('裁剪画布',`<p>将全部图层裁剪为 ${p.width} × ${p.height} px。可随时撤销。</p>`,()=>this.apply('crop',p));}
  resizeDialog(){if(!this.context)return;this.dialog('调整图像尺寸',`<label>宽度 <input name="width" type="number" min="1" max="20000" value="${this.image.naturalWidth}" required></label><label>高度 <input name="height" type="number" min="1" max="20000" value="${this.image.naturalHeight}" required></label><label><input name="ratio" type="checkbox" checked> 保持比例</label><p>使用 Lanczos 重采样；作用于全部图层。</p>`,d=>this.apply('resize',{width:Number(d.querySelector('[name=width]').value),height:Number(d.querySelector('[name=height]').value)}));const d=document.querySelector('dialog[open]'),w=d.querySelector('[name=width]'),h=d.querySelector('[name=height]'),ratio=this.image.naturalWidth/this.image.naturalHeight;w.oninput=()=>{if(d.querySelector('[name=ratio]').checked)h.value=Math.round(w.value/ratio);};h.oninput=()=>{if(d.querySelector('[name=ratio]').checked)w.value=Math.round(h.value*ratio);};}
}
