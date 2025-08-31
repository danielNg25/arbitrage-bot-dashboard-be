.PHONY: help build run check test clean config docs

# Default target
help:
	@echo "Available targets:"
	@echo "  build         - Build the project"
	@echo "  run           - Run the API server"
	@echo "  check         - Check if the project compiles"
	@echo "  test          - Run tests"
	@echo "  clean         - Clean build artifacts"
	@echo "  config        - Generate configuration files"
	@echo "  mongodb-setup - Setup MongoDB configuration"
	@echo "  docs          - Generate documentation"

# Build the project
build:
	cargo build

# Build in release mode
build-release:
	cargo build --release

# Run the API server
run:
	cargo run

# Run with specific config
run-dev:
	RUST_LOG=info cargo run

# Check if the project compiles
check:
	cargo check

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Generate configuration files
config:
	@echo "Generating configuration files..."
	@mkdir -p config
	@if [ -f "config.toml" ]; then \
		echo "config.toml already exists, skipping..."; \
	else \
		echo "Creating config.toml..."; \
		cp config/config.toml config.toml 2>/dev/null || \
		echo "# Arbitrage Bot API Configuration" > config.toml && \
		echo "" >> config.toml && \
		echo "[server]" >> config.toml && \
		echo "host = \"127.0.0.1\"" >> config.toml && \
		echo "port = 8080" >> config.toml && \
		echo "log_level = \"info\"" >> config.toml && \
		echo "" >> config.toml && \
		echo "[database]" >> config.toml && \
		echo "uri = \"mongodb://localhost:27017\"" >> config.toml && \
		echo "database_name = \"arbitrage_bot\"" >> config.toml && \
		echo "connection_timeout_ms = 5000" >> config.toml && \
		echo "max_pool_size = 10" >> config.toml && \
		echo "" >> config.toml && \
		echo "[cors]" >> config.toml && \
		echo "allowed_origins = [\"http://localhost:3000\"]" >> config.toml && \
		echo "allowed_methods = [\"GET\", \"POST\", \"PUT\", \"DELETE\"]" >> config.toml && \
		echo "allowed_headers = [\"Authorization\", \"Accept\", \"Content-Type\"]" >> config.toml && \
		echo "supports_credentials = true" >> config.toml; \
	fi
	@echo "Configuration files ready!"

# Setup MongoDB configuration
mongodb-setup: config
	@echo "Setting up MongoDB configuration..."
	@./scripts/setup_mongodb.sh

# Generate documentation
docs:
	cargo doc --open

# Install dependencies
install:
	cargo install cargo-watch

# Watch for changes and rebuild
watch:
	cargo watch -x check -x test -x run

# Format code
fmt:
	cargo fmt

# Clippy linting
clippy:
	cargo clippy

# Full check (check + clippy + test)
full-check: check clippy test

# Docker build
docker-build:
	docker build -t arbitrage-bot-api .

# Docker run
docker-run:
	docker run -p 8080:8080 arbitrage-bot-api

# Setup development environment
setup: config install
	@echo "Development environment setup complete!"
	@echo "Edit config.toml to customize your settings."
	@echo "Run 'make mongodb-setup' to configure MongoDB."
	@echo "Run 'make run' to start the API server."
