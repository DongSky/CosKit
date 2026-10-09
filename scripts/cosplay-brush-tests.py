"""Exercise every added brush on the user's photo through dedicated MCP stroke tools."""
import json, sys, math, time
from PIL import Image, ImageChops, ImageDraw
from coskit_mcp_client import MCP, ROOT
sys.stdout.reconfigure(encoding='utf-8')
out=ROOT/'test_output/cosplay';results=[];tiles=[]
with MCP() as m:
    presets=m.tool('brush_list')['presets']
    added=[p['name'] for p in presets if p['name'].startswith(('CosKit ·','Deevad ·'))]
    assert len(added)==28,added
    m.tool('doc_new',{'width':240,'height':180,'background':'white'})
    for y,pressure,flow in [(35,0.2,1),(75,1,1),(115,1,0.12),(155,1,1)]:
        m.tool('brush_stroke',{'points':[{'x':30,'y':y,'pressure':pressure},{'x':210,'y':y,'pressure':pressure}], 'size':24,'opacity':1,'flow':flow,'color':'#000000','preset':'CosKit · Hair Single Strand','spacing':0.4})
    m.tool('doc_render_preview',{'max_side':240},image_path=out/'brush-pressure-flow.png')
    im=Image.open(out/'brush-pressure-flow.png').convert('RGB')
    span=lambda y:sum(im.getpixel((120,k))[0]<200 for k in range(y-15,y+16))
    low,high=span(35),span(75);assert high>=low*2 and low>0,(low,high)
    lowflow=im.getpixel((120,115))[0];highflow=im.getpixel((120,155))[0];assert lowflow>highflow+10,(lowflow,highflow)
    results.append({'name':'pressure-flow-measurement','status':'passed','pressure_02_width':low,'pressure_10_width':high,'flow_012_luminance':lowflow,'flow_10_luminance':highflow})
    m.tool('doc_close')
    for name in added:
        t=time.monotonic();m.tool('doc_open',{'path':'regression-base.pcraft'})
        m.tool('doc_render_preview',{'max_side':960},image_path=out/'brush-before.png')
        m.command('layer.new.layer',{'name':name})
        points=[{'x':840+i*7,'y':160+24*math.sin(i/9),'pressure':0.05+0.9*math.sin(math.pi*i/32),'time_ms':i*12} for i in range(33)]
        m.tool('brush_stroke',{'points':points,'size':2.5 if 'Single' in name or 'Flyaways' in name else 38,'color':'#ddfaff','opacity':0.9,'flow':0.5,'preset':name,'seed':72})
        m.tool('doc_render_preview',{'max_side':960},image_path=out/'brush-after.png')
        before=Image.open(out/'brush-before.png').convert('RGB');after=Image.open(out/'brush-after.png').convert('RGB')
        assert ImageChops.difference(before,after).getbbox() is not None,name
        d=m.tool('doc_inspect');assert (d['width'],d['height'])==(1600,1067)
        m.command('edit.undo');m.command('edit.redo')
        m.tool('doc_render_preview',{'max_side':960},image_path=out/'brush-replayed.png')
        assert ImageChops.difference(after,Image.open(out/'brush-replayed.png').convert('RGB')).getbbox() is None,name
        tile=after.crop((475,65,705,245)).resize((345,270))
        card=Image.new('RGB',(365,305),'#172026');card.paste(tile,(10,28));ImageDraw.Draw(card).text((10,8),name.replace(' · ',' / ')[:48],fill='white');tiles.append(card)
        results.append({'name':name,'status':'passed','seconds':round(time.monotonic()-t,2),'dimensions':[1600,1067],'replay_identical':True})
        print('passed',name,flush=True);m.tool('doc_close')
sheet=Image.new('RGB',(365*4,305*math.ceil(len(tiles)/4)),'#11181d')
for i,im in enumerate(tiles):sheet.paste(im,((i%4)*365,(i//4)*305))
sheet.save(out/'brush-photo-contact.jpg',quality=94)
(out/'brush-results.json').write_text(json.dumps(results,ensure_ascii=False,indent=2),encoding='utf-8')
print('PASS: 28 photo brush variants, deterministic redo, measured pressure and flow.',flush=True)
