#!/bin/bash

# 引数のチェック
if [ $# -ne 3 ]; then
    echo "使用方法: $0 <送信元IP> <送信先IP> <開始ポート-終了ポート>"
    echo "例: $0 192.168.0.1 192.168.0.2 80-100"
    exit 1
fi

# 引数を変数に格納
SOURCE_IP=$1
DESTINATION_IP=$2
PORT_RANGE=$3

# Rustのデフォルトツールチェーンを設定
echo "Rustのデフォルトツールチェーンを安定版に設定します..."
rustup default stable

# プロジェクトディレクトリに移動
echo "プロジェクトディレクトリに移動します..."
# shellcheck disable=SC2164
cd ~/RustroverProjects/port_scanner

# プロジェクトをビルド
echo "プロジェクトをビルドします..."
cargo build --release

# ビルドが成功した場合のみ、以下を実行
# shellcheck disable=SC2181
if [ $? -eq 0 ]; then
    # 実行ファイルに権限を付与
    echo "実行ファイルに権限を付与します..."
    sudo setcap cap_net_raw,cap_net_admin=eip target/release/port_scanner

    echo "ポートスキャナーを実行します..."
    # 引数を付けてアプリケーションを実行
    sudo ./target/release/port_scanner "$SOURCE_IP" "$DESTINATION_IP" "$PORT_RANGE"
else
    echo "ビルドに失敗しました。エラーを確認してください。"
fi