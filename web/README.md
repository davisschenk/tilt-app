# React + TypeScript + Vite

This template provides a minimal setup to get React working in Vite with HMR and some ESLint rules.

## TypeScript toolchain (two compilers, on purpose)

`package.json` installs two TypeScript versions side by side:

- **`@typescript/native`** (aliased to `typescript@7.0.2`, the native/Go compiler, pinned exactly and
  forced via `overrides`) is the **type-check authority**. `npm run build` invokes it explicitly
  (`node node_modules/@typescript/native/bin/tsc -b`) so it never depends on which package wins
  `node_modules/.bin/tsc`. If `npm run build` passes, the code type-checks.
- **`typescript`** (`~6.0.3`) exists **only for typescript-eslint**, which needs the JavaScript
  compiler API that TS 7.0 doesn't ship (typescript-eslint 8.x peers on `typescript <6.1`).
  Don't use its `tsc` for checking.

The `@typescript/native` name matches the optional peer typescript-eslint declares for its
experimental TS 7.1 backend. Without the pin, npm would install a nightly 7.1 build to fill that
peer. The `overrides` entry keeps it on the same pinned stable release. The experimental backend
is opt-in (`projectService.EXPERIMENTAL_backend: 'tsgo'`) and isn't enabled, so ESLint parses with
TS 6.

Both TypeScript packages are excluded from Dependabot (`.github/dependabot.yml`). Upgrade them by
hand, together. Once typescript-eslint supports the TS 7 API, collapse to a single
`"typescript": "^7.x"`, drop the alias, the override and the explicit build path, and go back to
plain `tsc -b`.

Currently, two official plugins are available:

- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react) uses [Babel](https://babeljs.io/) (or [oxc](https://oxc.rs) when used in [rolldown-vite](https://vite.dev/guide/rolldown)) for Fast Refresh
- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react-swc) uses [SWC](https://swc.rs/) for Fast Refresh

## React Compiler

The React Compiler is not enabled on this template because of its impact on dev & build performances. To add it, see [this documentation](https://react.dev/learn/react-compiler/installation).

## Expanding the ESLint configuration

If you are developing a production application, we recommend updating the configuration to enable type-aware lint rules:

```js
export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...

      // Remove tseslint.configs.recommended and replace with this
      tseslint.configs.recommendedTypeChecked,
      // Alternatively, use this for stricter rules
      tseslint.configs.strictTypeChecked,
      // Optionally, add this for stylistic rules
      tseslint.configs.stylisticTypeChecked,

      // Other configs...
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```

You can also install [eslint-plugin-react-x](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-x) and [eslint-plugin-react-dom](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-dom) for React-specific lint rules:

```js
// eslint.config.js
import reactX from 'eslint-plugin-react-x'
import reactDom from 'eslint-plugin-react-dom'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...
      // Enable lint rules for React
      reactX.configs['recommended-typescript'],
      // Enable lint rules for React DOM
      reactDom.configs.recommended,
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```
