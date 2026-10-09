"""Small stdio MCP client used by the reproducible CosKit regression workflows.

No model credentials are read here. A bridge uses the desktop's private token file.
"""
import base64, json, pathlib, queue, subprocess, threading, time

ROOT = pathlib.Path(__file__).resolve().parents[1]

class MCP:
    def __init__(self, *, bridge=None, root=None, executable=None):
        exe = pathlib.Path(executable) if executable else ROOT/'native/target-release/release/photocraft-cli.exe'
        args = [str(exe), 'mcp']
        if bridge:
            args += ['--bridge',f'127.0.0.1:{bridge}','--control-token-file',str(ROOT/'.dev-data/cosplay-control.token')]
        else:
            root = pathlib.Path(root or ROOT/'test_output/cosplay').resolve()
            args += ['--automation-read-root',str(root),'--automation-write-root',str(root)]
        self.stderr = open(ROOT/'test_output/cosplay-mcp-stderr.log','a',encoding='utf-8')
        self.process = subprocess.Popen(args,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=self.stderr,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
        self.responses = queue.Queue(); self.counter = 0
        def read():
            for line in self.process.stdout:
                try: self.responses.put(json.loads(line))
                except ValueError: self.responses.put({'error':{'message':'non-JSON MCP stdout'}})
            self.responses.put({'error':{'message':'MCP process exited'}})
        threading.Thread(target=read,daemon=True).start()
        self.info = self.request('initialize',{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'coskit-regression','version':'1.0'}})
        self.notify('notifications/initialized',{})
    def notify(self,method,params):
        self.process.stdin.write((json.dumps({'jsonrpc':'2.0','method':method,'params':params})+'\n').encode());self.process.stdin.flush()
    def request(self,method,params,timeout=180):
        self.counter+=1; ident=self.counter
        self.process.stdin.write((json.dumps({'jsonrpc':'2.0','id':ident,'method':method,'params':params})+'\n').encode());self.process.stdin.flush()
        end=time.monotonic()+timeout
        while True:
            r=self.responses.get(timeout=max(0.1,end-time.monotonic()))
            if 'id' not in r and 'error' not in r: continue
            if 'error' in r: raise RuntimeError(r['error'])
            if r.get('id')!=ident: raise RuntimeError('MCP response id mismatch')
            return r['result']
    def tool(self,name,args=None,*,image_path=None,allow_error=False):
        r=self.request('tools/call',{'name':name,'arguments':args or {}})
        text='\n'.join(c['text'] for c in r.get('content',[]) if c['type']=='text')
        if r.get('isError'):
            if allow_error:return {'error':text}
            raise RuntimeError(f'{name}: {text}')
        if image_path:
            for c in r.get('content',[]):
                if c['type']=='image': pathlib.Path(image_path).write_bytes(base64.b64decode(c['data']))
        try:return json.loads(text)
        except ValueError:return text
    def command(self,ident,params=None,wait=True):return self.tool('command_run',{'id':ident,'params':params or {},'wait':wait})
    def control(self,method,params=None):return self.tool('control_call',{'method':method,'params':params or {}})
    def close(self):
        self.process.stdin.close()
        try:self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:self.process.terminate();self.process.wait(timeout=5)
        self.stderr.close()
    def __enter__(self):return self
    def __exit__(self,*_):self.close()
