# Release v${VERSION}

**Full Changelog**: https://github.com/eddieland/confluence-dl/compare/v${PREVIOUS_TAG}...v${VERSION}

## Download and Install

### Ubuntu/Linux

```bash
# Download the release
curl -fL -o confluence-dl-linux-x86_64.tar.gz https://github.com/eddieland/confluence-dl/releases/download/v${VERSION}/confluence-dl-linux-x86_64-v${VERSION}.tar.gz

# Extract the archive
tar -xzf confluence-dl-linux-x86_64.tar.gz

# Install to /usr/local/bin (requires sudo)
sudo cp confluence-dl /usr/local/bin/

# Make executable (if not already)
sudo chmod +x /usr/local/bin/confluence-dl

# Clean up downloaded files
rm confluence-dl-linux-x86_64.tar.gz confluence-dl
```

### macOS

```bash
# Download the release
curl -fL -o confluence-dl-macos-x86_64.tar.gz https://github.com/eddieland/confluence-dl/releases/download/v${VERSION}/confluence-dl-macos-x86_64-v${VERSION}.tar.gz

# Extract the archive
tar -xzf confluence-dl-macos-x86_64.tar.gz

# Install to /usr/local/bin (requires sudo)
sudo cp confluence-dl /usr/local/bin/

# Make executable (if not already)
sudo chmod +x /usr/local/bin/confluence-dl

# Clean up downloaded files
rm confluence-dl-macos-x86_64.tar.gz confluence-dl
```

## Quick Install

### Ubuntu/Linux

```bash
curl -fL https://github.com/eddieland/confluence-dl/releases/download/v${VERSION}/confluence-dl-linux-x86_64-v${VERSION}.tar.gz | tar -xz && sudo cp confluence-dl /usr/local/bin/ && sudo chmod +x /usr/local/bin/confluence-dl && rm confluence-dl
```

### macOS

```bash
curl -fL https://github.com/eddieland/confluence-dl/releases/download/v${VERSION}/confluence-dl-macos-x86_64-v${VERSION}.tar.gz | tar -xz && sudo cp confluence-dl /usr/local/bin/ && sudo chmod +x /usr/local/bin/confluence-dl && rm confluence-dl
```

### OCI / Docker Image

```bash
docker pull ghcr.io/eddieland/confluence-dl:v${VERSION}
```

Run the CLI directly from the container:

```bash
docker run --rm ghcr.io/eddieland/confluence-dl:v${VERSION} --help
```

## Verify Installation

```bash
# Check if confluence-dl is installed and accessible
which confluence-dl

# Check version
confluence-dl --version
```
