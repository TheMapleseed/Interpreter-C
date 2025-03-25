# Interpreter-C

This codebase represents a sophisticated, enterprise-grade compiler and IDE infrastructure for C/C++ development. The architecture follows industry best practices with proper separation of concerns, thorough error handling, and comprehensive testing infrastructure. The modular design allows for extension and customization, making it suitable for integration into larger systems or deployment as a standalone tool.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Command Line Interface](#command-line-interface)
- [Architecture Support](#architecture-support)
- [Execution Modes](#execution-modes)
- [Advanced Features](#advanced-features)
- [Performance Optimization](#performance-optimization)
- [Troubleshooting](#troubleshooting)
- [Building from Source](#building-from-source)
- [Contributing](#contributing)

## Installation


## Quick Start

1. Create a simple C file:

```c
// hello.c
#include <stdio.h>

int main() {
    printf("Hello, Interpreter-C!\n");
    return 0;
}
```

2. Run it with the interpreter:

```bash
c-interpreter hello.c
```

## Command Line Interface

### Basic Usage

```bash
c-interpreter [OPTIONS] [FILE]
```

If no file is provided, the interpreter reads from standard input.

### Common Options

| Option | Description |
|--------|-------------|
| `-j, --jit` | Use JIT compilation (default mode) |
| `-i, --interpret` | Use interpretation only (no JIT) |
| `-c, --compile` | Compile to object file instead of executing |
| `-o, --output <FILE>` | Output file (for compiled mode) |
| `-O, --opt <LEVEL>` | Optimization level (0-3), default is 2 |
| `-a, --arch <ARCH>` | Target architecture |
| `-I, --include <DIR>` | Add directory to include search path |
| `-v, --verbose` | Enable verbose output |
| `--help` | Show help information |
| `--version` | Show version information |

## Architecture Support

Interpreter-C supports multiple target architectures:

### CPU Architectures

```bash
# Compile for x86_64
c-interpreter -c -a x86_64 myprogram.c

# Compile for ARM64/Apple Silicon
c-interpreter -c -a aarch64 myprogram.c

# Compile for 32-bit ARM
c-interpreter -c -a arm myprogram.c
```

### GPU Architectures

```bash
# Compile for AMD GPUs
c-interpreter -c -a amdgpu myprogram.c

# Compile for NVIDIA GPUs
c-interpreter -c -a nvptx myprogram.c
```

## Execution Modes

### JIT Compilation (Default)

JIT compilation provides the best performance for most use cases by compiling code at runtime:

```bash
# Explicit JIT mode
c-interpreter -j myprogram.c

# Default mode (JIT)
c-interpreter myprogram.c
```

### Interpretation Mode

Interpretation mode executes code without compilation, useful for debugging or educational purposes:

```bash
c-interpreter -i myprogram.c
```

### Compilation Mode

Compilation mode generates executable files:

```bash
# Compile with default output name (a.out)
c-interpreter -c myprogram.c

# Compile with custom output name
c-interpreter -c -o myprog myprogram.c

# Compile with optimization level 3
c-interpreter -c -O3 -o myprog myprogram.c
```

## Advanced Features

### Executing Code from stdin

```bash
echo 'int main() { return 42; }' | c-interpreter
```

### Using Include Paths

```bash
c-interpreter -I /path/to/includes -I /another/path program.c
```

### Cross-Compilation

```bash
# Cross-compile x86_64 program on ARM machine
c-interpreter -c -a x86_64 -o program.x86_64 program.c

# Cross-compile ARM program on x86_64 machine
c-interpreter -c -a arm -o program.arm program.c
```

## Performance Optimization

### Optimization Levels

- `-O0`: No optimization (fastest compilation, slowest execution)
- `-O1`: Basic optimizations
- `-O2`: Default optimizations (recommended)
- `-O3`: Aggressive optimizations (slowest compilation, fastest execution)

### Architecture-Specific Optimization

Specify the target architecture to enable architecture-specific optimizations:

```bash
# Optimize for Apple Silicon
c-interpreter -a aarch64 -O3 program.c
```

## Troubleshooting

### Common Issues

1. **Compilation Errors**: Use verbose mode to see more details
   ```bash
   c-interpreter -v program.c
   ```

2. **Architecture Mismatch**: Ensure proper architecture is specified for cross-compilation
   ```bash
   c-interpreter -v -a aarch64 program.c
   ```

3. **Missing Libraries**: Include necessary paths
   ```bash
   c-interpreter -I /path/to/libs program.c
   ```

### Error Messages

| Error | Solution |
|-------|----------|
| "Failed to initialize compiler" | Check installation and system requirements |
| "Parse error" | Check C syntax in source file |
| "Unsupported architecture" | Use one of the supported architectures |
| "Runtime error" | Debug your C code logic |

## Building from Source

### Prerequisites

- LLVM 17.0+
- Rust 1.70+
- CMake 3.20+
- C++17 compatible compiler

### Build Steps

```bash
# Clone the repository
git clone https://github.com/your-org/interpreter-c.git
cd interpreter-c

# Build using Cargo
cargo build --release

# The binary will be in target/release/c-interpreter
```

### Running Tests

```bash
cargo test
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to contribute to this project.

### Development Environment Setup

```bash
# Setup development environment
./scripts/setup_dev.sh

# Build in debug mode
cargo build

# Run with debug logging
RUST_LOG=debug ./target/debug/c-interpreter program.c
```

## License

This project is licensed under [LICENSE](LICENSE) - see the file for details.
  