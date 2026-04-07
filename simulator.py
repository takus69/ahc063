import numpy as np
from tqdm import tqdm
import pandas as pd
import time
import multiprocessing
import subprocess
import json
import datetime


def run(i):
    # output_str = subprocess.run(f'powershell cat in/{i:04}.txt | .\\target\\debug\\ahc063.exe > out/{i:04}.txt', shell=True, capture_output=True, text=True).stderr
    output_str = subprocess.run(f'powershell cat in/{i:04}.txt | .\\target\\release\\ahc063.exe > out/{i:04}.txt', shell=True, capture_output=True, text=True).stderr
    # print('output_str:', output_str.split('\n'))
    result = json.loads(output_str.split('\n')[-2])
    return result


def main(i):
    start = time.time()
    # print(i, 'start')
    r = run(i)
    t = round(time.time()-start, 4)
    data = {'i': i, **r, 'time': t}
    print('\r', 'end', i, end='')
    # print(i, 'end')
    return data


def run_simulate(trial=200):
    start_wall = datetime.datetime.now()
    start = time.perf_counter()
    print(f"start time: {start_wall.strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]}")
    '''
    result = []
    for i in tqdm(range(trial)):
        data = main(i)
        result.append(data)
    '''
    processes = multiprocessing.cpu_count()
    with multiprocessing.Pool(processes=processes) as pool:
        data = [pool.apply_async(main, (i,)) for i in range(trial)]
        result = [d.get() for d in data]
    print()
    # '''
    df = pd.DataFrame(result)
    columns = [
        'i',
        'score',
        'k',
        'm',
        'e',
        't',
        'prefix_len',
        'remaining_food',
        'completed',
        'full_length',
        'escape_bite_count',
        'rebuild_bite_count',
        'safe_collect_count',
        'forced_safe_collect',
        'used_safe_branch',
        'elapsed_ms',
        'time',
    ]
    for column in columns:
        if column not in df.columns:
            df[column] = np.nan
    df = df[columns]
    score = np.mean(df['score'])
    sum_score = score * 50
    print(f"score: {format(int(sum_score), ',')}, score mean: {format(int(score), ',')}")
    df.to_csv('result.csv', index=False)
    end_wall = datetime.datetime.now()
    elapsed_ms = (time.perf_counter() - start) * 1000
    print(f"end time: {end_wall.strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]}, elapsed: {elapsed_ms:.1f}ms")
    return score


if __name__ == '__main__':
    run_simulate()
