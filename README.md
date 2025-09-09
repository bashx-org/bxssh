# bxssh

A WebAssembly-compatible SSH client CLI built in Rust.

## Features

- SSH key-based authentication
- Interactive shell sessions
- Remote command execution
- WebAssembly-ready architecture with conditional compilation
- Automatic SSH key discovery (`~/.ssh/id_rsa`, `~/.ssh/id_ed25519`)

## Usage

### Basic connection with SSH key (preferred format)
```bash
bxssh user@hostname
```

### Alternative format (backward compatibility)
```bash
bxssh -u username hostname
```

### Connection with specific port and key
```bash
bxssh -p 2222 -i ~/.ssh/my_key user@hostname
```

### Execute a single command
```bash
bxssh -c "ls -la" user@hostname
```

### Use password authentication
```bash
bxssh --password user@hostname
```

### Interactive shell session
```bash
bxssh user@hostname
# Use Ctrl+C to exit
```

## Installation

```bash
cargo build --release
```

## Development

### Prerequisites

- Rust 1.70+ with Cargo
- Git for version control

### Setup for New Contributors

1. **Clone and Setup**:
   ```bash
   git clone <repository-url>
   cd bxssh
   cargo check  # Verify setup
   ```

2. **Install Development Tools**:
   ```bash
   # Test coverage tool
   cargo install cargo-tarpaulin

   # WebAssembly toolchain (future use)
   rustup target add wasm32-unknown-unknown
   cargo install wasm-pack
   ```

### Test-Driven Development (TDD) Workflow

**⚠️ CRITICAL: All components MUST follow strict TDD practices.**

#### TDD Development Process

1. **Write Tests First** (Red Phase):
   ```bash
   # Create failing tests for new functionality
   cargo test  # Should fail with new tests
   ```

2. **Implement Minimum Code** (Green Phase):
   ```bash
   # Write just enough code to make tests pass
   cargo test  # Should pass
   ```

3. **Refactor** (Refactor Phase):
   ```bash
   # Clean up code while keeping tests green
   cargo test  # Must stay green
   cargo check  # Verify no warnings
   ```

#### Daily Development Commands

```bash
# Run all tests (should be done frequently)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration

# Check code without building
cargo check

# Generate test coverage report
cargo tarpaulin --out Html

# Build for release
cargo build --release

# Run with logging
RUST_LOG=debug cargo run -- --help
```

### Testing Requirements

#### Mandatory Test Coverage
- **100% coverage** of all business logic
- **Unit tests** for every public function/method
- **Integration tests** for CLI interactions
- **Error case testing** alongside success cases
- **Edge case validation** (empty inputs, boundaries, invalid data)

#### Test Structure
```bash
src/
├── module.rs           # Implementation
│   └── #[cfg(test)]    # Unit tests in same file
└── tests/
    └── integration.rs  # Integration tests separate
```

#### Mock Requirements
- **All external dependencies** must be mocked
- Use `mockall` crate for trait-based mocking
- **Network calls**, **file system access**, **SSH connections** must be mocked
- Tests should run without external dependencies

### Code Quality Standards

#### Before Every Commit
```bash
# 1. Ensure all tests pass
cargo test

# 2. Check for warnings
cargo check

# 3. Verify no compilation errors
cargo build

# 4. Run integration tests
cargo test --test integration
```

#### Code Review Checklist
- [ ] Tests written **before** implementation
- [ ] All tests passing
- [ ] New functionality has corresponding tests
- [ ] Error cases are tested
- [ ] No unwrap() calls without justification
- [ ] Proper error handling with `anyhow::Result`
- [ ] Documentation for public APIs
- [ ] WebAssembly compatibility maintained

### Architecture Guidelines

#### Trait-Based Design
- Use **traits for all external dependencies**
- Enable **dependency injection** for testing
- Follow **SOLID principles**

#### WebAssembly Compatibility
- Use `#[cfg(target_arch = "wasm32")]` for conditional compilation
- Avoid platform-specific dependencies in shared code
- Test both native and WASM builds

#### Error Handling
- Use `anyhow::Result` for all fallible operations
- Provide **meaningful error messages**
- Test error scenarios extensively

### Contribution Workflow

1. **Create Feature Branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Follow TDD Process**:
   - Write failing tests
   - Implement minimal code
   - Refactor while keeping tests green

3. **Verify Quality**:
   ```bash
   cargo test           # All tests pass
   cargo tarpaulin      # Check coverage
   cargo check          # No warnings
   ```

4. **Commit with Tests**:
   ```bash
   git add .
   git commit -m "feat: implement feature X with comprehensive tests"
   ```

5. **Push and Create PR**:
   ```bash
   git push origin feature/your-feature-name
   # Create pull request with test results
   ```

### Performance Testing

```bash
# Profile the application
cargo build --release
time ./target/release/bxssh --help

# Memory usage analysis
valgrind ./target/release/bxssh --help
```

### Debugging

```bash
# Debug build with symbols
cargo build
gdb ./target/debug/bxssh

# With logging
RUST_LOG=debug cargo run -- args...

# With backtrace
RUST_BACKTRACE=1 cargo run -- args...
```

## WebAssembly Support

The project is designed to be compiled for WebAssembly:

```bash
# Build for WebAssembly (future implementation)
wasm-pack build --target web --out-dir pkg

# Test WASM build
cargo build --target wasm32-unknown-unknown
```

## Architecture

- **Native**: Uses `ssh2` crate with system SSH libraries
- **WebAssembly**: Placeholder implementation for browser-based SSH (with tests)
- **Cross-platform**: Built with `crossterm` for terminal handling
- **Testing**: Comprehensive test suite with mocked dependencies
- **Trait-based**: Clean abstractions for dependency injection
