"""Record a real CosKit desktop walkthrough; all personal media stays outside Git.

Requires a locally configured AI provider, FFmpeg, Pillow and a built native editor.
Never prints credentials. The model receives the selected photo and goal.
"""
import argparse, hashlib, importlib.util, json, os, pathlib, shutil, subprocess, time
from PIL import Image, ImageOps
from coskit_mcp_client import MCP, ROOT

spec=importlib.util.spec_from_file_location('window_recorder',pathlib.Path(__file__).with_name('record-native-window.py'))
recorder_module=importlib.util.module_from_spec(spec);spec.loader.exec_module(recorder_module)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--ai-data', type=pathlib.Path, required=True)
    parser.add_argument('--port', type=int, default=50531)
    parser.add_argument('--advanced', action='store_true', help='Full background, lighting and skin-retouch demo')
    parser.add_argument('--prompt-file', type=pathlib.Path, help='UTF-8 creative brief, including verified character context')
    args = parser.parse_args()
    if args.advanced and not args.prompt_file:
        parser.error("--advanced requires --prompt-file with a verified creative brief")
    out = args.output.resolve()
    project_path = 'exports/雪霁·庭前试刀.ckpipe' if args.advanced else 'exports/花间·演示.ckpipe'
    for folder in ('originals','screenshots','recordings','exports','video'):
        (out/folder).mkdir(parents=True, exist_ok=True)
    original = ImageOps.exif_transpose(Image.open(args.image)).convert('RGB')
    # A fresh raster removes EXIF, GPS, comments and private original filenames.
    clean = Image.new('RGB', original.size); clean.paste(original)
    clean.save(out/'originals/source.png')
    shutil.copy2(ROOT/'native/target-release/release/photocraft.exe',out/'coskit-demo.exe')
    env = os.environ.copy()
    env['COSKIT_DATA_DIR'] = str(args.ai_data.resolve())
    env['COSKIT_NATIVE_CONFIG_DIR'] = str(ROOT/'.dev-data'/f'demo-v1-{int(time.time())}')
    log = (out/'recordings/native.log').open('w',encoding='utf-8')
    app = subprocess.Popen([str(out/'coskit-demo.exe'),'--control',str(args.port),
        '--control-token-file',str(ROOT/'.dev-data/cosplay-control.token'),
        '--automation-read-root',str(out),'--automation-write-root',str(out)],env=env,stdout=log,stderr=log)
    recorder = None
    report = {'image_size':list(clean.size),'source_sha256':hashlib.sha256(args.image.read_bytes()).hexdigest(),'chapters':[]}
    try:
        time.sleep(4)
        with MCP(bridge=args.port) as m:
            m.control('ui.resize',{'width':1600,'height':880})
            m.control('ui.focus')
            time.sleep(1)
            recorder=recorder_module.WindowRecorder(out/'recordings',args.port)
            recorder.start()
            start=recorder.t0
            def mark(name):
                report['chapters'].append({'name':name,'time':round(time.monotonic()-start,3)})
                (out/'recordings/timeline.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
                print(name,flush=True)
            def shot(name):
                m.tool('ui_screenshot',{'max_side':1600},image_path=out/'screenshots'/f'{name}.png')
            mark('open')
            time.sleep(3)
            m.tool('doc_open',{'path':'originals/source.png'})
            m.control('ui.set',{'fit':True})
            m.command('coskit.project.checkpoint',{'label':'原片 · 花间'})
            m.control('app.save',{'path':project_path})
            time.sleep(5);shot('before')
            mark('ai')
            m.tool('ai_configure',{'harness_enabled':True,'harness_max_steps':24 if args.advanced else 16,'harness_max_images':4 if args.advanced else 0,'harness_max_rollbacks':3})
            prompt='请把这张花树下的 cosplay 人像修成清新通透、自然柔和的成片。适度改善人物肤色与白衣亮度，保留皮肤质感、银蓝发丝、白花高光和深蓝衣裙层次，背景绿叶稍微柔和一些。保持人物身份、五官、服装、花朵、武器、构图和画布尺寸，不增删物体，不磨皮成塑料，不要改动五官。你自行分析并决定调整与验收步骤。'
            if args.prompt_file:
                prompt=args.prompt_file.read_text(encoding='utf-8').strip()
            (out/'recordings/demo-goal.txt').write_text(prompt,encoding='utf-8')
            m.tool('ai_edit',{'prompt':prompt})
            last=0
            while time.monotonic()-start<(2100 if args.advanced else 950):
                status=m.tool('ai_status')
                if not status['busy']:break
                elapsed=int(time.monotonic()-start)
                if elapsed-last>=30:print('AI working:',elapsed,'s',flush=True);last=elapsed
                time.sleep(2)
            else:
                m.tool('ai_cancel');raise RuntimeError('AI timed out; no unreviewed result committed')
            (out/'recordings/ai-status.private.json').write_text(json.dumps(status,ensure_ascii=False,indent=2),encoding='utf-8')
            report['ai_outcome']=status['outcome']
            if status['outcome']!='applied':raise RuntimeError('AI did not apply a reviewed edit; inspect local private status')
            mark('review')
            m.command('coskit.project.checkpoint',{'label':'AI · 换景 / 调色 / 自然精修' if args.advanced else 'AI · 通透调色'})
            time.sleep(5);shot('ai-review')
            m.tool('doc_save',{'path':'exports/ai.png'})
            time.sleep(5)
            mark('brush')
            m.control('ui.set',{'studio':{'inspector':'layers'}})
            m.command('layer.new.layer',{'name':'手绘 · 晨光' if args.advanced else '手绘 · 花间柔光'})
            # Original-photo coordinates. Soft light is placed among flowers, away from facial features.
            scale=clean.width/1620
            points=[(1110,330),(1130,305),(1150,280),(1170,255),(1190,235),(1210,220)]
            for index,(x,y) in enumerate(points):
                m.tool('brush_stroke',{'preset':'Soft Round','points':[{'x':x*scale,'y':y*scale,'pressure':.35}],
                    'size':180*scale,'color':'#fff0d0','flow':.08,'opacity':.12,'seed':100+index})
                time.sleep(.45)
            m.command('coskit.project.checkpoint',{'label':'笔刷 · 晨光' if args.advanced else '笔刷 · 花间柔光'})
            time.sleep(4);shot('brush')
            mark('parameters')
            m.command('image.adjustments.curves',{'points':[[0,0],[64,66],[128,130],[192,194],[255,255]]})
            project=m.command('coskit.project.inspect')['project']
            m.control('ui.project',{'parameters':True,'operation':len(project['operations'])-1,'version':'v1'})
            time.sleep(7);shot('parameters')
            m.control('ui.project',{'parameters':False})
            m.command('coskit.project.checkpoint',{'label':'成片 · 细节确认'})
            mark('compare')
            m.control('ui.project',{'version':'v1','compare':True,'zoom':0})
            time.sleep(6);shot('comparison')
            m.control('ui.project',{'version':'v1','compare':True,'zoom':1,'center':[960*scale,560*scale]})
            time.sleep(6);shot('detail')
            m.control('ui.project',{'zoom':2})
            time.sleep(4)
            m.control('ui.project',{'zoom':1,'difference':True})
            time.sleep(4);shot('difference')
            m.control('ui.project',{'compare':False,'difference':False})
            mark('save')
            m.control('ui.set',{'studio':{'inspector':'ai'},'fit':True})
            info=m.tool('doc_inspect')
            assert (info['width'],info['height'])==clean.size
            m.control('app.save',{'path':project_path})
            m.tool('doc_save',{'path':'exports/after.png'})
            time.sleep(6);shot('workspace')
            report['final_document']={k:info[k] for k in ('width','height')}
            # Save/reopen and verify decoded pixels, rather than relying on file existence.
            m.tool('doc_open',{'path':project_path})
            m.tool('doc_save',{'path':'exports/reopened.png'})
            from PIL import ImageChops
            assert ImageChops.difference(Image.open(out/'exports/after.png'),Image.open(out/'exports/reopened.png')).getbbox() is None
            report['reopen_pixels_match']=True
            mark('end')
            recorder.stop();recorder=None
            m.command('file.closeAll');m.control('app.quit')
        (out/'recordings/validation.private.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
        print('Demo complete; geometry and reopened pixels verified.',flush=True)
    finally:
        if recorder:
            recorder.stop_event.set();recorder.thread.join(timeout=30)
        try:app.wait(timeout=8)
        except subprocess.TimeoutExpired:app.terminate();app.wait(timeout=5)
        log.close()

if __name__=='__main__':main()
