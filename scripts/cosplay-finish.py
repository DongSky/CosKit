"""Manual layered photo finishing: all editing commands and strokes go through CosKit MCP."""
import json,math,sys,time
from coskit_mcp_client import MCP,ROOT
sys.stdout.reconfigure(encoding='utf-8')
out=ROOT/'test_output/cosplay';S=5472/1920;strokes=[];repairs=[]
def curve(a,b,c,d,n=40):
    pts=[]
    for i in range(n+1):
        t=i/n;u=1-t
        x=u*u*u*a[0]+3*u*u*t*b[0]+3*u*t*t*c[0]+t*t*t*d[0]
        y=u*u*u*a[1]+3*u*u*t*b[1]+3*u*t*t*c[1]+t*t*t*d[1]
        pts.append({'x':x*S,'y':y*S,'pressure':0.03+0.85*math.sin(math.pi*t)**0.65,'time_ms':i*14})
    return pts
def draw(m,preset,points,size,color,opacity,flow):
    args={'preset':preset,'points':points,'size':size,'color':color,'opacity':opacity,'flow':flow,'seed':904+len(strokes)}
    result=m.tool('brush_stroke',args);strokes.append({'arguments':args,'result':result})
def layer(m,name,blend='Normal',opacity=1):
    m.command('layer.new.layer',{'name':name});m.command('layer.setProps',{'blend':blend,'opacity':opacity})
def clone_patch(m,name,polygon,at,offset,size,expand=0):
    m.command('select.lasso',{'points':[[x*S,y*S] for x,y in polygon]})
    if expand:m.command('select.modify.expand',{'radius':expand})
    m.command('select.modify.feather',{'radius':8})
    layer(m,name)
    args={'points':[[at[0]*S,at[1]*S,1]],'offset':[offset[0]*S,offset[1]*S],'size':size*S,'hardness':100,'opacity':100,'flow':100,'sampleLayer':'all'}
    result=m.command('paint.cloneStamp',args);repairs.append({'name':name,'command':'paint.cloneStamp','params':args,'result':result})
    m.command('select.deselect')
def save(m,name,psd=False):
    d=m.tool('doc_inspect');assert (d['width'],d['height'])==(5472,3648),d
    m.tool('doc_save',{'path':name+'.pcraft'});m.tool('doc_save',{'path':name+'.png'});m.tool('doc_save',{'path':name+'.jpg'})
    if psd:m.tool('doc_save',{'path':name+'.psd'})
    (out/(name+'-document.json')).write_text(json.dumps(d,ensure_ascii=False,indent=2),encoding='utf-8')
    print('Saved',name,len(d['layers']),'layers at 5472x3648',flush=True)

with MCP(bridge=50507) as m:
    m.tool('doc_open',{'path':'02-background-clean.pcraft'})
    # Interior background holes need their own masks; the outer silhouette cannot describe them.
    clone_patch(m,'Cleanup · floor visible between legs',[(856,815),(850,839),(837,865),(814,891),(778,924),(724,958),(782,945),(841,930),(884,910),(928,895)],(830,885),(-340,-50),390,8)
    clone_patch(m,'Cleanup · remaining foil at floor',[(980,1084),(1050,1095),(1125,1097),(1200,1087),(1248,1090),(1270,1108),(1236,1123),(1108,1124),(990,1114)],(1130,1108),(0,75),380)
    clone_patch(m,'Cleanup · backdrop between wig and ear',[(1070,272),(1082,245),(1105,221),(1110,221),(1098,246),(1089,271)],(1088,250),(-90,20),85)
    # Local skin colour correction is an editable masked adjustment, preserving makeup.
    m.command('select.rect',{'x':round(1143*S),'y':round(245*S),'width':round(161*S),'height':round(166*S),'ellipse':True,'feather':30})
    m.command('layer.newAdjustmentLayer.colorBalance',{'midtones':[4,-3,-4],'highlights':[2,-1,-2],'preserveLuminosity':True})
    m.command('layer.setProps',{'name':'Skin · restrained colour balance','opacity':0.55});m.command('select.deselect')
    # Light and shadow shaping are painted on separate, reversible layers.
    layer(m,'Light · hand-painted soft shaping','Soft Light',0.35)
    for a,b,c,d in [((1179,298),(1168,322),(1170,350),(1181,365)),((867,664),(899,685),(955,749),(973,809)),((1159,605),(1148,653),(1154,694),(1157,725))]:
        draw(m,'CosKit · Soft Dodge Burn',curve(a,b,c,d),95,'#f1e7dc',0.3,0.05)
    layer(m,'Shadow · hand-painted depth','Multiply',0.18)
    draw(m,'CosKit · Soft Dodge Burn',curve((715,1018),(791,1050),(934,1096),(1050,1089)),130,'#314651',0.25,0.05)
    # Tapered strands follow the wig's existing growth, rather than drawing across the face.
    layer(m,'Hair · tapered strand repair')
    for i in range(14):
        j=(i-7)*1.4
        pts=curve((1181+j,167),(1138+j,189),(1083+j,285),(1105+j,350))
        draw(m,'CosKit · Hair Single Strand',pts,3.0+(i%3)*0.7,['#c1d9db','#829da6','#e2ebe5'][i%3],0.3,0.25)
    for i in range(7):
        j=i*1.8
        draw(m,'CosKit · Hair Soft Flyaways',curve((1307+j,223),(1351+j,282),(1351+j,339),(1309+j,365)),2.4,'#c6e4e8',0.4,0.22)
    draw(m,'Deevad · 3 rake',curve((1110,239),(1088,282),(1089,315),(1112,350)),12,'#a9cbd0',0.16,0.16)
    layer(m,'Rim · cool reflected light','Screen',0.35)
    draw(m,'CosKit · Rim Light',curve((1071,225),(1051,261),(1055,285),(1066,311)),18,'#66cbd7',0.2,0.12)
    draw(m,'CosKit · Rim Light',curve((1540,743),(1596,711),(1644,674),(1679,631)),16,'#44a9bf',0.2,0.1)
    save(m,'03-cosplay-clean',True)
    # The effect version keeps the clean portrait as its editable lower stack.
    m.command('select.loadSelection',{'channel':'Subject protection contour'})
    m.command('select.inverse')
    layer(m,'FX · soft energy bloom','Screen',0.7)
    arc=curve((277,1041),(142,672),(1389,507),(1760,506),100)
    draw(m,'CosKit · Glow Airbrush',arc,180,'#2692b3',0.6,0.35)
    layer(m,'FX · hand-drawn energy trail','Screen',0.8)
    draw(m,'CosKit · Rim Light',arc,12,'#50deec',0.65,0.4)
    draw(m,'CosKit · Hair Single Strand',arc,3.5,'#d0faff',0.9,0.8)
    layer(m,'FX · CC0 painted atmosphere','Screen',0.55)
    for x,y,diameter in [(392,816,90),(585,669,75),(749,647,65),(465,740,55)]:
        draw(m,'Deevad · splat dots',[{'x':x*S,'y':y*S,'pressure':0.8}],diameter*S,'#88dce5',0.22,0.25)
    layer(m,'FX · sparse sparks','Screen',0.7)
    for x,y,size in [(339,899,27),(475,779,16),(591,702,23),(745,638,14),(439,675,12),(693,749,15),(829,568,11)]:
        draw(m,'CosKit · Spark Star',[{'x':x*S,'y':y*S,'pressure':1}],size*S,'#d2faff',0.95,1)
    m.command('select.deselect');save(m,'04-cosplay-effects',True)
    m.tool('ui_set',{'fields':{'fit':True}});m.tool('ui_screenshot',{'max_side':1600},image_path=out/'04-final-workspace.png')
(out/'manual-strokes.json').write_text(json.dumps(strokes,ensure_ascii=False,indent=2),encoding='utf-8')
(out/'manual-repairs.json').write_text(json.dumps(repairs,ensure_ascii=False,indent=2),encoding='utf-8')
print('Completed',len(strokes),'explicit MCP brush strokes.',flush=True)
