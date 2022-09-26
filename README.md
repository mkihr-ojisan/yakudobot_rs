# yakudobot

## これは何?
- ハッシュタグ「#mis1yakudo」がついてるツイートを自動的にリツイートしたり、yakudo写真を評価したりするbotです。
- [yakudobot](https://github.com/mkihr-ojisan/yakudobot)をRustに移植したものです。~~Rustが使いたかっただけ~~

## 使い方
### 環境構築
ビルドに必要なライブラリなどをインストールします。
```console
$ sudo apt install libopencv-dev clang  # Debian系の場合
```

`.env`ファイルを作成します。
```console
$ cp .env.template .env
$ vim .env
```

### アプリの起動
```console
$ docker compose up -d
```
