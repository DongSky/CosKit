"""Full-resolution cosplay workflow through the actual CosKit stdio MCP bridge.

Input is the user's private test copy. Outputs remain in ignored test_output/cosplay.
Use --stage background to start AI; --stage finish-background verifies/completes it.
"""
import argparse, json, math, pathlib, sys, time
from coskit_mcp_client import MCP, ROOT
sys.stdout.reconfigure(encoding='utf-8')
p=argparse.ArgumentParser();p.add_argument('--stage',required=True);p.add_argument('--port',type=int,default=50507);a=p.parse_args()
OUT=ROOT/'test_output/cosplay'
# The contour was drawn on a 1920x1280 preview; full-resolution coordinates are 2.85x.
SUBJECT=[(18,1178),(24,1140),(59,1110),(112,1098),(160,1078),(227,1066),(291,1070),(335,1050),(408,1024),(461,1007),(525,966),(600,880),(677,797),(745,719),(796,663),(838,638),(876,639),(925,668),(957,684),(972,646),(990,603),(1011,570),(1020,542),(1038,515),(1054,498),(1065,474),(1080,457),(1072,423),(1044,424),(1028,411),(1046,382),(1073,363),(1069,336),(1040,320),(1001,339),(1012,312),(992,325),(1011,285),(1022,250),(1021,217),(1039,195),(1055,168),(1077,144),(1105,133),(1127,112),(1165,92),(1200,128),(1228,142),(1261,144),(1289,153),(1301,141),(1307,117),(1331,98),(1351,107),(1374,160),(1385,209),(1398,256),(1394,290),(1380,318),(1437,300),(1466,301),(1474,327),(1470,363),(1455,406),(1442,436),(1480,460),(1528,476),(1571,440),(1588,416),(1617,406),(1636,396),(1662,392),(1691,405),(1722,428),(1743,452),(1773,464),(1808,497),(1817,510),(1797,504),(1770,487),(1762,502),(1757,524),(1749,544),(1738,550),(1735,534),(1740,511),(1729,484),(1715,474),(1706,480),(1723,501),(1722,535),(1711,581),(1690,631),(1659,670),(1605,713),(1560,740),(1529,761),(1485,786),(1481,812),(1504,852),(1520,893),(1532,940),(1548,982),(1602,1049),(1660,1084),(1691,1119),(1620,1131),(1550,1129),(1466,1103),(1385,1101),(1308,1115),(1243,1126),(1176,1139),(1102,1146),(1036,1144),(964,1130),(899,1120),(819,1102),(758,1092),(682,1084),(630,1087),(589,1108),(557,1140),(508,1186),(450,1195),(388,1200),(328,1210),(269,1208),(208,1224),(152,1230),(96,1220),(48,1208)]
S=5472/1920
def dimensions(m):
    d=m.tool('doc_inspect');assert (d['width'],d['height'])==(5472,3648),d;return d
def save(m,name):
    dimensions(m);m.tool('doc_save',{'path':name+'.pcraft'});m.tool('doc_save',{'path':name+'.png'})
def snapshot(m,name):m.tool('ui_screenshot',{'max_side':1600},image_path=OUT/(name+'.png'))

with MCP(bridge=a.port) as m:
    if a.stage in ('background','background-fixed'):
        tools=m.request('tools/list',{});assert any(t['name']=='brush_stroke' for t in tools['tools'])
        (OUT/'live-mcp-tools.json').write_text(json.dumps(tools,ensure_ascii=False,indent=2),encoding='utf-8')
        m.tool('doc_open',{'path':'original.jpg'});d=dimensions(m)
        m.tool('doc_save',{'path':'00-original-native.png'})
        m.control('ui.resize',{'width':1600,'height':1000});m.tool('ui_set',{'fields':{'fit':True}})
        m.command('select.lasso',{'points':[[round(x*S),round(y*S)] for x,y in SUBJECT],'feather':4})
        if a.stage=='background-fixed':
            # Protect the interior at full resolution, give the model an overlap band for
            # clean hair/garment contours instead of preserving polygon-shaped old backdrop.
            m.command('select.modify.contract',{'radius':120})
            m.command('select.modify.feather',{'radius':45})
        m.command('select.saveSelection',{'name':'Subject protection contour'})
        m.command('select.inverse');snapshot(m,'01-background-selection')
        m.tool('ai_configure',{'harness_enabled':False,'retouch':a.stage=='background-fixed','background':a.stage!='background-fixed','effects':False,'agent_mode':a.stage!='background-fixed','review_enabled':a.stage!='background-fixed','save_intermediates':True})
        m.tool('ai_edit',{'prompt':'仅编辑选区内的背景与地面。将漫展现场的人群、三脚架、灯架、杂物与反光锡纸替换为干净的写实科幻摄影棚：深青灰色空间，低调的冷白竖向灯带，简洁的哑光地台，可信的接触阴影，轻微景深。保持原始相机视角和3:2构图，不裁切不扩图。人物的脸、五官、表情、蓝灰假发、黑色耳饰、服装、肢体、手指、鞋子及人物携带的道具全部保持原样。不要重绘人物，不添加文字，不添加新的角色或明显炫光。背景应简洁自然，不要抢夺人物注意力。'})
        (OUT/'ai-background-before.json').write_text(json.dumps({'layers':len(d['layers']),'width':d['width'],'height':d['height']},indent=2))
        print('Full-resolution masked background AI submitted through MCP.',flush=True)
    elif a.stage=='retry-background':
        # Direct image editing remains usable if the separate planning model is unavailable.
        dimensions(m)
        (OUT/'planner-failure.json').write_text(json.dumps(m.tool('ai_status'),ensure_ascii=False,indent=2),encoding='utf-8')
        m.tool('ai_configure',{'harness_enabled':False,'retouch':True,'background':False,'effects':False,'agent_mode':False,'review_enabled':False})
        m.tool('ai_edit',{'prompt':'Only edit the selected background and floor. Remove convention attendees, tripods, light stands, clutter and silver foil. Replace with a clean photorealistic dark teal-gray science-fiction photography studio, subtle vertical cool-white light strips, a matte platform and realistic contact shadows. Preserve the original 3:2 composition, camera angle and all person pixels: identity, face, expression, gray-blue wig, black ears, outfit, hands, fingers, limbs and boots. Do not redraw the person. No cropping, no text, no extra characters, no flashy effects. Keep the background quiet and natural.'})
        print('Direct image-edit fallback submitted via MCP.',flush=True)
    elif a.stage=='status':print(json.dumps(m.tool('ai_status'),ensure_ascii=False),flush=True)
    elif a.stage=='finish-background':
        status=m.tool('ai_status');assert not status['busy'],status;assert not status['pending'],status
        d=dimensions(m);before=json.loads((OUT/'ai-background-before.json').read_text());assert len(d['layers'])==before['layers']+1,status
        m.command('select.deselect');save(m,'02-background-clean');snapshot(m,'02-background-ui')
        from PIL import Image,ImageChops
        original=Image.open(OUT/'00-original-native.png').convert('RGB');result=Image.open(OUT/'02-background-clean.png').convert('RGB')
        assert ImageChops.difference(original,result).getbbox(),'AI did not change any pixels'
        protected=[]
        for x,y in [(1205,333),(1210,600),(1000,870)]:
            point=(round(x*S),round(y*S));old=original.getpixel(point);new=result.getpixel(point)
            assert old==new,(point,old,new)
            protected.append({'point':point,'before':old,'after':new})
        (OUT/'ai-protected-pixels.json').write_text(json.dumps({'dimensions':[5472,3648],'protected_samples':protected,'outcome':status.get('outcome')},indent=2))
        print('Background result saved; original 5472x3648 dimensions verified.',flush=True)
    elif a.stage=='inspect': print(json.dumps({'ai':m.tool('ai_status'),'document':m.tool('doc_inspect')},ensure_ascii=False),flush=True)
