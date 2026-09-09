# Contributing

Thanks for contributing to the Chess Engine.

## Development workflow

1. Create a focused branch from the default branch.
2. Make the smallest change that solves the problem.
3. Add or update tests in [`test.js`](./test.js).
4. Run the test suite:

   ```bash
   node test.js
   ```

5. Run the terminal application manually when changing user-facing behavior:

   ```bash
   node terminal/chess.js
   ```

6. Open a pull request using the repository template.

## Code expectations

- Preserve the existing board and move formats unless the change requires an API update.
- Keep piece-specific logic in the relevant `pieces/` module.
- Add regression tests for rule changes and edge cases.
- Do not add external dependencies without explaining why they are needed.
- Keep the terminal interface usable without a browser or network connection.

## Pull requests

Pull requests should describe the behavior changed, tests run, and any limitations or compatibility concerns.
