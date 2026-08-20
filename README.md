# EPGStation–Amatsukaze Encode Bridge

## 概要

EPGStationの録画をAmatsukazeサーバへ取得し、エンコード後の動画をEPGStationへ戻すBridgeです。

1. EPGStationから元動画をダウンロード
2. Amatsukazeの指定プロファイルでエンコード
3. 完成動画を元の録画へアップロード

BridgeはAmatsukazeサーバ上で動かす想定です。
また、Amatsukazeはrigaya氏のforkで動作させる前提で作成しています。

## 使用方法

1. Releaseから本体をAmatukazeサーバが動作しているPCにダウンロード（以下、Bridge）
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
