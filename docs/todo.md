# todo

## 停止条件の確認

[ ] `result` JSON に `stop_reason` を追加する
[ ] SafeCollect 中の bite 候補評価を実装する
[ ] SafeCollect の回収経路を改善する

## 安全モード改善（E>0を改善）

[ ] SafeCollect を終盤または連続失敗時だけ使う条件を調整する
[ ] 盤面都合で少し頭側から切る再修復案を比較する
[ ] bite 再構成で `E` がどこまで下がるかを評価する

## Greedy改善（E=0を増やす）

## 貪欲強化

[x] 同じ target への複数経路候補を作れるようにする
[x] 探索順の違う BFS で経路候補を複数生成する
[x] 同じ餌に対する経路候補を比較して選ぶ
[x] 経路候補の比較に次の餌への戻りやすさを入れる
[x] GreedyTarget で同色餌の上位 K 候補を列挙して比較する
[x] target 候補ごとに複数経路候補を比較する
[x] 候補評価に到達可能マス数と到達可能餌数を追加する
[x] 複数経路化が効いた seed / 悪化した seed を result.csv で切り分ける
[x] GreedyFallback で候補餌を複数比較できるようにする
[x] GreedyFallback で同じ餌への複数経路候補を比較する
[x] GreedyFallback の候補数を小さく制限して計算量を抑える
[x] release 実行で GreedyFallback 強化の効果を確認する

## 分析用出力

[x] `result` JSON に最終状態の要約項目を追加する
[x] `result` JSON に bite / safe / branch の回数系項目を追加する
[x] `result` JSON に `completed` / `full_length` を追加する
[x] simulator.py で追加項目を CSV に出せるようにする

## bite改善

[x] 脱出用 bite の候補順を「頭から近い順」に変更する
[x] 脱出用 bite 後の再展開しやすさを確認する
[x] nearest-first と現状方針の差をログで比較する

## BFS改善

[x] GreedyFallback の到達判定を GreedyTarget と同じ動的到達可能性ベースに揃える
[x] 胴体マスの `cell_open_turn` を使う BFS 共通核を整理する
[x] GreedyTarget / GreedyFallback で停止条件だけを分ける
[x] visualizer で fallback の BFS スナップショットが意図どおりか確認する

## ビジュアライザ対応

[x] debug ビルド時のみ `debug.txt` に phase trace を出力する
[x] 再計画タイミングで `turn phase` 形式のログを書く
[x] `Phase::as_str()` を追加して phase 名を文字列化できるようにする
[x] `BfsResult` の dist 行列を debug 用テキストへ変換する関数を追加する
[x] debug 出力ファイルのブロック形式を `TURN / PHASE / TARGET_COLOR / TARGET / BFS / END` に拡張する
[x] GreedyTarget の再計画時に、実際に使った BFS スナップショットを debug ファイルへ追記する
[x] GreedyFallback の再計画時に、実際に使った BFS スナップショットを debug ファイルへ追記する
[x] debug ビルド時のみ BFS スナップショットを出力し、release では一切出さない
[x] stdout は提出形式の U/D/L/R のみを維持することを確認する

## 状態管理と安全モード

[x] safe branch を起動する条件を実装する
[x] safe branch 用に best snapshot を保持する仕組みを実装する
[x] safe branch で SafeCollect を最後まで実行する処理を実装する
[x] 最終出力で本線結果と safe branch 結果の良い方を採用する

## M=k, E=0 達成戦略

[x] 最短の次色を取りに行く greedy を安定動作させる
[x] 次色が取れない場合に近い色を取る fallback を実装する
[x] greedy / fallback でも進展しない場合の判定条件を実装する
[x] 進展しない場合に安全回収モードへ切り替える
[x] 安全回収モードで端沿いまたはジグザグ系ルートで残餌を回収する
[x] 上記方針で `M = k` を安定して達成できるか確認する
[x] `M = k` 到達後に噛み切りを開始するベースラインを実装する
[x] 最長 prefix を残す噛み切り位置の選択を実装する
[x] no-bite で進めない場合に脱出用 bite を打つ判定を実装する
[x] 脱出用 bite の後に通常 greedy へ戻す遷移を実装する

## greedy による色優先構築

[x] 目標色探索では別色の餌を障害物として扱う BFS を実装
[x] 目標色探索では最初に到達した同色餌で停止する処理を実装
[x] 欲しい色に届かない場合は最短の任意餌を選ぶ fallback を実装
[x]fallback でも途中の餌を通過しない経路復元にする
[x] greedy 初版の `M-k` と `E` の悪化要因を simulator で再確認する
[x] 現在位置から到達可能なマスを BFS で列挙できるようにする
[x] 次に欲しい色 `d_k` の餌マスを BFS 距離で最短選択する処理を実装
[x] 選んだ餌までの移動列を復元して出力に追加する

## 実行環境の整備

[x] ジグザグで全てのエサを食べるベースラインを実装
[x] 最終状態から絶対スコアを計算する `score()` を実装
[x] ジグザグ出力に対して `score()` の値を手計算または既知ケースで検証
[x] `simulator.py` を実行して出力と最終状態を確認
[x] `README.md` 記載の実行コマンドがそのまま動くか確認
[x] `score()` の結果と simulator の結果が一致するか確認

