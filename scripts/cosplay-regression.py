"""Real-photo command regression through MCP, separate from the full-resolution final edit.

The original is opened at full size and checked; a deliberately resized 1600px test
document limits the cost of dozens of disposable filter variants. Every case records
its dimensions, pixel difference, elapsed time and whether undo restored the baseline.
"""
import hashlib, json, pathlib, sys, time
from PIL import Image, ImageChops
from coskit_mcp_client import MCP, ROOT
sys.stdout.reconfigure(encoding='utf-8')
OUT=ROOT/'test_output/cosplay'; RESULTS=[]
CASES=[]
def case(name,steps,kind='pixels',dimensions=None): CASES.append((name,steps,kind,dimensions))
def one(name,ident,params): case(name,[(ident,params)])

for name,ident,p in [
    ('exposure','image.adjustments.exposure',{'exposure':0.25}),
    ('contrast','image.adjustments.brightnessContrast',{'brightness':5,'contrast':8}),
    ('curves','image.adjustments.curves',{'points':[[0,0],[64,59],[128,137],[255,255]]}),
    ('skin-cyan-correction','image.adjustments.hueSaturation',{'cyans':{'saturation':-15},'saturation':-3}),
    ('vibrance','image.adjustments.vibrance',{'vibrance':12,'saturation':-3}),
    ('levels','image.adjustments.levels',{'inBlack':5,'inWhite':248,'gamma':1.08}),
    ('color-balance','image.adjustments.colorBalance',{'midtones':[3,-2,-4]}),
    ('black-white','image.adjustments.blackWhite',{}),
    ('camera-raw','filter.cameraRaw',{'temperature':5,'tint':3,'highlights':-18,'shadows':12,'texture':5,'noiseColor':12}),
    ('gaussian-background','filter.blur.gaussianBlur',{'radius':3}),
    ('motion-blur','filter.blur.motionBlur',{'angle':-20,'distance':12}),
    ('surface-blur','filter.blur.surfaceBlur',{'radius':3,'threshold':12}),
    ('median-cleanup','filter.noise.median',{'radius':1}),
    ('denoise','filter.noise.reduceNoise',{'strength':3,'preserveDetails':85,'reduceColorNoise':30,'sharpenDetails':5}),
    ('unsharp','filter.sharpen.unsharpMask',{'amount':35,'radius':0.7,'threshold':4}),
    ('high-pass','filter.other.highPass',{'radius':2}),
    ('dust-scratches','filter.noise.dustAndScratches',{'radius':2,'threshold':8}),
]: one(name,ident,p)

# Coordinates are on the disposable 1600x1067 working document.
path=[[1010,276,0.3],[1015,280,0.8],[1022,286,0.5]]
for tool,p in [
    ('cloneStamp',{'source':[1010,267],'sampleLayer':'current'}),
    ('healingBrush',{'source':[1010,267]}),
    ('spotHealing',{'type':'proximityMatch'}),
    ('dodge',{'exposure':10,'range':'midtones'}),
    ('burn',{'exposure':10,'range':'highlights'}),
    ('sponge',{'mode':'desaturate','flow':100,'opacity':100,'size':70,'points':[[870,225,1],[900,245,1]]}),
    ('blur',{'strength':20}),('sharpen',{'strength':20}),('smudge',{'strength':20}),
]: one('retouch-'+tool,'paint.'+tool,{'points':path,'size':22,'hardness':15,'opacity':70,'flow':35,**p})
one('spot-content-aware','paint.spotHealing',{'points':[[300,650,1]],'size':15,'type':'contentAware'})
one('spot-texture','paint.spotHealing',{'points':[[300,650,1]],'size':15,'type':'createTexture'})
one('mixer-brush','paint.mixerBrush',{'points':path,'size':20,'wet':20,'load':30,'mix':60,'flow':20,'color':'#a9b7b9'})
one('color-replacement','paint.colorReplacement',{'points':[[986,240,1],[990,244,1]],'size':16,'color':'#41caff','tolerance':50})
one('liquify-fabric','filter.liquify',{'strokes':[{'tool':'forwardWarp','size':45,'pressure':10,'points':[[1215,527,1],[1219,526,1]]}]})

rect=('select.rect',{'x':975,'y':225,'width':95,'height':105})
for name,steps in [
    ('rectangle',[rect]),
    ('ellipse',[('select.rect',{'x':970,'y':210,'width':110,'height':135,'ellipse':True})]),
    ('lasso',[('select.lasso',{'points':[[940,205],[1040,200],[1090,330],[960,350]],'feather':2})]),
    ('feather',[rect,('select.modify.feather',{'radius':4})]),
    ('inverse',[rect,('select.inverse',{})]),
    ('color-range',[('select.colorRange',{'select':'cyans','fuzziness':40})]),
    ('quick-select',[('select.quick',{'points':[[1010,290]],'size':25,'mode':'replace'})]),
    ('subject',[('select.subject',{})]),
    ('save-load-selection',[rect,('select.saveSelection',{'name':'Face test'}),('select.deselect',{}),('select.loadSelection',{'channel':'Face test'})]),
]: case('selection-'+name,steps,'selection')
case('masked-adjustment',[rect,('layer.newAdjustmentLayer.exposure',{'exposure':0.2})])
case('mask-paint',[('layer.layerMask.revealAll',{}),('paint.stroke',{'points':[[1010,280,1],[1040,310,1]],'size':35,'color':'#000000','target':'mask','flow':0.3})])
case('layer-opacity',[('layer.setProps',{'opacity':0.6})])
case('layer-blend',[('layer.duplicate',{}),('layer.setProps',{'blend':'Soft Light','opacity':0.3})])
case('smart-object-filter',[('layer.smartObjects.convertToSmartObject',{}),('filter.blur.gaussianBlur',{'radius':1.5})])
case('patch',[('select.rect',{'x':280,'y':630,'width':25,'height':25}),('paint.patch',{'offset':[30,0],'mode':'source'})])
case('text-layer',[('type.create',{'x':80,'y':95,'text':'COSKIT / FIELD TEST','size':25,'color':'#e5ffff'})])
case('gradient-light',[('layer.new.layer',{'name':'Gradient test'}),('paint.gradient',{'from':[100,100],'to':[500,600],'colors':['#37c4df','#091324'],'transparency':[[0,40],[1,0]]})])
case('intentional-image-resize',[('image.imageSize',{'width':1200,'height':800,'resample':'lanczos'})],dimensions=(1200,800))
case('intentional-canvas-extension',[('image.canvasSize',{'width':1660,'height':1107,'anchor':'center','extensionColor':'transparent'})],dimensions=(1660,1107))

if '--only-failed' in sys.argv:
    RESULTS=json.loads((OUT/'regression-results.json').read_text(encoding='utf-8'))
    failed={r['name'] for r in RESULTS if r['status']=='failed'}
    CASES=[c for c in CASES if c[0] in failed]
    RESULTS=[r for r in RESULTS if r['name'] not in failed]

with MCP() as m:
    info=m.info
    (OUT/'mcp-tools.json').write_text(json.dumps(m.request('tools/list',{}),ensure_ascii=False,indent=2),encoding='utf-8')
    m.tool('doc_open',{'path':'original.jpg'})
    d=m.tool('doc_inspect');assert (d['width'],d['height'])==(5472,3648)
    m.command('image.imageSize',{'width':1600,'height':1067,'resample':'lanczos'})
    base=m.tool('doc_inspect');dims=(base['width'],base['height'])
    m.tool('doc_save',{'path':'regression-base.pcraft'})
    for name,steps,kind,expected in CASES:
        start=time.monotonic();row={'name':name,'commands':[s[0] for s in steps],'baseline_dimensions':dims}
        # Reopening the immutable baseline also resets non-history tool and selection state.
        m.tool('doc_close');m.tool('doc_open',{'path':'regression-base.pcraft'})
        m.tool('doc_render_preview',{'max_side':960},image_path=OUT/'regression-before.png')
        before=Image.open(OUT/'regression-before.png').convert('RGBA')
        try:
            old=m.tool('doc_inspect');old_hist=len(old['history']);old_layers=len(old['layers'])
            for ident,p in steps:m.command(ident,p)
            d=m.tool('doc_inspect');row['dimensions']=[d['width'],d['height']]
            assert tuple(row['dimensions'])==(expected or dims),row['dimensions']
            m.tool('doc_render_preview',{'max_side':960},image_path=OUT/f'test-{name}.png')
            after=Image.open(OUT/f'test-{name}.png').convert('RGBA')
            changed=before.size!=after.size or ImageChops.difference(before,after).convert('RGB').getbbox() is not None
            row['pixels_changed']=changed
            if kind=='selection':assert d['hasSelection'],'selection missing'
            elif expected is None:assert changed or len(d['layers'])!=old_layers,'command produced no visible effect'
            for _ in range(len(d['history'])-old_hist):m.command('edit.undo')
            m.tool('doc_render_preview',{'max_side':960},image_path=OUT/'regression-undo.png')
            undone=Image.open(OUT/'regression-undo.png').convert('RGBA')
            row['undo_restored_pixels']=before.size==undone.size and ImageChops.difference(before,undone).getbbox(alpha_only=False) is None
            assert row['undo_restored_pixels'],'undo pixels differ'
            row['status']='passed'
        except Exception as e:row.update(status='failed',error=str(e))
        row['seconds']=round(time.monotonic()-start,2);RESULTS.append(row)
        (OUT/'regression-results.json').write_text(json.dumps(RESULTS,ensure_ascii=False,indent=2),encoding='utf-8')
        print(row['status'],name,row.get('error',''),flush=True)
print('TOTAL',len(RESULTS),'FAILED',sum(r['status']=='failed' for r in RESULTS),flush=True)
