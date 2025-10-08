### Makefile

.PHONY: help
help: ## Display this help
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {section="General"} /^### /{section=substr($$0,5); printf "\n\033[1m%s\033[0m\n", section} /^[a-zA-Z0-9_-]+:.*?## / {match($$0, /## (.*)$$/, a); printf "  \033[36m%-18s\033[0m %s\n", substr($$1,1,length($$1)-1), a[1]}' $(MAKEFILE_LIST)

### Development

.PHONY: fmt
fmt: ## Format code using rustfmt
	cargo fmt --all
	cargo clippy --fix --allow-dirty

.PHONY: lint
lint: ## Run clippy for linting
	cargo clippy -- -D warnings

.PHONY: lint-all
lint-all: ## Run clippy with all features
	cargo clippy --all-targets -- -D warnings

.PHONY: test
test: build ## Run tests
	cargo nextest run --all-targets

.PHONY: check
check: ## Run cargo check
	cargo check

.PHONY: doc
doc: ## Generate documentation
	cargo doc --no-deps

.PHONY: watch-test
watch-test: ## Run tests in watch mode (requires cargo-watch)
	cargo watch -x "nextest run --all-targets"

.PHONY: all
all: fmt lint test ## Run fmt, lint, and test

### Snapshot Testing

.PHONY: insta-review
insta-review: ## Review Insta snapshots
	cargo insta review

.PHONY: insta-accept
insta-accept: ## Accept all pending Insta snapshots
	cargo insta accept

.PHONY: insta-reject
insta-reject: ## Reject all pending Insta snapshots
	cargo insta reject

.PHONY: update-snapshots
update-snapshots: ## Run tests and update snapshots
	INSTA_UPDATE=1 cargo nextest run

### Analysis

.PHONY: cloc
cloc: ## Count lines of code using Docker
	docker run --rm -v "$(PWD):/tmp" aldanial/cloc /tmp \
		--exclude-dir=.git,.github,example,docs,ref,target \
		--fullpath

### Coverage

.PHONY: coverage
coverage: ## Run code coverage
	cargo llvm-cov nextest --all-targets

.PHONY: coverage-html
coverage-html: ## Generate HTML coverage report
	cargo llvm-cov nextest --all-targets --html

.PHONY: coverage-open
coverage-open: ## Generate HTML coverage report and open it in browser
	cargo llvm-cov nextest --all-targets --html --open

.PHONY: coverage-report
coverage-report: ## Generate LCOV report
	cargo llvm-cov nextest --all-targets --lcov --output-path lcov.info

### Build

.PHONY: build
build: ## Build the project
	cargo build

.PHONY: release
release: ## Build release version
	cargo build --release

.PHONY: release-size
release-size: ## Build size-optimized release version
	cargo build --release
	@echo "\nBinary size before compression:"
	@du -h target/release/confluence-dl

.PHONY: clean
clean: ## Clean build artifacts
	cargo clean

.PHONY: run
run: ## Run the application
	cargo run
