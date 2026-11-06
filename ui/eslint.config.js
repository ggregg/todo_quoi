// Configuration ESLint (format « flat », ESLint ≥ 9) — volontairement minimale :
// règles recommandées JS + TypeScript + Vue 3, puis désactivation de tout ce qui relève de la
// mise en forme (laissée à Prettier). Les types sont vérifiés à part par `vue-tsc`.
import js from '@eslint/js'
import tseslint from 'typescript-eslint'
import pluginVue from 'eslint-plugin-vue'
import prettier from 'eslint-config-prettier'

export default tseslint.config(
  { ignores: ['dist/', 'node_modules/', 'coverage/'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs['flat/recommended'],
  {
    files: ['**/*.vue'],
    languageOptions: { parserOptions: { parser: tseslint.parser } },
  },
  {
    files: ['**/*.{ts,vue,js}'],
    rules: {
      // Variables non déclarées : déjà vérifiées par `vue-tsc` (recommandation typescript-eslint),
      // ce qui évite de lister les globales du navigateur et du Worker à la main.
      'no-undef': 'off',
      // Paramètres volontairement ignorés : préfixe `_` (ex. `_pos` dans un plugin ProseMirror).
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },
  prettier,
)
