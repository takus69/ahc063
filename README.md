# AHC063

## command

- tester実行

```
cargo build --release
cat .\in\0000.txt | .\tester .\target\release\ahc063.exe > .\out\0000.txt
```

- 1テストケース実行

```
cargo build
cat .\in\0000.txt | .\target\debug\ahc063.exe > .\out\0000.txt
```

- 一括実行

```
cargo build
python .\simulator.py
```
