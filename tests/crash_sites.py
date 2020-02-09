#!/usr/bin/env python
import filecmp
import os
import os.path
import subprocess
import sys

in_path = __file__.replace('.py', '.txt')
ko_path = __file__.replace('crash_sites.py', 'out/ko.txt')
rej_path = __file__.replace('crash_sites.py', 'out/reject.txt')

os.environ['RUST_BACKTRACE'] = '1'
os.environ['RUSTLOG'] = 'cabot'

cabot_tmp = '/tmp/cabot.txt'
curl_tmp = '/tmp/curl.txt'
curl_rej_tmp = '/tmp/curl2.txt'


def clean_up():
    for file_ in (ko_path, rej_path, cabot_tmp, curl_tmp, curl_rej_tmp):
        if os.path.exists(file_):
            os.unlink(file_)


def get_websites():
    with open(in_path) as f:
        for site in f.readlines():
            if site.startswith('#'):
                continue
            site = site.strip()
            yield site, 'http://{}'.format(site)


def process_domain(domain, url, devnull):
    print(url)
    print('.', end='', flush=True)
    subprocess.run(
        [
            './target/debug/cabot',
            url,
            '--timeout',
            '30',
            '--user-agent',
            'curl/7.68.0',
            '-o',
            cabot_tmp,
        ],
        stdout=devnull,
        check=True,
    )
    print('.', end='', flush=True)
    subprocess.run(
        ['timeout', '30s', 'curl', url, '-o', curl_tmp],
        stdout=devnull,
        stderr=devnull,
    )
    print('.', end='', flush=True)

    eq = filecmp.cmp(cabot_tmp, curl_tmp)
    if not eq:
        subprocess.run(
            ['timeout', '30s', 'curl', url, '-o', curl_rej_tmp],
            stdout=devnull,
            stderr=devnull,
        )
        eq = filecmp.cmp(curl_rej_tmp, curl_tmp)
        outfile, msg = {True: (ko_path, 'KO'), False: (rej_path, 'REJ')}[eq]
        with open(outfile, 'a') as outfd:
            outfd.write(domain + '\n')
        print(f'\n{msg}')

        # if msg == 'KO':
        #     subprocess.run(['diff', cabot_tmp, curl_tmp])
        #     print('')
        #     print(url)
        #     print('meld', cabot_tmp, curl_tmp)
        #     sys.exit(1)

        os.unlink(cabot_tmp)
        os.unlink(curl_tmp)
        os.unlink(curl_rej_tmp)

    else:
        print('\nOK')
        os.unlink(cabot_tmp)
        os.unlink(curl_tmp)


def main():
    clean_up()
    return
    with open(os.devnull) as devnull:
        for (domain, url) in get_websites():
            try:
                process_domain(domain, url, devnull)
            except KeyboardInterrupt:
                if not input('Continue: Y/n ?').startswith('n'):
                    continue

            except Exception as err:
                print(err)


main()
