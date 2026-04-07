# AHC063

## command

- 1テストケース実行

```
cargo build
cat .\in\0000.txt | .\target\debug\ahc063.exe > .\out\0000.txt
```

- 一括実行

```
cargo build --release
python .\simulator.py
```
