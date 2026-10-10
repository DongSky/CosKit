"""Synthesize public tutorial text using Edge neural TTS; never fall back silently.

Install edge-tts in a separate venv, then pass that interpreter to the renderer.
Only storyboard text is sent to the online speech service.
"""
import argparse
import asyncio
import hashlib
import json
from pathlib import Path
import subprocess

import edge_tts


async def synthesize(args):
    directory = args.directory.resolve()
    segments = json.loads((directory / 'storyboard.json').read_text(encoding='utf-8'))
    manifest = {'provider': 'Microsoft Edge online neural TTS', 'voice': args.voice,
                'rate': args.rate, 'segments': []}
    for segment in segments:
        name = segment['id']
        if not name or any(c not in 'abcdefghijklmnopqrstuvwxyz0123456789-_' for c in name):
            raise ValueError('Unsafe segment id')
        text = segment['zh']
        fingerprint = hashlib.sha256((args.voice + args.rate + text).encode()).hexdigest()
        mp3 = directory / f'{name}-neural.mp3'
        cache = directory / f'{name}-neural.json'
        timing = directory / f'{name}-timing.json'
        cached = json.loads(cache.read_text()) if cache.exists() else {}
        if not mp3.exists() or not timing.exists() or cached.get('fingerprint') != fingerprint:
            temporary = mp3.with_suffix('.tmp.mp3')
            cues = []
            with temporary.open('wb') as audio:
                async for chunk in edge_tts.Communicate(text, args.voice, rate=args.rate).stream():
                    if chunk['type'] == 'audio':
                        audio.write(chunk['data'])
                    elif chunk['type'] == 'SentenceBoundary':
                        cues.append({'start': chunk['offset'] / 10000000,
                                     'end': (chunk['offset'] + chunk['duration']) / 10000000,
                                     'text': chunk['text']})
            if not cues or temporary.stat().st_size == 0:
                raise RuntimeError(f'Missing speech or subtitle timings: {name}')
            temporary.replace(mp3)
            timing.write_text(json.dumps(cues, ensure_ascii=False), encoding='utf-8')
            cache.write_text(json.dumps({'fingerprint': fingerprint}), encoding='utf-8')
        subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-i', str(mp3),
                        '-ar', '48000', '-ac', '1', '-map_metadata', '-1',
                        str(directory / f'{name}.wav')], check=True)
        manifest['segments'].append({'id': name, 'fingerprint': fingerprint})
        print(f'Narration ready: {name}', flush=True)
    (directory / 'voice-manifest.json').write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding='utf-8')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--voice', default='zh-CN-XiaoxiaoNeural')
    parser.add_argument('--rate', default='-4%')
    asyncio.run(synthesize(parser.parse_args()))
