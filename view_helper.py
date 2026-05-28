import sys
sys.stdout.reconfigure(encoding='utf-8')
with open(sys.argv[1], 'r', encoding='utf-8') as f_in:
    lines = f_in.readlines()
    start = int(sys.argv[2]) - 1 if len(sys.argv) > 2 else 0
    end = int(sys.argv[3]) if len(sys.argv) > 3 else len(lines)
    for i, line in enumerate(lines[start:end]):
        print(f'{start+i+1:4d}: {line}', end='')
