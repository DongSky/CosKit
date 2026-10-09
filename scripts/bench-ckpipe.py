"""Compare identical CKPipe workloads through a chosen shipped MCP executable."""
import argparse,json,time,threading,platform
import psutil
from pathlib import Path
from coskit_mcp_client import MCP,ROOT
p=argparse.ArgumentParser();p.add_argument('--executable',required=True);p.add_argument('--label',required=True);a=p.parse_args()
out=ROOT/'test_output'/'p0';out.mkdir(exist_ok=True)
report={'label':a.label,'stages':[], 'machine':{'os':platform.platform(),'logical_cpus':psutil.cpu_count(),'ram_bytes':psutil.virtual_memory().total}}
with MCP(root=ROOT/'test_output',executable=a.executable) as m:
 proc=psutil.Process(m.process.pid)
 peaks={'stage':0,'process':0};stop=threading.Event()
 def sample():
  while not stop.is_set():
   try:
    rss=proc.memory_info().rss
    peaks['stage']=max(peaks['stage'],rss);peaks['process']=max(peaks['process'],rss)
   except psutil.Error:break
   stop.wait(0.02)
 sampler=threading.Thread(target=sample,daemon=True);sampler.start()
 def run(label,name,args):
  peaks['stage']=proc.memory_info().rss
  t=time.perf_counter();v=m.tool(name,args);report['stages'].append({'stage':label,'seconds':round(time.perf_counter()-t,4),'peak_rss_bytes':peaks['stage']});return v
 run('open','doc_open',{'path':'harness/reviewed.pcraft'})
 report['document']=m.tool('doc_inspect')
 # Fixed sparse strokes leave almost every full-resolution tile unchanged.
 run('layer','command_run',{'id':'layer.new.layer','params':{'name':'P0 deterministic strokes'}})
 for i in range(6):
  run(f'stroke_{i}','brush_stroke',{'points':[{'x':3000+i*9,'y':800,'pressure':0.5},{'x':3100+i*9,'y':850,'pressure':0.6}], 'size':8,'flow':0.2,'opacity':0.4,'color':'#88ccee','seed':41})
  run(f'checkpoint_{i}','command_run',{'id':'coskit.project.checkpoint','params':{'label':f'P0-{i}'}})
 target=f'p0/{a.label}.ckpipe'
 run('save','doc_save',{'path':target})
 report['file_bytes']=(ROOT/'test_output'/target).stat().st_size
 run('reopen','doc_open',{'path':target})
 run('restore','command_run',{'id':'coskit.project.restore','params':{'version':'v1'}})
 report['restored']=m.tool('doc_inspect')
 stop.set();sampler.join();report['process_peak_rss_bytes']=peaks['process']
(out/f'{a.label}.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps({'file_bytes':report['file_bytes'],'stages':report['stages']}))
