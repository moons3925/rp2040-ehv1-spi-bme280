# ラズピコでRust

BME280センサーから温度・湿度・気圧を計測してLCDに表示し、1分間隔でこれらの値をSDカードに保存します。

## setting.json

.vscode\setting.jsonの１行目と２行目は皆さんの環境に合わせてパスを設定してください。
  
１行目： openocd.exe へのパス  
２行目： arm-none-eabi-gdb.exe へのパス

## ブログ

詳しくは以下をご覧になってください。

[ラズピコでRust(17)　温度・湿度・気圧を計測して保存する](https://moons.link/pico/post-1487/)
