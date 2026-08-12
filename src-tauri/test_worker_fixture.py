import os
import sys
import time

import json


mode = os.environ.get('ACTAVOCES_TEST_WORKER_MODE', 'normal')
for line in sys.stdin:
    command = json.loads(line)
    command_id = command['id']
    if mode == 'exit':
        raise SystemExit(7)
    if mode == 'malformed':
        print('not json', flush=True)
        continue
    if mode == 'timeout':
        time.sleep(5)
        continue
    if mode == 'serialized':
        print(json.dumps({'commandId': command_id, 'event': 'started', 'payload': {}}), flush=True)
        time.sleep(0.1)
    print(json.dumps({'commandId': command_id, 'event': 'health.ok', 'payload': {'pid': os.getpid()}}), flush=True)
    print(json.dumps({'commandId': command_id, 'event': 'command.finished', 'payload': {}}), flush=True)
