"""Build metadata-free local promo images and a captioned tutorial from real footage.

Usage: python scripts/render-release-media.py --media /path/to/local/demo
Requires Pillow, FFmpeg, and edge-tts (optionally in a separate voice venv).
"""
import argparse, json, math, pathlib, shutil, subprocess, sys, textwrap, wave
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
    p=argparse.ArgumentParser();p.add_argument('--media',type=pathlib.Path,required=True);p.add_argument('--advanced',action='store_true');p.add_argument('--voice-python',default=sys.executable);p.add_argument('--voice',default='zh-CN-XiaoxiaoNeural');a=p.parse_args();base=a.media.resolve()
    web=base/'site-package/coskit/media';web.mkdir(parents=True,exist_ok=True)
    video=base/'video';video.mkdir(exist_ok=True)
    images={'before':base/'originals/source.png','after':base/'exports/after.png',**{n:base/'screenshots'/f'{n}.png' for n in ['workspace','ai-review','brush','parameters','detail']}}
    clean_workspace=base/'screenshots/workspace-clean.png'
    if clean_workspace.exists():images['workspace']=clean_workspace
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
    card('雪霁之后，\n庭前试刀。' if a.advanced else '对话与画笔，\n同一张画布。','复杂修图实战\n神里绫华 · 庭院换景 · 自然精修' if a.advanced else '功能介绍 + 新手教程\n从原片，到可继续编辑的成片','intro').resize((1200,675)).save(web/'tutorial-poster.jpg',quality=93)
    card('保留每一次尝试。','保存 .ckpipe 留下完整工程\n导出 PNG / JPEG 分享成片','outro')
    fresh(web/'tutorial-poster.jpg').resize((1200,630)).save(web/'social-card.jpg',quality=92)
    chapters=json.loads((base/'recordings/timeline.json').read_text(encoding='utf-8'))['chapters'];times={x['name']:x['time'] for x in chapters}
    script=[
      ('intro','CosKit 1.0.0','第一个正式版本 · 对话修图 + 原生编辑','欢迎使用 Cos Kit 一点零。这是第一个正式版本。接下来，用一张花间人像，看一次从对话修图到手工收尾的完整流程。','Welcome to CosKit 1.0, the first stable release. Follow a floral portrait from conversational editing to manual finishing.'),
      ('open','01 / 打开照片，留下原片','Ctrl+O 打开 · 另存为 .ckpipe · 记录基准','打开照片后，先确认画布尺寸，再存一份工程，给原片留个版本。之后的图层、参数和对话，就都能保存在一起。想重新尝试，也有地方可以退回来。','Open the photo and check its dimensions. Save a CKPipe project and name a baseline. Layers, parameters, conversations and versions stay together.'),
      ('ai','02 / 只描述你想要的效果','先在「选项」配置模型 · 不需要指定具体工作流','使用对话前，在选项中配置自己的模型。这里只描述目标：让人物清新通透，同时保留肤质、白衣和花朵的层次。模型自己分析步骤，发现工具并执行。这一段等待已经加速。','Configure your models in Options. Describe the desired result, preserving skin, white fabric and flowers. The model discovers tools and plans the steps. Waiting is accelerated here.'),
      ('review','03 / 结果还需要细看','独立审核 · 原尺寸局部 · 不通过就回退','调整完成后，模型还会检查全图和原尺寸局部。你也可以展开过程记录，了解做了什么。审核通过后，结果才进入工程；是否满意，仍由你决定。','The reviewer inspects the whole image and native-size crops. Expand the trace to see the actions. Approved results enter the project; the final creative judgment remains yours.'),
      ('brush','04 / 在独立图层上收尾','低流量柔笔刷 · 克制地补一点花间光感','接下来新建图层，用低流量的柔笔刷，在花朵附近补一点柔光。手工收尾和 AI 使用同一张画布。独立图层让你随时降低强度、隐藏，或者撤销。','Create a separate layer and add restrained soft light near the flowers with a low-flow brush. Manual work shares the AI canvas. Reduce, hide or undo the layer whenever needed.'),
      ('parameters','05 / 看得懂，也改得动','版本条「参数」 · 曲线、滑杆 · 明确选择基准','点开下方版本条里的参数，就能看到曲线和滑杆。想换一种调色，可以选好起始版本和那条编辑记录，再调整参数、重新执行。新的结果会单独留下来，方便继续比较。','Open Parameters in the version strip for curves and sliders. Choose the baseline and operation before replaying. A successful replay creates a branch and retains earlier versions.'),
      ('compare','06 / 从全图检查到发丝细节','对比 → 100% → 200% → 差异图','最后对比原片和成片。左右同步缩放，检查脸部、发丝与衣料。百分之百看原尺寸，百分之二百放大，差异图帮助定位变化。浏览对比不会修改画布。','Compare the original and result with synchronized views. Inspect the face, hair and fabric at 100% and 200%. Difference view locates changes. Comparison does not modify the canvas.'),
      ('save','07 / 工程与成片，分别保存','保存 .ckpipe · 导出 PNG / JPEG','修好以后，记得把工程也存下来。它会保留图层、对话和版本，方便下次接着改。发图时，再导出普通图片。普通图片会合并图层，所以工程也要留着。最后，我们重新打开工程，确认保存前后的尺寸和画面完全一致。','Save CKPipe for layers, conversation and versions. Export PNG or JPEG for sharing. The demo reopens the project and verifies unchanged dimensions and exported pixels.'),
      ('outro','CosKit 1.0.0','开源创作工作台 · 凉月 / Suzutsuki','先说出想法，再动手打磨。希望这次演示，能给你的下一张作品一点灵感。','Start with a goal, and pick up the brush whenever you want. CosKit 1.0 is ready for your next creative project.')]
    segments=[dict(zip(['id','title','caption','zh','en'],x)) for x in script]
    if a.advanced:
        advanced = {
            'intro': ('CosKit / 复杂修图实战','换背景 · 调色 · 自然美颜磨皮 · 光影融合',
                '这次，我们来修一张神里绫华的照片。参考角色的冰元素和雪后庭院意象，先换背景，再整理肤色和光线。一起看看，对话修图和手工收尾，怎么在同一个工程里配合。',
                'Retouch a Kamisato Ayaka cosplay with a snow-cleared courtyard, natural skin and coherent light, drawing on her Cryo and garden imagery. Follow conversational editing and manual finishing in the same project.'),
            'ai': ('02 / 一条完整的成片指令','背景可重建 · 人物身份与服饰保留 · 画布尺寸不变',
                '先说清楚想要的成片：雪后的日式庭院，冷白、浅蓝和淡紫，带一点清晨的暖光。保留人物的闭眼神态、服装和手里的道具。肤色修得自然一些，细节别磨掉。接下来，由模型安排步骤、调用工具，再检查效果。这里加速了等待过程。',
                'Describe a Japanese courtyard after snow: cool whites, pale blue and lilac with a little warm morning light. Preserve the closed eyes, pose, costume, prop and skin detail. The harness plans, executes and reviews its workflow. Waiting is accelerated.'),
            'review': ('03 / 换景之后，检查人物','检查脸、发丝、手部、服装和武器边缘',
                '换好背景以后，先别急着导出。放大看看脸、手指和刀鞘，再检查发丝和白衣的边缘。模型会审核结果，发现新问题就回退。我们也要再看一遍，确认画面符合自己的想法。',
                'After replacing the background, inspect the face, fingers, scabbard, hair and white fabric at full size. The harness reviews the result and rolls back new problems. Check the final image yourself as well.'),
            'brush': ('04 / 独立图层补光','低流量软笔刷 · 保留可撤销的手工收尾',
                '接着，新建一个图层，用低流量的软笔刷，轻轻补一点晨光。别涂到五官上，也不用一次画得太重。这个图层可以随时隐藏，或者降低强度，再慢慢找到合适的光感。',
                'Add a restrained touch of ambient light on a separate layer with a low-flow soft brush, away from facial features. Manual finishing and AI edits share one project and remain editable.'),
            'compare': ('06 / 大变化，也要查小细节','原尺寸对比 · 肤质 / 发丝 / 衣料 · 画布 1620 × 1080',
                '现在把原片和成片放在一起看。先看整体，再放大检查肤质、发丝和衣料。留意有没有过度磨皮，边缘有没有不自然的亮圈。这里改变的只是查看倍率，画布仍然是一千六百二十乘一千零八十。',
                'Compare the full image, then inspect the face, hair and white fabric. Look for excessive smoothing, edge halos and mismatched lighting. Zooming only changes the view: the canvas remains 1620 by 1080.'),
        }
        for segment in segments:
            if segment['id'] in advanced:
                segment.update(dict(zip(('title','caption','zh','en'),advanced[segment['id']])))
    (video/'storyboard.json').write_text(json.dumps(segments,ensure_ascii=False,indent=2),encoding='utf-8')
    subprocess.run([a.voice_python,str(ROOT/'scripts/synthesize-release-narration.py'),'--directory',str(video),'--voice',a.voice],check=True)
    voice_manifest=json.loads((video/'voice-manifest.json').read_text(encoding='utf-8'))
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
        cues=json.loads((video/(s['id']+'-timing.json')).read_text(encoding='utf-8'))
        for cue_index,cue in enumerate(cues):
            # Service sentence timings can overlap by about 50 ms.
            next_start=cues[cue_index+1]['start'] if cue_index+1<len(cues) else duration
            cue_end=min(cue['end'],next_start,duration)
            subs['zh'].append(f"{stamp(clock+cue['start'])} --> {stamp(clock+cue_end)}\n{cue['text']}\n")
        subs['en'].append(f"{stamp(clock)} --> {stamp(clock+duration)}\n{s['en']}\n")
        s.update(start=round(clock,3),duration=round(duration,3),playback_speed=round(speed,3));clock+=duration
        print('Rendered',s['id'],round(duration,1),'s',flush=True)
    concat=video/'concat.txt';concat.write_text('\n'.join("file '"+p.name+"'" for p in edits),encoding='utf-8')
    final=video/('CosKit-1.0.0-advanced-retouch-tutorial.mp4' if a.advanced else 'CosKit-1.0.0-introduction-and-tutorial.mp4')
    run(['ffmpeg','-hide_banner','-loglevel','error','-y','-f','concat','-safe','0','-i',str(concat),'-c','copy','-map_metadata','-1','-movflags','+faststart',str(final)])
    probe=json.loads(subprocess.check_output(['ffprobe','-v','error','-show_streams','-of','json',str(final)]))
    assert any(s.get('codec_type')=='video' and s.get('width')==1920 and s.get('height')==1080 for s in probe['streams']), 'Missing 1080p video stream'
    assert any(s.get('codec_type')=='audio' for s in probe['streams']), 'Missing narration audio stream'
    shutil.copy2(final,web/'coskit-tutorial.mp4')
    for lang,cues in subs.items():
        text='WEBVTT\n\n'+'\n'.join(cues);(video/f'tutorial.{lang}.vtt').write_text(text,encoding='utf-8');(web/f'tutorial.{lang}.vtt').write_text(text,encoding='utf-8')
    (video/'edit-manifest.json').write_text(json.dumps({'duration':clock,'segments':segments,'source':'real-window-capture','voice':voice_manifest,'author':'Suzutsuki / 凉月'},ensure_ascii=False,indent=2),encoding='utf-8')
    for file in (ROOT/'website/coskit').iterdir():
        if file.is_file():shutil.copy2(file,web.parent/file.name)
    local=ROOT/'website/coskit/media';local.mkdir(exist_ok=True)
    for file in web.iterdir():shutil.copy2(file,local/file.name)
    print('Tutorial duration:',round(clock,2),'s; local website media prepared.',flush=True)

if __name__=='__main__':main()
