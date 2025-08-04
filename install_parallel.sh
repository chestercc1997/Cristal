#!/bin/bash

# 设置安装目录（用户目录下的 .local/bin）
INSTALL_DIR="$HOME/.local/bin"
PARALLEL_URL="http://ftp.gnu.org/gnu/parallel/parallel-latest.tar.bz2"

# 如果安装目录不存在，则创建
mkdir -p "$INSTALL_DIR"

# 下载 GNU Parallel 的最新版本
echo "Downloading GNU Parallel..."
wget -q "$PARALLEL_URL" -O parallel-latest.tar.bz2

# 解压下载的文件
echo "Extracting GNU Parallel..."
tar -xvf parallel-latest.tar.bz2 > /dev/null

# 进入解压后的目录
cd parallel-20250622 || { echo "Failed to enter extracted directory!"; exit 1; }

# 配置并安装到用户目录
echo "Configuring and installing GNU Parallel to $INSTALL_DIR..."
./configure --prefix="$HOME/.local" > /dev/null
make > /dev/null
make install > /dev/null

# 清理临时文件
cd ..
rm -rf parallel-20250622 parallel-latest.tar.bz2

# 自动更新 PATH 环境变量
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$HOME/.bashrc"
    echo "Added $INSTALL_DIR to PATH in ~/.bashrc"
    # 自动刷新环境变量
    export PATH="$INSTALL_DIR:$PATH"
    echo "Updated PATH for current session."
fi

# 验证安装
echo "Verifying installation..."
"$INSTALL_DIR/parallel" --version