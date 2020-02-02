#!/usr/bin/env python
import subprocess
import os

fpath = __file__.replace('.py', '.txt')

def get_websites():
    with open(fpath) as f:
        for site in f.readlines():
            yield 'http://{}'.format(site)


os.environ['RUST_BACKTRACE'] = '1'

for url in get_websites():
    print(url)
    with open(os.devnull) as devnull:
        subprocess.run(['./target/debug/cabot', url], stdout=devnull)
