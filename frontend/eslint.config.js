import tseslint from "typescript-eslint"
import vueEslint from "eslint-plugin-vue"
import vueParser from "vue-eslint-parser"

export default tseslint.config(
  // TypeScript/JavaScript 基础配置
  {
    extends: [tseslint.configs.recommended],
    rules: {
      // 降级规则为警告或关闭
      "@typescript-eslint/no-unused-vars": ["warn", { "argsIgnorePattern": "^_", "varsIgnorePattern": "^_", "caughtErrorsIgnorePattern": "^_" }],
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-this-alias": "off",
      "@typescript-eslint/explicit-function-return-type": "off",
      "@typescript-eslint/explicit-module-boundary-types": "off",
      "@typescript-eslint/no-unused-expressions": "off", // 关闭，因为 Vue 模板中常见
      "no-console": process.env.NODE_ENV === 'production' ? 'warn' : 'off',
      "no-debugger": process.env.NODE_ENV === 'production' ? 'warn' : 'off',
      "comma-dangle": "off" // 与 Prettier 冲突
    }
  },
  // Vue 文件配置
  {
    files: ["**/*.vue"],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tseslint.parser
      }
    },
    plugins: {
      vue: vueEslint
    },
    rules: {
      "vue/multi-word-component-names": "off",
      // 覆盖 TypeScript 规则为警告
      "@typescript-eslint/no-unused-vars": ["warn", { "argsIgnorePattern": "^_", "varsIgnorePattern": "^_", "caughtErrorsIgnorePattern": "^_" }],
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-expressions": "off"
    }
  }
)
