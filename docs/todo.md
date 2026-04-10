# todo

## 最優先

[x] 候補評価で距離・手数の重要性を上げる
[ ] bite 候補評価に食べ直し可能性を入れる
[ ] bite 候補評価に食べ直しまでの追加手数を入れる
[ ] GreedyTarget/Fallback の長い plan を一定手数ごとに再評価する

## 上記に付随する改善

[ ] 一定手数ごとの再評価で同じ経路に戻りすぎないよう、BFS 候補生成に seed 固定の多様性を入れる
[ ] BFS 候補は初手 4 方向を必ずカバーし、残り順序は seed 固定乱択にする
[ ] 候補評価で「同じ状態に回復可能なら短い経路を優先する」を入れる
[ ] 候補評価で詰まりやすさや出口の少なさを罰する
[ ] 候補評価に胴体が餌密集地を塞ぐかを入れる

## snapshot / 着地品質の改善

[ ] `main_prefix_snapshot` の seed を特定し、残り餌を取れなかった理由を visualizer で切り分ける
[ ] `main_prefix_snapshot` が採用される条件を確認し、full-length を崩しにくいように保守化する
[ ] `main_full_length_snapshot` の代表 seed を確認し、completed に届かない要因を分類する
[ ] `main_full_length_snapshot` から completed に寄せる小改善を 1 つ検討する
[ ] snapshot の保存・採用バランスを調整する

## safe branch / SafeCollect 改善

[ ] `safe_branch_full_length` の代表 seed を確認し、bite 後の continuation と回収経路のどちらが弱いかを切り分ける
[ ] `safe_branch_full_length` の質改善候補を再整理する
[ ] safe branch の回収途中で軽い greedy 復帰を試せるようにする
[ ] safe branch の bite -> zigzag をさらに改善する
[ ] release 200 ケースで stop_reason の遷移を継続監視する

## bite 再構成

[ ] 盤面都合で少し頭側から切る再修復案を比較する
[ ] bite 再構成で `E` がどこまで下がるかを評価する

## 評価関数ベース化 / 将来の探索

[ ] GreedyTarget/Fallback の候補評価を 1 つの評価関数に整理する
[ ] 評価関数に `prefix_len`、次の target/fallback 可否、到達可能餌数、到達可能マス数を入れる
[ ] 候補生成と候補評価を分離する
[ ] greedy では評価関数最大の候補を選ぶ形に整理する
[ ] 評価関数を beam search の枝選択に流用できる形にする
[ ] bite 位置や助走を近傍として hill climbing/SA に流用できる形にする
[ ] rollout の末端評価に同じ評価関数を使えるようにする


## E=0 後のスコア改善

[ ] 残り餌が少ない局面だけ no-bite 完走探索を入れる
[ ] completed 近傍で不要な bite を避ける tie-break を入れる
[ ] completed ケースの `T` を下げる endgame 評価を入れる

## snapshot 改善

[x] bite を含む plan の適用直前に snapshot 候補を保存する
[x] `prefix_len` 更新時ではなく score 改善時に snapshot 候補を保存する
[x] `prefix_len` 保存と score 保存のどちらが良いか release 200 ケースで比較する
[x] `main_prefix_snapshot` を減らせるか確認する

## 連続一致 greedy 改善

[x] GreedyTarget の候補評価に「次の 2 手で連続一致できるか」を入れる
[x] 候補評価で 2 手先の連続一致数を強くボーナスする
[x] bite 直前に no-bite で連続一致できる候補を再確認する
[x] 効果があれば 3 手先まで伸ばせるか検討する

## snapshot 改善

[x] SafeCollect bite 時以外でも best snapshot を保存する条件を追加する
[x] `M = k` 到達時に snapshot を保存する
[x] `prefix_len` 更新時に snapshot を保存する
[x] `no_plan` 時に良い状態なら snapshot を保存する

## no_plan 改善

[x] `no_plan` を避けるための last resort bite を維持する
[x] last resort 候補評価で `prefix` 減少を強く罰する
[x] last resort 候補評価で次の target / fallback 可否を強く見る
[x] `no_plan` をゼロに近づけられるか release 実行で確認する

## 停止条件の確認

[x] `result` JSON に `stop_reason` を追加する

## 安全モード改善（E>0を改善）

[x] safe branch の採用条件に `E` と `prefix_len` を強く反映する
[x] SafeCollect 中の bite 候補評価を実装する
[x] SafeCollect の回収経路を改善する
[x] SafeCollect の `bite -> zigzag` を改善して色順の崩れを減らす
[x] SafeCollect を終盤または連続失敗時だけ使う条件を調整する

## Greedy改善（E=0を増やす）

[x] `stop_reason = no_plan` のケースを completed に寄せる小改善を検討する
[x] mainの流れの出力を `debug_ans.txt` に出力する
[x] BiteRebuild で改善候補が無い場合でも続行用の bite 候補を選べるようにする
[x] BiteRebuild の last resort 候補評価に到達可能餌数と到達可能マス数を入れる
[x] `no_plan` を減らせるか release 実行で確認する

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

