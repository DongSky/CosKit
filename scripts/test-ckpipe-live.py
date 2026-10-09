"""Full-size CKPipe save/reopen/branch/parameter checks through the shipped MCP server."""
import json,time,argparse
from pathlib import Path
from PIL import Image,ImageChops
from coskit_mcp_client import MCP,ROOT
parser=argparse.ArgumentParser();parser.add_argument('--executable');args=parser.parse_args()
out=ROOT/'test_output/harness'
def equal(a,b):
    assert ImageChops.difference(Image.open(out/a).convert('RGBA'),Image.open(out/b).convert('RGBA')).getbbox(alpha_only=False) is None
report={}
with MCP(root=out,executable=args.executable) as m:
    m.tool('doc_open',{'path':'reviewed.pcraft'})
    doc=m.tool('doc_inspect')
    assert (doc['width'],doc['height'])==(5472,3648)
    status=json.loads((out/'skin-live-status.json').read_text(encoding='utf-8'))
    info=json.loads((out/'skin-live-before.json').read_text(encoding='utf-8'))
    m.command('coskit.project.annotate',{'document':json.loads(__import__('zipfile').ZipFile(out/'reviewed.pcraft').read('manifest.json'))['document']['id'],'messages':[{'document':'Cosplay','role':'你','text':info['prompt'],'conversation':1},{'document':'Cosplay','role':'CosKit','text':'模型审核通过的候选；最初自动提交因 ICC 缓存比较被保护性拦截，本工程由已保存候选打开。','conversation':1}],'run':{'events':status['harness'],'original_outcome':status['outcome']}})
    m.command('layer.new.layer',{'name':'CKPipe brush parameter test'})
    base=m.command('coskit.project.checkpoint',{'label':'Reviewed image before brush test'})['version']
    m.tool('doc_save',{'path':'ckpipe-before.png'})
    m.tool('brush_stroke',{'points':[{'x':3400,'y':700,'pressure':0.1},{'x':3440,'y':725,'pressure':0.6},{'x':3480,'y':760,'pressure':0.2}],'size':7,'flow':0.2,'opacity':0.4,'preset':'CosKit · Hair Single Strand','color':'#88ccee','seed':41})
    first=m.command('coskit.project.checkpoint',{'label':'Controlled hair brush test'})['version']
    m.tool('doc_save',{'path':'ckpipe-stroke.png'})
    state=m.command('coskit.project.inspect')['project']
    op=next(o for o in reversed(state['operations']) if o['command']=='paint.stroke')
    op['params']['flow']=0.08
    second=m.command('coskit.project.replay',{'version':base,'steps':[{'command':'paint.stroke','params':op['params'],'active_layer':op['active_layer']}]})['version']
    m.tool('doc_save',{'path':'ckpipe-replayed.png'})
    m.command('coskit.project.restore',{'version':base})
    m.tool('doc_save',{'path':'ckpipe-restored.png'})
    equal('ckpipe-before.png','ckpipe-restored.png')
    t=time.monotonic();m.tool('doc_save',{'path':'cosplay-project.ckpipe'});report['save_seconds']=round(time.monotonic()-t,3)
    before=m.command('coskit.project.inspect')
    m.command('image.adjustments.invert')
    m.tool('doc_save',{'path':'ckpipe-second-save.ckpipe'})
    saved=m.command('coskit.project.inspect')
    assert len(saved['project']['versions'])>len(before['project']['versions'])
    m.tool('doc_open',{'path':'cosplay-project.ckpipe'})
    after=m.command('coskit.project.inspect')
    (out/'ckpipe-before-state.json').write_text(json.dumps(before,ensure_ascii=False),encoding='utf-8')
    (out/'ckpipe-after-state.json').write_text(json.dumps(after,ensure_ascii=False),encoding='utf-8')
    assert after['project']==before['project']
    m.tool('doc_save',{'path':'ckpipe-reopened.png'})
    equal('ckpipe-before.png','ckpipe-reopened.png')
    m.command('coskit.project.restore',{'version':first})
    m.tool('doc_save',{'path':'ckpipe-reopened-stroke.png'})
    equal('ckpipe-stroke.png','ckpipe-reopened-stroke.png')
    m.command('coskit.project.restore',{'version':second})
    m.tool('doc_save',{'path':'ckpipe-reopened-replay.png'})
    equal('ckpipe-replayed.png','ckpipe-reopened-replay.png')
    m.command('coskit.project.restore',{'version':base})
    for ext in ['psd','png','jpg']:m.tool('doc_save',{'path':'cosplay-project.'+ext})
    report.update({'width':after['width'],'height':after['height'],'versions':len(after['project']['versions']),'operations':len(after['project']['operations']),'conversations':len(after['project']['conversations']),'runs':len(after['project']['runs']),'file_bytes':(out/'cosplay-project.ckpipe').stat().st_size,'verified':['full-resolution layered roundtrip','project conversations and trace','branch version restore pixel equality','brush size flow pressure seed replay','second-save persistent version','PSD PNG JPG export']})
(out/'ckpipe-validation.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps(report,ensure_ascii=False,indent=2))
