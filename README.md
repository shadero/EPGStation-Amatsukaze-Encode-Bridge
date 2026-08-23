# EPGStation–Amatsukaze Encode Bridge

## 概要

EPGStationに録画された番組を、別PCで動作しているAmatsukazeでエンコードさせるためのBridgeです。

以下のような流れで動作します。

1. EPGStationから対象の動画をダウンロード
2. Amatsukazeの指定プロファイルでエンコード
3. 完成動画をEPGStationにアップロード

BridgeはAmatsukazeサーバ上で動かす想定です。
また、Amatsukazeはrigaya氏のfork版で動作させる前提で作成しています。

## 使用方法

1. Releaseから本体をAmatsukazeサーバへダウンロード（以下、Bridge）
2. Bridgeのconfig.tomlを設定
3. Bridgeを起動（スタートアップ登録をしておくと便利です！）
4. `epgstation/amatsukaze-bridge-client.js`をEPGStationから参照できる場所へコピー
5. EPGStationの`config.yml`にエンコード設定を追加（各引数の説明は後で書く）

  ```yaml
  encode:
    - name: Amatsukaze NVEnc HEVC
      cmd: '%NODE% /app/config/amatsukaze-bridge-client.js http://192.168.0.16:8765 "NVEnc_HEVC" "Amatsukaze NVEnc HEVC"'
  ```

6. EPGStationの録画をAmatsukazeでエンコードするように設定
