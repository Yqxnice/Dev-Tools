import pluginVue from 'eslint-plugin-vue'
import ts from 'typescript-eslint'
import js from '@eslint/js'

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...pluginVue.configs['flat/recommended'],
  {
    files: ['src/**/*.vue'],
    languageOptions: { parserOptions: { parser: ts.parser } },
  },
  {
    rules: {
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  },
  { ignores: ['dist/', 'node_modules/', 'src-tauri/'] },
)
