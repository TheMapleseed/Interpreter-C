use clap::{Arg, ArgAction, Command};
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process;

// Import our interpreter components
mod abi;
mod analysis;
mod arch;
mod build;
mod compiler;
mod cpu;
mod debug;
mod diagnostics;
mod docs;
mod driver;
mod frontend;
mod gui;
mod ide;
mod interpreter;
mod jit;
mod kernel;
mod linker;
mod lto;
mod memory;
mod metrics;
mod monitoring;
mod optimizer;
mod orchestrator;
mod pgo;
mod pipeline;
mod project;
mod runtime;
mod stdlib;
mod syscall;
mod testing;
mod types;

use compiler::CompilerOptions;
use jit::JITOptions;
use interpreter::c_runtime::CRuntimeEnvironment;
use frontend::c23::C23Parser;

/// The main entry point for the Interpreter-C CLI
fn main() -> io::Result<()> {
    let matches = Command::new("c-interpreter")
        .version("0.1.0")
        .author("Interpreter-C Team")
        .about("A high-performance C interpreter with JIT compilation")
        .arg(
            Arg::new("file")
                .help("The C source file to interpret")
                .index(1),
        )
        .arg(
            Arg::new("jit")
                .long("jit")
                .short('j')
                .help("Use JIT compilation (default)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("interpret")
                .long("interpret")
                .short('i')
                .help("Use interpretation only (no JIT)")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("optimization")
                .long("opt")
                .short('O')
                .help("Optimization level (0-3)")
                .default_value("2"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Output file (for compiled mode)"),
        )
        .arg(
            Arg::new("compile")
                .long("compile")
                .short('c')
                .help("Compile to object file instead of executing")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("architecture")
                .long("arch")
                .short('a')
                .help("Target architecture (x86_64, aarch64, arm, amdgpu, nvptx)")
                .value_parser(["x86_64", "aarch64", "arm", "amdgpu", "nvptx"])
                .default_value(std::env::consts::ARCH),
        )
        .arg(
            Arg::new("include")
                .long("include")
                .short('I')
                .help("Add directory to include search path")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Verbose output")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    // Get source code
    let source_code = if let Some(filename) = matches.get_one::<String>("file") {
        fs::read_to_string(filename)?
    } else {
        // Read from stdin if no file is specified
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer
    };

    // Parse optimization level
    let opt_level = matches
        .get_one::<String>("optimization")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(2);

    // Parse target architecture
    let architecture = matches
        .get_one::<String>("architecture")
        .unwrap_or(&String::from(std::env::consts::ARCH))
        .clone();

    // Validate architecture selection
    let arch_valid = match architecture.as_str() {
        "x86_64" | "aarch64" | "arm" | "amdgpu" | "nvptx" => true,
        _ => false
    };

    if !arch_valid {
        eprintln!("Error: Unsupported architecture '{}'. Supported: x86_64, aarch64, arm, amdgpu, nvptx", architecture);
        process::exit(1);
    }

    // If verbose, print configuration
    if matches.get_flag("verbose") {
        println!("Source length: {} characters", source_code.len());
        println!("Optimization level: {}", opt_level);
        println!("Target architecture: {}", architecture);
        println!("Mode: {}", if matches.get_flag("interpret") {
            "Interpret"
        } else if matches.get_flag("compile") {
            "Compile"
        } else {
            "JIT"
        });
    }

    // Execute or compile based on options
    if matches.get_flag("compile") {
        compile_code(&source_code, matches.get_one::<String>("output"), opt_level, &architecture)?;
    } else if matches.get_flag("interpret") {
        interpret_code(&source_code)?;
    } else {
        // Default: JIT execution
        jit_execute(&source_code, opt_level, &architecture)?;
    }

    Ok(())
}

/// Get LLVM target triple for the specified architecture
fn get_target_triple(architecture: &str) -> &'static str {
    match architecture {
        "x86_64" => "x86_64-unknown-linux-gnu",
        "aarch64" => "aarch64-unknown-linux-gnu",
        "arm" => "arm-unknown-linux-gnueabihf",
        "amdgpu" => "amdgcn-amd-amdhsa",
        "nvptx" => "nvptx64-nvidia-cuda",
        _ => "x86_64-unknown-linux-gnu", // Default fallback
    }
}

/// Compile C code to an object file
fn compile_code(source: &str, output_file: Option<&String>, opt_level: u32, architecture: &str) -> io::Result<()> {
    if let Some(output) = output_file {
        println!("Compiling to {}", output);
    } else {
        println!("Compiling to a.out");
    }

    // Create compiler instance
    let compiler = unsafe {
        match compiler::Compiler::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to initialize compiler: {:?}", e);
                process::exit(1);
            }
        }
    };

    // Set up compiler options
    let output_path = output_file.map(|s| s.as_str()).unwrap_or("a.out");
    let options = CompilerOptions {
        optimization_level: opt_level,
        link: true,
        link_options: compiler::LinkOptions {
            libraries: vec![],
            library_paths: vec![],
        },
        debug_info: true,
        target_features: vec![],
        target_architecture: arch::Architecture::from_str(architecture).ok(),
        target_triple: Some(get_target_triple(architecture).to_string()),
    };

    // Compile the code
    unsafe {
        if let Err(e) = compiler.compile_string(source, output_path, &options) {
            eprintln!("Compilation error: {:?}", e);
            process::exit(1);
        }
    }

    println!("Compilation successful");
    Ok(())
}

/// Interpret C code without JIT compilation
fn interpret_code(source: &str) -> io::Result<()> {
    println!("Interpreting code...");

    // Create a parser
    let mut parser = C23Parser::new();
    
    // Parse the source
    let ast = match parser.parse(source) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parse error: {:?}", e);
            process::exit(1);
        }
    };

    // Create runtime environment
    let mut runtime = match CRuntimeEnvironment::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize runtime: {:?}", e);
            process::exit(1);
        }
    };

    // Execute the code
    match runtime.execute(&ast) {
        Ok(result) => {
            println!("Program executed successfully");
            println!("Return value: {}", result.return_value);
            Ok(())
        }
        Err(e) => {
            eprintln!("Runtime error: {:?}", e);
            process::exit(1);
        }
    }
}

/// JIT compile and execute C code
fn jit_execute(source: &str, opt_level: u32, architecture: &str) -> io::Result<()> {
    println!("JIT compiling and executing code...");

    // Create compiler instance
    let compiler = unsafe {
        match compiler::Compiler::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to initialize compiler: {:?}", e);
                process::exit(1);
            }
        }
    };

    // Set up JIT options
    let jit_options = JITOptions {
        optimization_level: opt_level,
        enable_fast_isel: true,
        enable_guard_pages: true,
        stack_size: 8 * 1024 * 1024, // 8MB stack
        target_architecture: arch::Architecture::from_str(architecture).ok(),
        target_triple: Some(get_target_triple(architecture).to_string()),
    };

    // JIT compile and execute
    unsafe {
        match compiler.jit_compile(source, &jit_options) {
            Ok(func_ptr) => {
                // Cast function pointer to the appropriate type (main function)
                let main_fn: extern "C" fn(i32, *const *const i8) -> i32 = 
                    std::mem::transmute(func_ptr);
                
                // Prepare argc and argv
                let args: Vec<*const i8> = vec![std::ptr::null()];
                
                // Call the function
                let result = main_fn(0, args.as_ptr());
                println!("Program executed successfully");
                println!("Return value: {}", result);
                Ok(())
            }
            Err(e) => {
                eprintln!("JIT compilation error: {:?}", e);
                process::exit(1);
            }
        }
    }
} 
