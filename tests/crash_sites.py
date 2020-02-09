#!/usr/bin/env python
import filecmp
import os
import subprocess

fpath = __file__.replace('.py', '.txt')
os.environ['RUST_BACKTRACE'] = '1'
os.environ['RUSTLOG'] = 'cabot'


def get_websites():
    with open(fpath) as f:
        for site in f.readlines():
            if site.startswith('#'):
                continue
            yield 'http://{}'.format(site.strip())


for url in get_websites():
    print(url)
    file1 = '/tmp/cabot.txt'
    file2 = '/tmp/curl.txt'

    with open(os.devnull) as devnull:
        print('.', end='', flush=True)
        subprocess.run(
            [
                './target/debug/cabot',
                url,
                '--dns-timeout',
                '30',
                '--read-timeout',
                '30',
                '-o',
                file1,
            ],
            stdout=devnull,
        )
        print('.', flush=True)
        subprocess.run(
            ['curl', url, '-o', file2], stdout=devnull, stderr=devnull,
        )

    eq = filecmp.cmp(file1, file2)
    if not eq:
        subprocess.run(['diff', file1, file2])
        print('')
        print(url)
        print('meld', file1, file2)
        break
    else:
        print('OK')
        os.unlink(file1)
        os.unlink(file2)
