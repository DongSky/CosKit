"""Layered photo, mask confinement, alpha, precision and format tests over stdio MCP."""
import json, sys
from PIL import Image, ImageChops
from coskit_mcp_client import MCP, ROOT
sys.stdout.reconfigure(encoding='utf-8')
out=ROOT/'test_output/cosplay';results=[]
with MCP() as m:
    m.tool('doc_open',{'path':'regression-base.pcraft'})
    m.tool('doc_save',{'path':'mask-before.png'})
    m.command('select.rect',{'x':950,'y':220,'width':160,'height':140})
    m.command('layer.new.layer',{'name':'Selection confinement'})
    m.tool('brush_stroke',{'points':[{'x':880,'y':290,'pressure':1},{'x':1180,'y':290,'pressure':1}],'size':60,'color':'#ff00ff','opacity':1,'flow':1,'preset':'Hard Round'})
    m.command('select.deselect');m.tool('doc_save',{'path':'mask-after.png'})
    before=Image.open(out/'mask-before.png').convert('RGBA');after=Image.open(out/'mask-after.png').convert('RGBA')
    bbox=ImageChops.difference(before,after).getbbox(alpha_only=False)
    assert bbox and bbox[0]>=950 and bbox[1]>=220 and bbox[2]<=1110 and bbox[3]<=360,bbox
    results.append({'name':'paint-selection-confinement','status':'passed','changed_bounds':bbox})
    m.command('layer.layerMask.revealAll')
    m.tool('brush_stroke',{'points':[{'x':1000,'y':290,'pressure':1}],'size':30,'color':'#000000','opacity':1,'flow':1,'mask':True,'preset':'Soft Round'})
    m.command('layer.newAdjustmentLayer.curves',{'points':[[0,0],[128,136],[255,255]]})
    original=m.tool('doc_inspect');count=len(original['layers']);assert count==3
    for ext,layered in [('pcraft',True),('psd',True),('png',False),('jpg',False),('tif',False),('webp',False)]:
        filename='format-test.'+ext
        m.tool('doc_save',{'path':filename})
        m.tool('doc_open',{'path':filename});d=m.tool('doc_inspect')
        assert (d['width'],d['height'])==(1600,1067),(ext,d)
        if layered:assert len(d['layers'])==count,(ext,d)
        results.append({'name':'format-'+ext,'status':'passed','layers':len(d['layers']),'dimensions':[d['width'],d['height']]})
        m.tool('doc_close')
    # A layered TIFF is separately requested; a plain .tif intentionally flattens.
    m.tool('doc_save',{'path':'format-layered.tif','tiffLayers':True});m.tool('doc_open',{'path':'format-layered.tif'})
    d=m.tool('doc_inspect');assert len(d['layers'])==count;results.append({'name':'layered-tiff','status':'passed','layers':count});m.tool('doc_close')
    m.tool('doc_close')
    for depth in [16,32]:
        m.tool('doc_open',{'path':'regression-base.pcraft'})
        m.command(f'image.mode.bits{depth}')
        d=m.tool('doc_inspect');assert d['depth']==depth,d
        m.command('layer.new.layer',{'name':f'{depth}-bit pressure retouch'})
        m.tool('brush_stroke',{'points':[{'x':980,'y':200,'pressure':0.1},{'x':1050,'y':240,'pressure':0.9}],'size':10,'color':'#dcfaff','opacity':0.5,'flow':0.3,'preset':'CosKit · Hair Single Strand'})
        name=f'precision-{depth}.pcraft';m.tool('doc_save',{'path':name});m.tool('doc_close');m.tool('doc_open',{'path':name});d=m.tool('doc_inspect')
        assert d['depth']==depth and len(d['layers'])==2,d
        results.append({'name':f'precision-{depth}-photo-roundtrip','status':'passed','depth':depth,'dimensions':[d['width'],d['height']]});m.tool('doc_close')
(out/'io-results.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
print('PASS:',len(results),'photo mask / precision / format workflows',flush=True)
