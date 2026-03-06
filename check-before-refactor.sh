#!/bin/bash

# 检查重构前的代码健康状况

echo "========================================"
echo "   检查代码健康状况"
echo "========================================"
echo ""

# 检查文件大小
echo "文件统计："
find src -name "*.rs" -exec wc -l {} +;
echo ""

# 检查模块依赖
echo ""
echo "模块分析："
echo "handlers/"
wc -l handlers/*.rs
echo ""
echo "模块复杂度："
echo "handlers/"
cloc handlers/auth.rs handlers/images.rs handlers/admin.rs 2>/dev/null || echo "cloc 未安装"
echo ""

# 检查测试覆盖率
echo "测试文件："
find . -name "*_test.rs" -type f 2>/dev/null || echo "没有测试文件"
echo ""

echo "========================================"
echo "   检查完成"
echo "========================================"
