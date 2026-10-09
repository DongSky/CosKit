import json,re
from pathlib import Path
root=Path(__file__).resolve().parents[1]
result={}
for name in ['native-tests-final','native-integration-tests','native-panic-hunt']:
    path=root/'test_output'/f'{name}.log'
    text=path.read_text(encoding='utf-8')
    totals={'unit':[0,0,0,0],'integration':[0,0,0,0]}
    kind='unit'
    for line in text.splitlines():
        if 'Running unittests' in line:kind='unit'
        elif 'Running tests' in line:kind='integration'
        match=re.search(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',line)
        if match:
            values=list(map(int,match.groups()))+[1]
            totals[kind]=[a+b for a,b in zip(totals[kind],values)]
    result[name]={'totals_pass_fail_ignored_binaries':totals,'failed':'test result: FAILED' in text}
print(json.dumps(result,indent=2))
(root/'test_output/native-test-summary.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
