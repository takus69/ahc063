# log

## 2026-04-04

変更: 列ジグザグで盤面全体を 1 回ずつたどるベースラインを `src/main.rs` に実装。

実験:
- `cargo check`
- `cargo build`
- PowerShell で合法なランダムケース 5 件を生成し、出力手順が想定した列ジグザグ経路と一致することを確認
- 同じ 5 件について、列ジグザグ経路上で全エサ回収後の絶対スコアを外部計算

結果:
- 5/5 ケースで想定した操作列と一致
- 5/5 ケースで全エサ回収により `k = M`
- スコア: seed1 `190095`, seed2 `540191`, seed3 `390095`, seed4 `1200220`, seed5 `240116`
- 平均スコア: `512143.4`

考察:
- 全食べを安定して達成するベースラインとしては機能した
- 一方で色順は考慮していないため `E` が大きく、スコアの大半を不一致ペナルティが占める

変更: `score()` を実装し、操作列から移動・捕食・噛み切りを再現して絶対スコアを計算するようにした。

実験:
- `cargo check`
- `cargo build`
- PowerShell で合法なランダムケース 5 件を生成し、solver 実行後に stderr の JSON スコアを取得

結果:
- 5/5 ケースで solver が最後まで実行され、スコア JSON を出力
- スコア: seed11 `220139`, seed12 `660251`, seed13 `660139`, seed14 `280059`, seed15 `370164`
- 平均スコア: `438150.4`

考察:
- solver 内で最終状態から絶対スコアを返せるようになり、以後の改善比較の基盤は整った
- ただし `score()` の値が外部計算や simulator と一致するかの照合はまだ未実施で、次タスクで確認が必要

変更: `score()` の内部処理を `simulate()` と `apply_move()` に分離し、蛇状態をシミュレーションした結果からスコアを計算する形に整理。

実験:
- `cargo check`
- `cargo build`
- PowerShell 生成の合法ランダムケース 3 件で solver 実行後のスコア JSON 出力を確認

結果:
- 3/3 ケースで solver が最後まで実行され、スコア JSON を出力
- スコア: seed21 `1060191`, seed22 `180076`, seed23 `820191`

考察:
- `score()` は最終色列との比較だけを担当し、状態更新の責務が分離された
- 今後は `simulate()` の返り値を見れば途中状態や最終状態を追いやすい

変更: 現在の蛇状態を障害物付きグリッドとして扱う BFS 基盤を追加し、到達可否・距離・親・移動列復元を保持する `BfsResult` を実装。

実験:
- `cargo check`
- `cargo build`
- 手製の合法 1 ケースで solver を実行し、既存のジグザグ出力手数とスコア JSON 出力を確認

結果:
- コンパイル成功
- 手製ケースで操作数 `59` を出力し、既存のジグザグベースライン出力は維持
- 同ケースでスコア JSON `70059` を出力

考察:
- BFS はまだ greedy 本体に未接続のため未使用警告は出るが、基盤としては追加できた
- 現在状態の頭以外の蛇マスを障害物にしているため、初手の U ターン禁止も自然に満たせる

変更: `BfsResult` の距離表を使って、欲しい色の餌マスから最短距離の候補を 1 つ選ぶ `FoodTarget` と選択処理を追加。

実験:
- `cargo check`
- `cargo build`
- 手製の合法 1 ケースで solver を実行し、既存のジグザグ出力手数とスコア JSON 出力を確認

結果:
- コンパイル成功
- 手製ケースで操作数 `59` を出力し、既存のジグザグベースライン出力は維持
- 同ケースでスコア JSON `70059` を出力

考察:
- 選択処理は「到達可能かつ指定色の餌」を BFS 距離最小で選ぶ最小実装として追加できた
- 同距離の候補は座標の辞書順で決めており、挙動は決定的になっている

変更: `solve()` で BFS と色選択を使い、選んだ餌までの移動列を復元して `ops` に追加するように変更。

実験:
- `cargo check`
- `cargo build`
- 手製の合法 2 ケースで solver を実行し、復元移動列が出力に反映されることを確認

結果:
- コンパイル成功
- case1: 欲しい色が隣接するケースで操作数 `1`、出力 `D`、スコア `1`
- case2: 複数餌ケースで操作数 `40`、先頭 8 手 `RRDDDLLU`、スコア `70040`

考察:
- 復元した移動列がそのまま出力へ反映され、色優先 greedy の最小ループとして動作し始めた
- 到達不能な欲しい色しか残っていない場合はその時点で停止する単純挙動で、fallback はまだ持たない

変更: 目標色探索専用の BFS を追加し、目標色以外の餌マスを障害物として扱うようにした。

実験:
- `cargo check`
- `cargo build`
- 手製の合法 2 ケースで solver を実行し、別色餌を踏み抜かないことを確認

結果:
- コンパイル成功
- case_block: 直下の別色餌を避けて `RDDL` を出力、操作数 `4`、スコア `4`
- case_direct: 直下が目標色のケースで `D` を出力、操作数 `1`、スコア `1`

考察:
- 目標色探索では別色餌を障害物として扱えており、途中の誤捕食を 1 段抑えられた
- まだ同色餌の通過停止は入れていないため、次タスクで「最初に到達した同色餌で止まる」条件を足す

変更: 目標色探索 BFS で、同色餌に到達したマスから先へは展開しないようにして「最初に到達した同色餌で止まる」条件を追加。

実験:
- `cargo check`
- `cargo build`
- 手製の合法 2 ケースで solver を実行し、同色餌での停止と別色餌回避を確認

結果:
- コンパイル成功
- case_same_stop: 最初の `2` を食べたあと、次の目標 `3` に向けて `DRDDL` を出力、操作数 `5`、スコア `5`
- case_block_again: 別色餌を避けて `RDDL` を出力、操作数 `4`、スコア `4`

考察:
- 目標色探索では最初に到達した同色餌で一度停止して再計画できるようになった
- まだ欲しい色に届かない場合の fallback は無いため、次は任意餌への退避を入れて `M-k` 悪化を抑えたい

変更: 欲しい色に届かない場合、汎用 BFS 上で最短の任意餌を選んで食べに行く fallback を `solve()` に追加。

実験:
- `cargo check`
- `cargo build`
- 手製の合法 2 ケースで solver を実行し、target 到達時と fallback 時の両方を確認

結果:
- コンパイル成功
- case_target: 目標色に直接届くケースで `D` を出力、操作数 `1`、スコア `1`
- case_fallback_wall: 目標色 `3` が別色餌の壁で遮られるケースで `DRRRRRRRDLLLLLLL` を出力、操作数 `16`、スコア `20016`

考察:
- 欲しい色に届かないときでも即停止せず、まず reachable な餌を食べて `k` を伸ばす fallback が入った
- fallback 経路はまだ汎用 BFS を使っており、途中の餌を通過する可能性は残るため、次タスクでそこを詰める

変更: fallback 用にも「最初に到達した餌で止まる」BFS を追加し、途中の餌を通過しない経路復元に差し替えた。

実験:
- `cargo check`
- `cargo build`
- 手製の合法 2 ケースで solver を実行し、target 到達時と fallback 時の動作維持を確認

結果:
- コンパイル成功
- case_target: 目標色に直接届くケースで `D` を出力、操作数 `1`、スコア `1`
- case_fallback_wall: 目標色 `3` が別色餌の壁で遮られるケースで `DRRRRRRRDLLLLLLL` を出力、操作数 `16`、スコア `20016`

考察:
- fallback でも「最初に到達した餌で一度止まって再計画する」探索へ統一できた
- 次は simulator を回して、`M-k` と `E` の悪化要因がどこまで減ったかを確認したい
変更: `solve()` を phase ベースの骨組みに整理し、`Phase` / `Plan` / progress 小関数を追加した。既存の greedy target と fallback は専用 planner 入口に分離し、SafeCollect / BiteRebuild は暫定で fallback に寄せた。
実験:
- `cargo check`
- `cargo build`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `220026` を出力
- 出力手数は `26`、先頭 8 手は `D R D R U R U L`
考察:
- 既存の greedy / fallback 挙動を保ったまま、phase 選択と plan 適用を分ける最小構造整理は入れられた
- `prefix_len` / `remaining_food_count` / `can_reach_any_food` を追加したので、次の TODO で進展判定や SafeCollect 切替条件を載せやすくなった

変更: fallback 中に prefix が伸びない状態を数える進展判定を追加した。`fallback_stall_count` と `is_stalled` を progress に保持し、SafeCollect 切替で再利用できる骨組みにした。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `220026` を出力
- `out/0000.main.txt` の先頭 12 手は `D R D R U R U L U U U U`
考察:
- 今回は stalled 判定の追加だけで、phase 切替にはまだ使っていないため既存挙動は維持できている
- 次の TODO で `is_progress_stalled()` を使えば、GreedyFallback から SafeCollect への切替条件を最小差分で追加できる

変更: stalled 判定が立ったときに `choose_phase()` が `SafeCollect` を返すようにした。SafeCollect planner はまだ fallback 相当の仮実装なので、今回の差分は phase 切替条件の追加に留めた。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `220026` を出力
- `out/0000.main.txt` の先頭 12 手は `D R D R U R U L U U U U`
考察:
- stalled 条件が立ったときに SafeCollect へ切り替える入口は追加できた
- ただし SafeCollect の経路自体はまだ fallback 相当なので、観測上の挙動差分は次の TODO で初めて出る

変更: SafeCollect planner に、現在位置から列ジグザグ経路の前後どちらかへ沿って最初の餌まで進む回収ルートを追加した。ジグザグ進行が現在の胴体で塞がる場合だけ既存 fallback に戻す形にした。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- 手製 stall ケースを PowerShell here-string で `cargo run --quiet`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `220026` を出力
- 手製 stall ケースでは出力 `D D D R`、score JSON `40004`
考察:
- SafeCollect に「端沿い / ジグザグ系で餌を拾う」最小実装を入れられた
- 今回は現在の胴体を固定障害物として扱う簡略版なので、ジグザグ経路が塞がる場合は fallback に戻して安全側に倒している

変更: SafeCollect を「まず深い合法 bite を 1 手入れ、無理なら既存ジグザグ回収へ戻る」形に変更した。no-bite で target / fallback が進まないときは SafeCollect を有効化するようにした。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- 手製 safe-collect ケースを PowerShell here-string で `cargo run --quiet`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `100050` を出力
- 手製 safe-collect ケースでも最後まで実行され、score JSON `50019` を出力
考察:
- no-bite で進めないときに SafeCollect へ移り、bite 候補を返す escape hatch は入れられた
- bite 候補は「残る長さ最大、同点なら prefix 長、さらに到達可能餌ありを優先」の最小評価で選んでいる

変更: SafeCollect を「no-bite で行ける餌が無くなったら入り、その後は詰むたびに頭から最も遠い胴体へ 1 手 bite してからジグザグ回収する」形に修正した。SafeCollect 中はこの bite とジグザグ回収を繰り返し、全回収まで継続する。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- 手製 safe-collect ケースを PowerShell here-string で `cargo run --quiet`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `100050` を出力
- 手製 safe-collect ケースでは出力 `D D D R R`、score JSON `50005`
考察:
- SafeCollect の bite 選択は「今すぐ噛める胴体のうち頭から最も遠い位置」を選ぶ単純規則に揃えられた
- これで no-bite の詰み状態でも、bite -> ジグザグ回収 -> 再び詰んだら bite の繰り返しに入れる

変更: SafeCollect の bite を「隣接 1 手で噛める胴体」ではなく、「空きマスを BFS でたどって到達できる胴体のうち、頭から最も遠い位置」に変更した。bite 後は既存のジグザグ回収へ戻し、また詰んだら同じ BFS bite を繰り返す。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- 手製 safe-collect ケースを PowerShell here-string で `cargo run --quiet`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `90067` を出力
- 手製 safe-collect ケースでは出力 `D D D R R`、score JSON `50005`
考察:
- SafeCollect の bite が「頭の近くに胴体が無いから噛めない」ケースを避けやすくなった
- まだ 1 ケースの軽い確認だけなので、`M = k` 到達率の確認は次の TODO でまとめて見る必要がある

変更: `M = k` 到達後に prefix がまだ短い場合は `BiteRebuild` に入り、最長 prefix を残す bite を選んで通常 greedy に戻るベースラインを追加した。bite 候補は既存の body-target BFS を流用して列挙し、`prefix_len` 最大、同点なら bite 後の長さ最大で選ぶ。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `M = k` 後の bite を見る手製ケースを PowerShell here-string で `cargo run --quiet`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `80258` を出力
- `out/0000.main.txt` の出力ファイル長は `1550` bytes
- 手製 `M = k` ケースでは出力 `R U L U R D D`、score JSON `10007`
考察:
- `M = k` に達したあとも solver が停止せず、`BiteRebuild` へ入る最小ベースラインは立ち上がった
- bite 後に「現在の長さ = 正しい prefix 長」となる候補だけに絞っているので、その後は通常 greedy に戻しやすい

変更: `BiteRebuild` で、最長 prefix 固定だけでなく「少し頭側から切って通常 greedy / fallback に戻したときの prefix 伸び」も比較するようにした。bite 候補は 1 回の復帰後 prefix 長を最優先、同点なら bite 直後の prefix 長、その次に長さで比較する。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- 手製 `M = k` ケースを PowerShell here-string で `cargo run --quiet`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `80258` を出力
- 手製 `M = k` ケースでは出力 `R U L U R D D`、score JSON `10007`
考察:
- bite 候補比較に「少し頭側から切って再修復する案」の評価軸は入れられた
- 今回の軽い確認では選ばれる候補は従来と同じで、スコア差分は出なかった

変更: `choose_phase()` の前半判定を見直し、SafeCollect へ入る条件を「no-bite で target / fallback のどちらも plan できず、到達可能な餌がない場合」に限定した。stalled 条件では SafeCollect に入らないようにした。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `80258` を出力
考察:
- SafeCollect を「最後の手」に下げる判定の土台は入れられた
- 脱出用 bite の後に greedy へ戻す遷移はまだ未実装なので、実際の運用改善は次タスクで確認する

変更: 脱出用 `SafeCollect` bite の直後だけ `safe_collect_active` を解除し、次の反復で通常の `GreedyTarget / GreedyFallback` 判定へ戻る遷移を追加した。`Plan` に遷移フラグを持たせ、SafeCollect の bite のみ `resume_greedy_after_apply = true` にした。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `276` を出力
- 出力ファイル長は `1658` bytes、先頭 12 手は `D R D R U R U L U U U U`
考察:
- 脱出用 bite の後に SafeCollect に張り付かず、通常 greedy に戻す土台は入れられた
- 軽い確認ではスコアが大きく改善しており、方針変更の方向性は良さそう

変更: SafeCollect を保険に下げる条件を追加した。`1.9 sec` 経過、操作数 `10000` 到達、または脱出用 bite を 3 回連続で使った場合だけ SafeCollect を sticky にするようにした。
実験:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver が最後まで実行され、stderr に score JSON `276` を出力
考察:
- 通常時は「脱出用 bite -> すぐ greedy 復帰」、終盤や連続失敗時だけ SafeCollect へ落とす条件の土台は入れられた
- 軽い確認ではスコアは据え置きで、少なくとも既存の良いケースは壊していない
変更: `10000` 手または `1.9 sec` 超えで `force_safe_collect_mode` に入り、以降は `E` を諦めて SafeCollect を純ジグザグ回収モードに固定するよう修正した。強制 SafeCollect 中は `M = k` 到達で停止する。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver は最後まで動き、stderr に score JSON `276` を出力
- 強制 SafeCollect の発火条件を `10000` 手 / `1.9 sec` のみに絞り、escape bite 回数では発火しないようにした
考察:
- 終盤は `BiteRebuild` を諦めて、純ジグザグ回収でまず `M = k` だけを取りに行く挙動になった
- 閾値を跨ぐ長いケースでの `M = k` 到達率確認は次回の評価タスクで見る
変更: 強制 SafeCollect 中も、ジグザグが次の餌を拾えない場合は「頭から最も遠い胴体」への deepest bite を再開し、その後またジグザグへ戻るようにした。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver は最後まで動き、stderr に score JSON `276` を出力
- forced-safe 中の `plan_safe_collect()` は `zigzag -> deepest bite -> zigzag ...` を繰り返せるようになった
考察:
- これで「ジグザグ 1 本が詰まったらそのまま停止する」挙動は解消した
- 実際に最後の餌取り切りが増えるかは、詰まりケースでの追加確認が必要
変更: safe branch の起動条件だけを先に実装した。本線が通常の escape bite を打つ直前、つまり `Phase::SafeCollect` かつ `resume_greedy_after_apply = true` で、まだ `M = k` 前かつ forced-safe ではない場合だけ branch 対象にする。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
結果:
- コンパイル成功
- `in/0000.txt` で solver は最後まで動き、stderr に score JSON `276` を出力
- 今回は起動条件の骨組みだけなので、出力挙動は据え置き
考察:
- safe branch を毎ターンではなく、escape bite の節目だけで起動する条件を `solve()` に置けた
- 次の TODO でこの条件位置に state clone と snapshot 保存をそのまま差し込める
変更: safe branch 用の `best snapshot` 保持機構を追加した。`OutputSnapshot` に `ops` と `score` を持たせ、任意の操作列を `score` 比較で保持できるようにした。今回は safe branch 本体はまだ無く、本線の最終結果を baseline として snapshot に積むだけに留めた。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Get-Content in/0003.txt | cargo run --quiet > out/0003.main.txt`
結果:
- コンパイル成功
- `0000` の score JSON は `276`
- `0003` の score JSON は `1994413`
- best snapshot 保持機構の追加による出力挙動の変化はなし
考察:
- 次の TODO では、safe branch を実行した結果の `ops` をこの snapshot に流し込めばよい形になった
- まだ最終出力への採用はしていないため、現時点では purely infrastructure 追加である
変更: safe branch で SafeCollect を最後まで実行する処理を追加した。escape bite の節目で現在状態を clone し、`zigzag -> deepest bite` を繰り返して `M = k` まで進めた branch の `ops` を best snapshot に流し込む。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Get-Content in/0003.txt | cargo run --quiet > out/0003.main.txt`
結果:
- コンパイル成功
- `0000` の score JSON は `276`
- `0003` の score JSON は `1994413`
- safe branch は内部で実行されるが、最終出力への採用はまだ未実装なので表の出力は据え置き
考察:
- 次の TODO では、保持済みの best snapshot と本線結果を比較して、良い方の `ops` を採用すればよい
- `0003` のような中途半端ケースでも、escape bite 時点から safe branch の保険解を作れる土台ができた
変更: 最終出力で本線結果と safe branch 結果の良い方を採用するようにした。`solve()` の最後で `best_snapshot` を見て、保持済みの最良 `ops` を `self.ops` に反映する。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Get-Content in/0003.txt | cargo run --quiet > out/0003.main.txt`
結果:
- コンパイル成功
- `0000` の score JSON は `276` で据え置き
- `0003` の score JSON は `1994413 -> 513364` に改善
- safe branch の保険解が本線より良いケースでは、そのまま最終出力に採用されるようになった
考察:
- `0003` のように本線が中途半端に止まるケースで、safe branch 採用が効くことを確認できた
- 以降は safe branch の起動条件や SafeCollect 自体を改善すると、そのまま最終解の改善につながる
変更: debug ビルド時のみ `debug.txt` に phase trace を出すようにした。`solve()` 開始時にファイルを初期化し、各再計画タイミングで `turn phase` 形式を追記する。release ビルドでは phase trace 用の処理は no-op にしている。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Get-Content debug.txt -Head 8`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
結果:
- コンパイル成功
- debug 実行後の `debug.txt` 先頭は `0 GreedyTarget`, `2 GreedyTarget`, ... の形式で出力
- debug 実行時の stderr の score JSON は `21607`
- release 実行では `debug.txt` の更新時刻は変わらず、phase trace は出力されなかった
考察:
- stdout は提出形式の操作列のまま、stderr は score JSON のままで壊れていない
- tools 側が読む `turn phase` 形式の最小トレース基盤は入ったので、次は phase 以外の項目が必要になった時だけ拡張すればよい
変更: GreedyTarget / GreedyFallback の再計画時に、solver が実際に使った BFS 距離行列を `debug.txt` に block 形式で追記するようにした。`Phase::as_str()` と `BfsResult::dist_debug_text()` を追加し、debug ビルド時のみ `TURN / PHASE / TARGET_COLOR / TARGET / BFS / END` を出力する。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Select-String -Path debug.txt -Pattern "^TURN |^PHASE |^TARGET_COLOR |^TARGET |^BFS$|^END$" | Select-Object -First 18`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
結果:
- コンパイル成功
- debug 実行後の `debug.txt` に `TURN 0 / PHASE GreedyTarget / TARGET_COLOR 2 / TARGET 5 1 / BFS / ... / END` の block が追記された
- debug 実行時の stderr の score JSON は `21607`
- release 実行では `debug.txt` の更新時刻は変わらず、BFS スナップショットは出力されなかった
考察:
- visualizer 側は solver が実際に使った BFS をそのまま表示できるようになり、再計算由来のズレを切り分けやすくなった
- phase trace の plain line も残しているため、既存の phase 表示を壊さずに block 形式を追加できている
変更: GreedyFallback の BFS 到達判定を、GreedyTarget と同じ `cell_open_turn` ベースに揃えた。将来空く胴体マスは `arrival_turn >= cell_open_turn` なら通れるようにし、phase 差分は「最初に到達した餌で停止する」条件だけに残した。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Select-String -Path debug.txt -Pattern "^PHASE GreedyFallback$" | Select-Object -First 5`
結果:
- コンパイル成功
- `0000` の score JSON は `21607 -> 20087` に改善
- `debug.txt` には `PHASE GreedyFallback` の block が引き続き出力され、新しい fallback BFS が debug 出力にも反映された
考察:
- GreedyTarget / GreedyFallback で「将来空く胴体マスは通れる」という到達可能性の世界観が揃った
- これで static block 由来の不必要な SafeCollect / bite 落ちが減る方向になり、次の共通核整理にもつなげやすくなった
変更: 脱出用 bite の候補順を「頭から遠い順」から「頭から近い順」に変更した。前半の escape bite では最初に見つかった合法候補をそのまま採用し、BiteRebuild 側の評価は変更していない。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Get-Content in/0003.txt | cargo run --quiet > out/0003.main.txt`
結果:
- コンパイル成功
- `0000` の score JSON は `170`
- `0003` の score JSON は `631910`
- stdout は提出形式の操作列のまま、stderr は score JSON のままだった
考察:
- 変更は脱出用 bite の走査順だけなので、nearest-first と現状方針の差を比較しやすい状態になった
- BiteRebuild や SafeCollect 全体の設計には触れていないため、差分の原因は escape bite の候補順にほぼ限定される
変更: `result` JSON に `k`, `m`, `e`, `t`, `prefix_len`, `remaining_food`, `completed`, `full_length`, `escape_bite_count`, `rebuild_bite_count`, `safe_collect_count`, `forced_safe_collect`, `used_safe_branch`, `elapsed_ms` を追加した。最終状態から求まる項目は `result()` で計算し、回数系は `Solver` のカウンタと snapshot 側のメタデータで持つようにした。`simulator.py` は JSON をそのまま列展開して CSV に保存する形へ変更した。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `python -c "import simulator; print(simulator.main(0))"` 失敗
- `py -3 -c "import simulator; print(simulator.main(0))"` 失敗
結果:
- コンパイル成功
- `0000` の stderr JSON は `{"score":170,"k":30,"m":30,"e":0,"t":170,"prefix_len":30,"remaining_food":0,"completed":true,"full_length":true,"escape_bite_count":1,"rebuild_bite_count":2,"safe_collect_count":1,"forced_safe_collect":false,"used_safe_branch":false,"elapsed_ms":86}` 相当の 1 オブジェクトで出力された
- stdout の提出形式と既存の score 取得は維持された
- `simulator.py` の実行確認は、この環境で `python` / `py` コマンドが見つからず未実施
考察:
- seed ごとに `M-k` 由来か `E` 由来か、また SafeCollect / bite / safe branch の絡みかを切り分けやすくなった
- simulator 側は DataFrame に列を増やすだけの変更なので、既存の平均 score 集計ロジックはそのまま維持している
変更: GreedyTarget で同じ target cell に対して探索順の違う BFS を複数回回し、重複しない経路候補を比較して選ぶようにした。探索順は `UDLR`, `RDLU`, `LURD`, `DRUL` の 4 通りで、候補比較は「次の target が作れるか」「次の fallback が作れるか」「その距離」「現在の prefix」「移動手数」の順に行う最小実装にした。
実行:
- `cargo check`
- `Get-Content in/0000.txt | cargo run --quiet > out/0000.main.txt`
- `Select-String -Path debug.txt -Pattern "^PHASE GreedyTarget$|^TURN " | Select-Object -First 12`
結果:
- コンパイル成功
- `0000` の score JSON は `771`
- stdout は提出形式の操作列のまま、stderr の `result` JSON も維持された
- debug ビルド実行後も `debug.txt` は更新され、既存の debug 出力経路は壊れていない
考察:
- 同じ餌でも経路の違いで次の target / fallback の作りやすさを比較できる入口ができた
- 今回は GreedyTarget だけの最小実装なので、差分要因は「経路候補の複数化と軽い後評価」に限定される
変更: GreedyTarget で同色餌の上位 3 候補を列挙し、各 target ごとに探索順の違う BFS から複数経路候補を比較するようにした。候補評価は既存の次 target / fallback の作りやすさに加え、到達可能マス数と到達可能餌数を見て再展開しやすさも少し入れた。
実行:
- `cargo check`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
- `Get-Content in/0003.txt | .\\target\\release\\ahc063.exe > out/0003.release.txt`
結果:
- コンパイル成功
- release 実行で `0000` の score JSON は `100`
- release 実行で `0003` の score JSON は `521209`
- stdout は提出形式の操作列のまま、stderr の `result` JSON も維持された
考察:
- GreedyTarget が「どの同色餌を取るか」まで比較できるようになり、同じ色でも少し遠い餌を選ぶ余地ができた
- 今回は GreedyTarget のみ対象なので、改善が出るかどうかは seed 依存で、次は `result.csv` で効いた seed / 悪化した seed を切り分けるのが自然
変更: GreedyFallback でも到達可能な餌の上位 3 候補を列挙し、各候補に対して探索順の違う BFS から複数経路候補を比較するようにした。評価は最小に留め、次の target が作れるか、次の fallback が作れるか、それぞれの距離、prefix_len、今回の手数だけを見るようにした。
実行:
- `cargo check`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
- `Get-Content in/0003.txt | .\\target\\release\\ahc063.exe > out/0003.release.txt`
結果:
- コンパイル成功
- release 実行で `0000` の score JSON は `100`
- release 実行で `0003` の score JSON は `521209`
- stdout は提出形式の操作列のまま、stderr の `result` JSON も維持された
考察:
- GreedyFallback でも「どの餌を取るか」「どの入り方で取るか」を少数候補から比較できる入口ができた
- 今回は候補数を 3 に固定した最小実装なので、次は `result.csv` で completed や full_length に効いた seed を切り分けるのが自然
変更: `solve()` の終了分岐ごとに `stop_reason` を持たせ、`result` JSON に追加した。`completed`, `force_safe_full_length`, `no_plan`, `empty_plan`, `turn_limit` を本線側で区別し、safe branch 採用時は `safe_branch_full_length` を snapshot 側から返すようにした。`simulator.py` には `stop_reason` 列を追加した。
実行:
- `cargo check`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
- `Get-Content in/0003.txt | .\\target\\release\\ahc063.exe > out/0003.release.txt`
結果:
- コンパイル成功
- `0000` の result JSON に `stop_reason: "completed"` が追加された
- `0003` の result JSON に `stop_reason: "safe_branch_full_length"` が追加された
- stdout の提出形式と既存の score / completed / full_length / 各カウンタは維持された
考察:
- seed ごとに「完全一致で終わったのか」「safe branch 由来で full length を確保したのか」「no plan 系で止まったのか」を後から切り分けやすくなった
- 次の SafeCollect 改善と Greedy 改善の優先順位付けに直接使える分析項目になった
変更: safe branch の採用条件を確認した。現状の [update_best_snapshot()](/C:/workspaces/AHC/ahc063/src/main.rs#L366) は `score_ops()` による最終絶対スコアを本線 / safe branch ともに厳密比較しており、同点時だけ操作手数で比較している。したがって、この TODO に対するコード変更は行わなかった。
実行:
- `cargo check`
結果:
- コンパイル成功
- safe branch 採用条件はすでに最終 `score` 完全比較であることを確認した
- `E` や `prefix_len` を別途採用条件へ足しても、現在の実装では `score` に対して冗長になる
考察:
- `safe_branch_full_length` が悪い主因は「採用条件の粗さ」ではなく、「safe branch 自体の質」が低い可能性が高い
- 次に効くのは、safe branch / SafeCollect の bite 候補評価や `bite -> zigzag` の改善であり、この TODO は完了扱いでよい
変更: SafeCollect の zigzag 回収で、forward/backward の 2 方向を単純な最短距離ではなく、`projected_prefix_len`, `prefix_len`, 次の target/fallback の作りやすさ、到達可能マス数、到達可能餌数で比較して選ぶようにした。zigzag そのものの安全性は保ったまま、回収後に再展開しやすい向きを優先する最小改善にした。
実行:
- `cargo check`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
- `Get-Content in/0003.txt | .\\target\\release\\ahc063.exe > out/0003.release.txt`
結果:
- コンパイル成功
- release 実行で `0000` の score JSON は `100`
- release 実行で `0003` の score JSON は `504676`
- stdout の提出形式と stderr の `result` JSON は維持された
考察:
- SafeCollect の回収向きに「回収後の再展開しやすさ」を持ち込む入口ができた
- 少なくとも `0003` では `540810 -> 504676` と改善しており、`safe_branch_full_length` 群に効く方向の改善として期待できる
変更: SafeCollect の bite 候補評価で、bite 後の状態だけでなく「その直後に 1 本の zigzag 回収をつないだ状態」まで見て比較するようにした。これにより、bite 単体ではなく `bite -> zigzag` のつながりが自然な候補を優先する。
実行:
- `cargo check`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
- `Get-Content in/0003.txt | .\\target\\release\\ahc063.exe > out/0003.release.txt`
結果:
- コンパイル成功
- release 実行で `0000` の score JSON は `100`
- release 実行で `0003` の score JSON は `456955`
- `0003` では `E = 45`, `prefix_len = 44` まで改善した
- stdout の提出形式と stderr の `result` JSON は維持された
考察:
- bite 候補の比較に「直後の zigzag 接続」を入れたことで、safe branch の `E` を下げる方向の改善が見えた
- 少なくとも `0003` では `504676 -> 456955` と改善しており、次は 200 ケース集計で `safe_branch_full_length` 群の平均 `E` がどこまで下がるかを見るのが自然
変更: safe branch を毎回の escape bite で起動せず、終盤または連続失敗寄りのときだけ起動するように条件を絞った。具体的には、残り餌数が少ないか、進展 stalled か、SafeCollect 進入回数が 2 回以上のときだけ launch する。
実行:
- `cargo check`
- `cargo build --release`
- `Get-Content in/0000.txt | .\\target\\release\\ahc063.exe > out/0000.release.txt`
- `Get-Content in/0003.txt | .\\target\\release\\ahc063.exe > out/0003.release.txt`
結果:
- コンパイル成功
- release 実行で `0000` の score JSON は `100`
- release 実行で `0003` の score JSON は `456955`
- `0000` は `stop_reason = "completed"`、`0003` は `stop_reason = "safe_branch_full_length"` で据え置きだった
- stdout の提出形式と stderr の `result` JSON は維持された
考察:
- 軽い 2 ケース確認では差分は出ていないが、SafeCollect 本体ではなく safe branch 起動条件だけを調整できた
- 効果の有無は 200 ケースで `safe_branch_full_length` 件数と completed の変化を見る必要がある
