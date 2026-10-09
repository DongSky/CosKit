"""Drive the real native window through its authenticated local test protocol.

Use an isolated data directory and an automation read/write root of test_output.
--ai submits a real model request using the user's configured provider.
"""
import argparse,json,socket,time,sys
sys.stdout.reconfigure(encoding='utf-8')
from pathlib import Path
p=argparse.ArgumentParser()
p.add_argument('--port',type=int,default=50497)
p.add_argument('--ai',action='store_true')
p.add_argument('--inspect',action='store_true')
p.add_argument('--quit',action='store_true')
p.add_argument('--reopen-ai',action='store_true')
a=p.parse_args()
root=Path(__file__).resolve().parents[1]
token=(root/'.dev-data/native-control.token').read_text().strip()
sock=socket.create_connection(('127.0.0.1',a.port),timeout=60)
f=sock.makefile('rwb')
counter=0
def call(method,params=None):
    global counter
    counter+=1
    f.write((json.dumps({'id':counter,'method':method,'params':params or {}})+'\n').encode());f.flush()
    data=json.loads(f.readline())
    if not data.get('ok'):raise RuntimeError(f'{method}: {data.get("error")}')
    return data.get('result')
call('auth',{'token':token})
if a.quit:
    call('app.quit');print('Native smoke application closed.');raise SystemExit
def execute(command,params=None):return call('engine.execute',{'command':command,'params':params or {}})
if a.reopen_ai:
    call('app.open',{'path':'native-ai-result.pcraft'})
    document=execute('document.inspect')
    assert len(document['layers'])==3,document
    assert document['depth']==8,document
    assert call('coskit.ai.inspect')['messages']>=2,'Conversation history was not restored'
    call('ui.set',{'theme':'pro','fit':True})
    call('ui.resize',{'width':1440,'height':960})
    call('ui.screenshot',{'path':'native-studio-final.png'})
    call('ui.resize',{'width':960,'height':720})
    call('ui.screenshot',{'path':'native-ai-narrow-final.png'})
    call('ui.resize',{'width':1440,'height':960})
    print('PASS: final executable reopened the real AI layered document and persistent conversation.')
    raise SystemExit
if a.inspect:
    print(json.dumps({'ai':call('coskit.ai.inspect'),'document':execute('document.inspect')},ensure_ascii=False));raise SystemExit
if a.ai:
    execute('file.new',{'width':512,'height':512,'name':'CosKit AI synthetic smoke','background':'white'})
    execute('layer.new.layer',{'name':'Synthetic red shape'})
    execute('paint.stroke',{'points':[[160,256,1],[350,256,1]],'size':130,'color':'#ff0000'})
    call('ui.set',{'fit':True})
    before=execute('document.inspect')
    call('coskit.ai.submit',{'prompt':'仅把白色背景上的红色形状改成蓝色，保持其位置和形状不变，不要添加任何文字或其他元素。'})
    print('Real AI request submitted for synthetic geometry; no personal image used.',flush=True)
    deadline=time.monotonic()+720
    status=None
    while time.monotonic()<deadline:
        current=call('coskit.ai.inspect')
        if current.get('status')!=status:
            status=current.get('status');print(status,flush=True)
        if not current['busy']:
            assert not current['pending'],current
            after=execute('document.inspect')
            assert len(after['layers'])==len(before['layers'])+1,current
            call('app.save',{'path':'native-ai-result.pcraft'})
            call('app.save',{'path':'native-ai-result.png'})
            call('ui.screenshot',{'path':'native-ai-result-ui.png'})
            execute('edit.undo')
            assert len(execute('document.inspect')['layers'])==len(before['layers'])
            execute('edit.redo')
            print('PASS: real AI -> native layer -> save -> undo -> redo',flush=True)
            break
        time.sleep(2)
    else:raise TimeoutError('AI did not finish within smoke test deadline')
else:
    commands=call('engine.commands')
    (root/'test_output/native-command-registry.json').write_text(json.dumps(commands,ensure_ascii=False,indent=2),encoding='utf-8')
    menus=call('ui.menu.list')
    (root/'test_output/native-menu-tree.json').write_text(json.dumps(menus,ensure_ascii=False,indent=2),encoding='utf-8')
    execute('file.new',{'width':640,'height':480,'depth':16,'name':'CosKit complete editor smoke','background':'white'})
    execute('layer.new.layer',{'name':'Brush layer'})
    execute('paint.stroke',{'points':[[100,180,1],[360,220,0.8]],'size':75,'color':'#e74f6a'})
    execute('layer.newAdjustmentLayer.hueSaturation',{'hue':20,'saturation':10})
    before=execute('document.inspect')
    call('app.save',{'path':'native-layered-smoke.pcraft'})
    call('app.save',{'path':'native-layered-smoke.psd'})
    call('app.open',{'path':'native-layered-smoke.psd'})
    after=execute('document.inspect')
    assert after['depth']==16,(before,after)
    assert len(after['layers'])==len(before['layers']),(before,after)
    call('ui.set',{'fit':True})
    for theme in ['pro','proMedium','studio','studioLight','classic']:
        call('ui.set',{'theme':theme})
        call('ui.screenshot',{'path':f'native-{theme}.png'})
    call('ui.set',{'theme':'pro'})
    call('ui.resize',{'width':960,'height':720})
    call('ui.screenshot',{'path':'native-narrow.png'})
    call('ui.resize',{'width':1440,'height':960})
    call('ui.screenshot',{'path':'native-studio.png'})
    print('PASS: native 16-bit brush + adjustment layers + layered PSD save/reopen + 5 themes + narrow layout',flush=True)
sock.close()
