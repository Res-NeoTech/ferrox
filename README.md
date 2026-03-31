# Ferrox

Ferrox is a fast, secure, and ultra-lightweight web server written in Rust from first principles. Designed to explore the internal workings of an HTTP server, Ferrox strips away bloated frameworks to deliver high performance while exposing the fundamentals of asynchronous network programming. 

While built as an educational exploration, Ferrox is highly capable of serving static files and Single Page Applications (SPAs) efficiently with a custom HTTP stack.

## Why this project exists

Ferrox is built to provide a deep understanding of web server mechanics, specifically focusing on:

- Asynchronous TCP-based request handling in Rust using the Tokio runtime.
- Manual HTTP request parsing and response generation without relying on large frameworks.
- Static file serving, robust MIME type detection, and flexible SPA routing.
- Safe path resolution for files on disk to prevent directory traversal attacks.
- Clean, customizable fallback error pages.

## Key Features

- **Flexible Routing:** Serves standard static files or operates as a router for Single Page Applications (SPAs).
- **HTTPS & TLS Support:** Built-in TLS support powered by `tokio-rustls`, allowing secure connections via custom or Certbot-generated certificates.
- **High Performance:** Handles thousands of concurrent connections using Tokio's lightweight asynchronous tasks in an event-driven architecture.
- **Robust Parsing & Security:** Safely parses HTTP requests using dynamic buffers to protect against malformed data. Includes connection timeouts to mitigate Slowloris attacks.
- **Configurable & Extensible:** Easily managed via a `ferrox.yml` configuration file, allowing customization of ports, served directories, logging, and default HTTP security headers (e.g., `X-Content-Type-Options: nosniff`).
- **Docker Ready:** Includes a multi-stage Alpine Dockerfile for ultra-lightweight, natively compiled deployments.

## How it works

At a high level, Ferrox:

1. Binds an asynchronous TCP listener to the configured HTTP and HTTPS ports.
2. Loads and applies server configuration from `ferrox.yml`.
3. Accepts incoming connections and spawns a lightweight Tokio task for each request.
4. Safely reads the raw request into a dynamic buffer until the `\r\n\r\n` boundary.
5. Parses the request line (method, path, HTTP version) efficiently, minimizing memory allocations.
6. Maps the requested path into the target directory (e.g., `www/`), detecting content types via `mime_guess`.
7. Serves the file via async I/O, acts as an SPA router, or returns an appropriate error page (`400`, `403`, `404`, `500`).
8. Writes a full HTTP response back to the client.

## Configuration

Ferrox is configured using a simple `ferrox.yml` file. By default, it supports defining:
- HTTP and HTTPS listening ports and addresses.
- Router mode (`static` or `spa`).
- Target directories for serving (`www`) and logging (`logs`).
- Default injected security headers.
- TLS certificate and key paths.

## Deployment

### Running Locally with Cargo
```bash
cargo run --release
```
Then open `http://127.0.0.1/` (or your configured port) in your browser.

## Current Limitations

While Ferrox is evolving, it maintains a minimal footprint. Current limitations include:
- Request bodies are not fully parsed yet (optimized primarily for `GET`/`HEAD` requests).
- No keep-alive support (`Connection: close` is hardcoded for now).
- It is an experimental project and should be evaluated thoroughly before being used as a hardened production server facing the public internet.

## Future Direction

Natural next steps for the project include:
- [x] YAML configuration parsing
- [x] TLS Support (HTTPS) via `tokio-rustls`
- [x] Zero-copy request parsing for an even lower memory footprint.
- [ ] Keep-alive support for persistent connections.
- [ ] Implementing comprehensive tests and benchmarks.
- [ ] Reverse proxy feature.
- [ ] Host configuration.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for more details.