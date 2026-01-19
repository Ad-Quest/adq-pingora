# Contributing to ADQ Pingora

We welcome contributions to ADQ Pingora! This document provides guidelines for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and constructive in all interactions.

## How to Contribute

### Reporting Issues

Before creating an issue, please:

1. **Search existing issues** to avoid duplicates
2. **Use the issue templates** when available
3. **Provide detailed information** including:
   - ADQ Pingora version
   - Operating system and version
   - Configuration files (sanitized)
   - Steps to reproduce
   - Expected vs actual behavior
   - Relevant log output

### Suggesting Features

Feature requests are welcome! Please:

1. **Check existing feature requests** first
2. **Describe the use case** clearly
3. **Explain the expected behavior**
4. **Consider implementation complexity**
5. **Provide examples** if possible

### Contributing Code

#### Prerequisites

- **Rust 1.70+** with cargo
- **Git** for version control
- **Basic understanding** of HTTP proxies and load balancing
- **Familiarity with Pingora** framework (helpful but not required)

#### Development Setup

1. **Fork the repository**
   ```bash
   git clone https://github.com/YOUR_USERNAME/adquest-pingora-proxy.git
   cd adquest-pingora-proxy
   ```

2. **Install dependencies**
   ```bash
   # Install Rust if not already installed
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install development dependencies
   sudo apt install build-essential pkg-config libssl-dev
   ```

3. **Build the project**
   ```bash
   cargo build
   cargo test
   ```

4. **Run in development mode**
   ```bash
   RUST_LOG=debug cargo run -- -c config/proxy.yaml
   ```

#### Making Changes

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes**
   - Follow Rust coding conventions
   - Add tests for new functionality
   - Update documentation as needed
   - Ensure all tests pass

3. **Test your changes**
   ```bash
   # Run unit tests
   cargo test
   
   # Run integration tests
   cargo test --test integration_tests
   
   # Test configuration validation
   cargo run -- -t
   
   # Check code formatting
   cargo fmt --check
   
   # Run clippy for linting
   cargo clippy -- -D warnings
   ```

4. **Commit your changes**
   ```bash
   git add .
   git commit -m "feat: add new feature description"
   ```

   Use conventional commit messages:
   - `feat:` - New features
   - `fix:` - Bug fixes
   - `docs:` - Documentation changes
   - `style:` - Code style changes
   - `refactor:` - Code refactoring
   - `test:` - Test additions/changes
   - `chore:` - Maintenance tasks

5. **Push and create a pull request**
   ```bash
   git push origin feature/your-feature-name
   ```

#### Pull Request Guidelines

- **Provide a clear description** of the changes
- **Reference related issues** using `Fixes #123` or `Closes #123`
- **Include tests** for new functionality
- **Update documentation** if needed
- **Ensure CI passes** before requesting review
- **Keep PRs focused** - one feature/fix per PR
- **Be responsive** to review feedback

## Development Guidelines

### Code Style

- **Follow Rust conventions** using `rustfmt`
- **Use meaningful variable names**
- **Add comments for complex logic**
- **Keep functions focused and small**
- **Use error handling** with `Result<T, E>`

### Testing

- **Write unit tests** for new functions
- **Add integration tests** for new features
- **Test error conditions** and edge cases
- **Use descriptive test names**
- **Mock external dependencies** when appropriate

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_requests_under_limit() {
        // Test implementation
    }

    #[test]
    fn test_rate_limit_rejects_requests_over_limit() {
        // Test implementation
    }
}
```

### Documentation

- **Update README.md** for user-facing changes
- **Add/update docs/** for new features
- **Include code examples** in documentation
- **Use clear, concise language**
- **Test documentation examples**

### Configuration Changes

When adding new configuration options:

1. **Update configuration structs** in `src/config/`
2. **Add validation logic** if needed
3. **Update example configurations**
4. **Document the new options**
5. **Provide migration notes** if breaking changes

### Performance Considerations

- **Profile performance-critical code**
- **Avoid unnecessary allocations**
- **Use async/await properly**
- **Consider memory usage**
- **Benchmark significant changes**

## Project Structure

```
adq-pingora/
├── src/                    # Source code
│   ├── main.rs            # Main application entry
│   ├── proxy.rs           # Core proxy logic
│   ├── config/            # Configuration handling
│   ├── rate_limit/        # Rate limiting implementation
│   ├── cache/             # Caching functionality
│   ├── logging/           # Logging and monitoring
│   └── ...
├── docs/                  # Documentation
├── tests/                 # Integration tests
├── scripts/               # Installation and management scripts
├── config/                # Example configurations
└── sites-available/       # Example site configurations
```

## Release Process

Releases are managed by maintainers:

1. **Version bump** in `Cargo.toml`
2. **Update CHANGELOG.md**
3. **Create release tag**
4. **Build and test** release artifacts
5. **Publish release** on GitHub

## Getting Help

- **Documentation**: Check [docs/](docs/) first
- **Discussions**: Use [GitHub Discussions](https://github.com/Ad-Quest/adquest-pingora-proxy/discussions)
- **Issues**: Create an issue for bugs or feature requests
- **Chat**: Join our community chat (link TBD)

## Recognition

Contributors are recognized in:
- **CHANGELOG.md** for significant contributions
- **README.md** contributors section
- **Release notes** for major features

## License

By contributing to ADQ Pingora, you agree that your contributions will be licensed under the MIT License.

## Questions?

If you have questions about contributing, please:

1. Check this document first
2. Search existing issues and discussions
3. Create a new discussion or issue
4. Reach out to maintainers

Thank you for contributing to ADQ Pingora! 🚀