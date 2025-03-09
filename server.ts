import { serve } from "https://deno.land/std@0.208.0/http/server.ts";
import { serveDir } from "https://deno.land/std@0.208.0/http/file_server.ts";

// WASM MIME type configuration
const mimeTypes = {
  ".wasm": "application/wasm",
  ".js": "application/javascript",
};

serve(async (req) => {
  const url = new URL(req.url);
  
  // Handle WASM files specially
  if (url.pathname.endsWith('.wasm')) {
    return serveDir(req, {
      fsRoot: "www",
      urlRoot: "",
      enableCors: true,
      mimeTypes,
    });
  }

  // Serve static files
  return serveDir(req, {
    fsRoot: "www",
    urlRoot: "",
    showDirListing: true,
    enableCors: true,
  });
}, { 
  port: 8000,
  onListen: ({ port }) => console.log(`Server running on http://localhost:${port}`)
}); 