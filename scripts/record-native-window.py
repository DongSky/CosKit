"""Capture real rendered editor frames with timestamps, without the desktop/taskbar."""
import json, pathlib, subprocess, threading, time
from coskit_mcp_client import MCP

class WindowRecorder:
    def __init__(self, directory, port):
        self.directory=pathlib.Path(directory);self.port=port
        self.frames=self.directory/'frames';self.frames.mkdir(exist_ok=True)
        self.stop_event=threading.Event();self.ready=threading.Event();self.error=None;self.samples=[];self.t0=time.monotonic()
    def start(self):
        self.thread=threading.Thread(target=self.capture,daemon=True);self.thread.start()
        if not self.ready.wait(20):raise RuntimeError('Window capture did not start')
        if self.error:raise self.error
    def capture(self):
        try:
            with MCP(bridge=self.port) as m:
                self.t0=time.monotonic()
                while not self.stop_event.is_set():
                    tick=time.monotonic();name=f'frame-{len(self.samples):06}.png'
                    m.control('ui.screenshot',{'path':'recordings/frames/'+name,'focus':False})
                    self.samples.append({'file':name,'time':tick-self.t0})
                    self.ready.set()
                    self.stop_event.wait(max(0,.125-(time.monotonic()-tick)))
        except Exception as e:self.error=e;self.ready.set()
    def stop(self):
        self.stop_event.set();self.thread.join(timeout=30)
        if self.thread.is_alive():raise RuntimeError('Capture thread did not stop')
        if self.error:raise self.error
        if len(self.samples)<2:raise RuntimeError('No usable window frames')
        (self.directory/'frame-timestamps.json').write_text(json.dumps(self.samples,indent=2),encoding='utf-8')
        lines=[]
        for i,s in enumerate(self.samples):
            duration=self.samples[i+1]['time']-s['time'] if i+1<len(self.samples) else .125
            lines.extend([f"file 'frames/{s['file']}'",f'duration {duration:.6f}'])
        lines.append(f"file 'frames/{self.samples[-1]['file']}'")
        playlist=self.directory/'frames.txt';playlist.write_text('\n'.join(lines),encoding='utf-8')
        log=(self.directory/'ffmpeg.log').open('w',encoding='utf-8')
        subprocess.run(['ffmpeg','-hide_banner','-y','-f','concat','-safe','0','-i',str(playlist),'-vf','fps=30',
            '-c:v','libx264','-preset','fast','-crf','18','-pix_fmt','yuv420p','-map_metadata','-1',str(self.directory/'window-raw.mkv')],check=True,stdout=log,stderr=log)
        log.close()
