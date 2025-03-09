const buildWasm = async () => {
  // Build the Rust/WASM package
  const wasmBuild = new Deno.Command("wasm-pack", {
    args: ["build", "--target", "web"],
  });
  const { success } = await wasmBuild.output();
  if (!success) {
    throw new Error("WASM build failed");
  }

  // Ensure www/pkg directory exists
  try {
    await Deno.mkdir("www/pkg", { recursive: true });
  } catch (err) {
    if (!(err instanceof Deno.errors.AlreadyExists)) {
      throw err;
    }
  }

  // Copy WASM files to www directory
  const files = [
    ["pkg/c_ide_bg.wasm", "www/pkg/c_ide_bg.wasm"],
    ["pkg/c_ide.js", "www/pkg/c_ide.js"],
  ];

  for (const [src, dest] of files) {
    await Deno.copyFile(src, dest);
  }

  console.log("Build completed successfully");
};

// Run build
if (import.meta.main) {
  try {
    await buildWasm();
  } catch (err) {
    console.error("Build failed:", err);
    Deno.exit(1);
  }
} 