"""Build metadata-free local promo images and a captioned tutorial from real footage.

Usage: python scripts/render-release-media.py --media /path/to/local/demo
Requires Pillow, FFmpeg, and Windows System.Speech with a Chinese voice.
"""
import argparse, json, math, pathlib, shutil, subprocess, textwrap, wave
from PIL import Image, ImageDraw, ImageFont

ROOT=pathlib.Path(__file__).resolve().parents[1]
BG=(6,10,7);FG=(222,241,226);GREEN=(57,255,110);MUTED=(146,174,153)

def run(args):
    subprocess.run(args,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)

def font(size,mono=False):
    return ImageFont.truetype('C:/Windows/Fonts/consola.ttf' if mono else 'C:/Windows/Fonts/msyh.ttc',size)

def fresh(path):
    src=Image.open(path).convert('RGB'); out=Image.new('RGB',src.size);out.paste(src);return out

def main():
    p=argparse.ArgumentParser();p.add_argument('--media',type=pathlib.Path,required=True);a=p.parse_args();base=a.media.resolve()
    web=base/'site-package/coskit/media';web.mkdir(parents=True,exist_ok=True)
    video=base/'video';video.mkdir(exist_ok=True)
    images={'before':base/'originals/source.png','after':base/'exports/after.png',**{n:base/'screenshots'/f'{n}.png' for n in ['workspace','ai-review','brush','parameters','detail']}}
    for name,path in images.items():fresh(path).save(web/f'{name}.webp',quality=90,method=6)
    before=fresh(images['before']);after=fresh(images['after'])
    compare=Image.new('RGB',(before.width*2,before.height+70),BG)
    compare.paste(before,(0,70));compare.paste(after,(before.width,70));d=ImageDraw.Draw(compare)
    d.text((25,16),'BEFORE / 原片',font=font(28),fill=FG);d.text((before.width+25,16),'AFTER / 演示成片',font=font(28),fill=GREEN)
    compare.save(base/'exports/before-after.jpg',quality=95)
    def card(title,subtitle,name):
        image=Image.new('RGB',(1920,1080),BG);d=ImageDraw.Draw(image)
        for y in range(0,1080,40):d.line((0,y,1920,y),fill=(12,23,16))
        d.text((85,65),'> PRTS.si / COSKIT',font=font(24,True),fill=GREEN)
        d.text((85,230),title,font=font(72),fill=FG,spacing=15)
        d.text((88,465),'v1.0.0',font=font(82,True),fill=GREEN)
        for i,line in enumerate(subtitle.split('\n')):d.text((90,610+i*48),line,font=font(28),fill=MUTED)
        photo=after.resize((960,640),Image.Resampling.LANCZOS);image.paste(photo,(865,215));d.rectangle((864,214,1826,856),outline=(46,90,58),width=2)
        d.text((90,968),'凉月 / Suzutsuki   ·   真实窗口演示 / REAL APP CAPTURE',font=font(24),fill=MUTED)
        image.save(video/f'{name}.png');return image
    card('对话与画笔，\n同一张画布。','功能介绍 + 新手教程\n从原片，到可继续编辑的成片','intro').resize((1200,675)).save(web/'tutorial-poster.jpg',quality=93)
    card('保留每一次尝试。','保存 .ckpipe 留下完整工程\n导出 PNG / JPEG 分享成片','outro')
    fresh(web/'tutorial-poster.jpg').resize((1200,630)).save(web/'social-card.jpg',quality=92)
    chapters=json.loads((base/'recordings/timeline.json').read_text(encoding='utf-8'))['chapters'];times={x['name']:x['time'] for x in chapters}
    script=[
      ('intro','CosKit 1.0.0','第一个正式版本 · 对话修图 + 原生编辑','欢迎使用 Cos Kit 一点零。这是第一个正式版本。接下来，用一张花间人像，看一次从对话修图到手工收尾的完整流程。','Welcome to CosKit 1.0, the first stable release. Follow a floral portrait from conversational editing to manual finishing.'),
      ('open','01 / 打开照片，留下原片','Ctrl+O 打开 · 另存为 .ckpipe · 记录基准','先打开照片，确认画布尺寸。另存为 CK Pipe 工程，再给起始版本命名。这样，后续的图层、参数、对话和版本，都能保存在同一个工程里。','Open the photo and check its dimensions. Save a CKPipe project and name a baseline. Layers, parameters, conversations and versions stay together.'),
      ('ai','02 / 只描述你想要的效果','先在「选项」配置模型 · 不需要指定具体工作流','使用对话前，在选项中配置自己的模型。这里只描述目标：让人物清新通透，同时保留肤质、白衣和花朵的层次。模型自己分析步骤，发现工具并执行。这一段等待已经加速。','Configure your models in Options. Describe the desired result, preserving skin, white fabric and flowers. The model discovers tools and plans the steps. Waiting is accelerated here.'),
      ('review','03 / 结果还需要细看','独立审核 · 原尺寸局部 · 不通过就回退','调整完成后，模型还会检查全图和原尺寸局部。你也可以展开过程记录，了解做了什么。审核通过后，结果才进入工程；是否满意，仍由你决定。','The reviewer inspects the whole image and native-size crops. Expand the trace to see the actions. Approved results enter the project; the final creative judgment remains yours.'),
      ('brush','04 / 在独立图层上收尾','低流量柔笔刷 · 克制地补一点花间光感','接下来新建图层，用低流量的柔笔刷，在花朵附近补一点柔光。手工收尾和 AI 使用同一张画布。独立图层让你随时降低强度、隐藏，或者撤销。','Create a separate layer and add restrained soft light near the flowers with a low-flow brush. Manual work shares the AI canvas. Reduce, hide or undo the layer whenever needed.'),
      ('parameters','05 / 看得懂，也改得动','版本条「参数」 · 曲线、滑杆 · 明确选择基准','点开版本条的参数，可以直接看曲线和滑杆。需要重放时，先明确选择基准和编辑记录。重放成功会生成新分支，原来的版本仍然保留。','Open Parameters in the version strip for curves and sliders. Choose the baseline and operation before replaying. A successful replay creates a branch and retains earlier versions.'),
      ('compare','06 / 从全图检查到发丝细节','对比 → 100% → 200% → 差异图','最后对比原片和成片。左右同步缩放，检查脸部、发丝与衣料。百分之百看原尺寸，百分之二百放大，差异图帮助定位变化。浏览对比不会修改画布。','Compare the original and result with synchronized views. Inspect the face, hair and fabric at 100% and 200%. Difference view locates changes. Comparison does not modify the canvas.'),
      ('save','07 / 工程与成片，分别保存','保存 .ckpipe · 导出 PNG / JPEG','保存 CK Pipe 保留图层、对话和版本。需要分享时，再导出 PNG 或 JPEG。演示最后重新打开了工程，画布尺寸和导出的像素都与保存前一致。','Save CKPipe for layers, conversation and versions. Export PNG or JPEG for sharing. The demo reopens the project and verifies unchanged dimensions and exported pixels.'),
      ('outro','CosKit 1.0.0','开源创作工作台 · 凉月 / Suzutsuki','从一个目标开始，也随时拿回画笔。Cos Kit 一点零，期待你的下一张作品。','Start with a goal, and pick up the brush whenever you want. CosKit 1.0 is ready for your next creative project.')]
    segments=[dict(zip(['id','title','caption','zh','en'],x)) for x in script]
    (video/'storyboard.json').write_text(json.dumps(segments,ensure_ascii=False,indent=2),encoding='utf-8')
    subprocess.run(['powershell','-NoProfile','-File',str(ROOT/'scripts/synthesize-release-narration.ps1'),'-Directory',str(video)],check=True)
    clock=0;subs={'zh':[],'en':[]};edits=[]
    def stamp(t):
        ms=round(t*1000);return f'{ms//3600000:02}:{ms//60000%60:02}:{ms//1000%60:02}.{ms%1000:03}'
    for s in segments:
        with wave.open(str(video/(s['id']+'.wav'))) as w:duration=w.getnframes()/w.getframerate()+1.5
        duration=max(duration,8)
        if s['id'] in ('intro','outro'):
            inputs=['-loop','1','-i',str(video/(s['id']+'.png'))];vf=f'fps=30,format=yuv420p,fade=t=in:st=0:d=0.25,fade=t=out:st={duration-.25}:d=0.25'
            filters=['-vf',vf,'-map','0:v'];speed=1
        else:
            idx=next(i for i,x in enumerate(chapters) if x['name']==s['id']);start=times[s['id']];length=max(.5,chapters[idx+1]['time']-start-.75)
            speed=max(1,length/duration)
            overlay=Image.new('RGBA',(1920,1080),(0,0,0,0));d=ImageDraw.Draw(overlay)
            d.text((54,20),s['title'],font=font(28),fill=FG)
            d.text((1490,24),'COSKIT / 1.0.0',font=font(22,True),fill=GREEN)
            d.rectangle((191,79,1729,925),outline=(45,75,52),width=1)
            d.text((192,948),s['caption'],font=font(28),fill=FG)
            label=f'真实窗口 · 等待 / 操作加速 {speed:.1f}×' if speed>1.02 else '真实窗口 · 操作后停留讲解'
            d.text((192,1004),label+'   |   中文合成旁白 · 可选中英字幕',font=font(20),fill=MUTED)
            overlay.save(video/(s['id']+'-overlay.png'))
            inputs=['-ss',str(start),'-t',str(length),'-i',str(base/'recordings/window-raw.mkv'),'-loop','1','-i',str(video/(s['id']+'-overlay.png'))]
            # Capture and delivery both contain only the app's rendered 1600x880 client area.
            vf=f'[0:v]crop=1600:880:0:0,setpts=(PTS-STARTPTS)/{speed},fps=30,scale=1536:845,pad=1920:1080:192:80:color=0x060a07,tpad=stop_mode=clone:stop_duration={duration}[base];[base][1:v]overlay=0:0,format=yuv420p,fade=t=in:st=0:d=0.2,fade=t=out:st={duration-.2}:d=0.2[v]'
            filters=['-filter_complex',vf,'-map','[v]']
        audio_index=1 if s['id'] in ('intro','outro') else 2
        dest=video/(s['id']+'-edit.mp4')
        command=['ffmpeg','-hide_banner','-loglevel','error','-y',*inputs,'-i',str(video/(s['id']+'.wav')),*filters,'-map',f'{audio_index}:a','-af','apad,loudnorm=I=-16:TP=-1.5:LRA=11','-t',str(duration),'-c:v','libx264','-preset','fast','-crf','19','-c:a','aac','-b:a','160k','-ar','48000','-pix_fmt','yuv420p','-movflags','+faststart','-map_metadata','-1',str(dest)]
        run(command);edits.append(dest)
        for lang in subs:subs[lang].append(f'{stamp(clock)} --> {stamp(clock+duration)}\n{s[lang]}\n')
        s.update(start=round(clock,3),duration=round(duration,3),playback_speed=round(speed,3));clock+=duration
        print('Rendered',s['id'],round(duration,1),'s',flush=True)
    concat=video/'concat.txt';concat.write_text('\n'.join("file '"+p.name+"'" for p in edits),encoding='utf-8')
    final=video/'CosKit-1.0.0-introduction-and-tutorial.mp4'
    run(['ffmpeg','-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',str(concat),'-c','copy','-map_metadata','-1','-movflags','+faststart',str(final)])
    probe=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-of','json',str(final)]))
    assert any(s.get('codec_type')=='video' and s.get('width')==1920 and s.get('height')==1080 for s in probe['streams']), 'Missing 1080p video stream'
    assert any(s.get('codec_type')=='audio' for s in probe['streams']), 'Missing narration audio stream'
    shutil.copy2(final,web/'coskit-tutorial.mp4')
    for lang,cues in subs.items():
        text='WEBVTT\n\n'+'\n'.join(cues);(video/f'tutorial.{lang}.vtt').write_text(text,encoding='utf-8');(web/f'tutorial.{lang}.vtt').write_text(text,encoding='utf-8')
    (video/'edit-manifest.json').write_text(json.dumps({'duration':clock,'segments':segments,'source':'real-window-capture','voice':'Microsoft Huihui Desktop','author':'Suzutsuki / 凉月'},ensure_ascii=False,indent=2),encoding='utf-8')
    for file in (ROOT/'website/coskit').iterdir():
        if file.is_file():shutil.copy2(file,web.parent/file.name)
    local=ROOT/'website/coskit/media';local.mkdir(exist_ok=True)
    for file in web.iterdir():shutil.copy2(file,local/file.name)
    print('Tutorial duration:',round(clock,2),'s; local website media prepared.',flush=True)

if __name__=='__main__':main()
