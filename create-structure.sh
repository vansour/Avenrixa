#!/bin/bash

# 分层架构重构脚本
# 此脚本帮助快速创建重构所需的目录结构

set -e  # 遇免任何错误导致中断

echo "========================================"
echo "   创建分层架构目录结构"
echo "========================================"

# 创建目录
echo "创建目录..."
mkdir -p src/config
mkdir -p src/domain/{user,auth,image,admin}
mkdir -p src/infrastructure/{database,cache,storage,email}
mkdir -p src/api/{routes,handlers,middleware}
mkdir -p src/shared

echo "✓ 目录创建完成"

# 创建模块文件
echo "创建 mod.rs 文件..."
touch src/config/mod.rs
touch src/domain/mod.rs
touch src/infrastructure/mod.rs
touch src/api/mod.rs
touch src/shared/mod.rs

echo "✓ mod.rs 文件创建完成"

# 创建子模块
echo "创建子模块..."
touch src/domain/user/{mod.rs,service.rs,repository.rs,models.rs}
touch src/domain/auth/{mod.rs,service.rs,repository.rs,models.rs}
touch src/domain/image/{mod.rs,service.rs,repository.rs,models.rs,processor.rs}
touch src/domain/admin/{mod.rs,service.rs,repository.rs,models.rs}

touch src/infrastructure/database/{mod.rs,pool.rs,schema.rs}
touch src/infrastructure/cache/{mod.rs,redis.rs}
touch src/infrastructure/storage/{mod.rs,file_queue.rs,image_processor.rs}
touch src/infrastructure/email/{mod.rs,smtp.rs}

touch src/api/routes/{mod.rs,auth.rs,images.rs,admin.rs,health.rs}
touch src/api/handlers/{mod.rs,auth.rs,images.rs,images_cursor.rs,admin.rs}
touch src/api/middleware/mod.rs

echo "✓ 子模块文件创建完成"

echo ""
echo "========================================"
echo "   目录结构创建完成！"
echo "========================================"
echo ""
echo "下一步："
echo "1. 按照 REFACTORING.md 中的说明，逐步重构代码"
echo "2. 建议从 domain/auth 开始，因为认证是核心功能"
echo "3. 每次重构一个模块，确保能编译通过"
echo ""
echo "重构顺序建议："
echo "   1. domain/auth"
echo "  2. domain/image"
echo "  echo  3. infrastructure"
echo "   4. api/handlers"
echo ""
echo "运行 './create-structure.sh' 可重新创建目录结构"
echo ""
