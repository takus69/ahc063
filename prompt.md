# AI運用プロンプトメモ

このプロジェクトでは **AGENTS.md を唯一のルールとして参照する。**
以下のプロンプトは、各役割チャットで使用する。

---

# 1 Strategistチャット初期プロンプト

```
あなたはこのプロジェクトの Strategist（考察担当）です。

作業ルールと役割は AGENTS.md に従ってください。

まず AGENTS.md を読み、次を理解してください。

- Strategist の役割
- Source of Truth
- Workflow
- Rules

特に以下を守ってください。

- problem.md には問題の「事実のみ」を書く
- 仮説や解法は hypotheses.md に書く
- 実装は Implementer の役割

AGENTS.md を読み終えたら「OK」と答えてください。
```

---

# 2 problem.md 作成プロンプト

```
AGENTS.md に従って作業してください。

生の問題文は refs/problem.html です。
docs/problem.md の「たたき台」を作成してください。

重要:

- problem.md には事実のみを書く
- 仮説や解法は書かない
- 推測は書かない
- 問題文の内容だけを書く

出力は docs/problem.md の内容として
そのまま使える形式で書いてください。

```

---

# 3 problem.md レビュープロンプト

```
AGENTS.md に従って作業してください。

作成した problem.md をレビューしてください。

確認する点:

- 問題文の事実と矛盾がないか
- 制約が漏れていないか
- スコア構造が正しく整理されているか
- 推測や仮説が混ざっていないか

問題があれば修正してください。
```

---

# 4 hypotheses.md 生成プロンプト

```
AGENTS.md に従って作業してください。

problem.md を基に
docs/hypotheses.md の初期案を作成してください。

内容:

- 問題構造の観察
- 有望そうなアルゴリズム
- 探索方法の候補
- neighborhood候補
- スコア改善の方向性

注意:

- これは仮説なので推測を書いてよい
- ただし problem.md の事実と矛盾してはいけない
```

---

# 5 strategy.md 生成プロンプト

```
problem.md, hypotheses.md を基に
docs/strategy.md の初期案を作成してください。
```

---

# 6 改善案生成プロンプト

```
AGENTS.md に従って作業してください。

次のファイルを読んでください。

- docs/problem.md
- docs/strategy.md
- docs/log.md
- docs/scoreboard.md

次に試すべき改善案を提案してください。

各案について:

- 期待スコア改善
- 実装コスト
- リスク
- 優先順位

を簡潔に書いてください。

TODO候補として提示してください。
```

---

# 7 Implementer用 実装プロンプト

```
AGENTS.md に従って作業してください。

次のファイルを読んでください。

- docs/problem.md
- docs/strategy.md
- docs/todo.md

todo.md の未完了の先頭タスクを実装してください。

ルール:

- 最小差分で実装
- 制約違反を起こさない
- TODO以外の機能を実装しない

作業の流れ:

1 実装方針を説明
2 実装
3 実験
4 docs/log.md に結果を書く
5 変更の影響をまとめる
6 コミットメッセージ案とコミット対象候補を提示
```

---

# 8 Reviewer用 レビュープロンプト

```
AGENTS.md に従って作業してください。

次の変更をレビューしてください。

確認する点:

- バグの可能性
- 制約違反の可能性
- 実行時間悪化
- 不要な複雑化

危険な点があれば指摘してください。
```

---

# 9 コミット案生成プロンプト

```
AGENTS.md に従って作業してください。

今回の変更について

- 変更内容の要約
- コミットメッセージ案
- コミット対象ファイル

を提示してください。

注意:

- コミットは人間が実行する
- 確定した変更のみ対象にする
```

---

# 10 Visualizer用 プロンプト

```
あなたはこのプロジェクトの Visualizer（ビジュアライザ担当）です。

まず AGENTS.md を読み、次を理解してください。

- Visualizer の役割

以下のファイルを参照して、理解してください。

- docs/problem.md
- src/main.rs
- tools/*

以下の点に注意してください。

- 修正はビジュアライザ(tools配下)のみで、実行コード(src/main.rs)は修正しないください。
- ビジュアライザに必要な実行コード側の結果出力は、修正案を出してください。

```

---

# 推奨運用フロー

コンテスト開始直後

1 Strategistチャット作成
2 初期プロンプト送信
3 問題文投入
4 problem.md 作成
5 problem.md レビュー
6 hypotheses.md 作成
7 strategy.md 初期化
8 todo.md 作成

その後のサイクル

1 Implementer 実装
2 実験
3 log.md 更新
4 scoreboard.md 更新
5 Strategist 改善案
6 todo.md 更新
7 コミット
