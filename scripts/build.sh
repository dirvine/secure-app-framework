#!/usr/bin/env bash
set -euo pipefail

# Secure App Framework Build Script
# Handles cross-compilation, packaging, and reproducible builds

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-pc-windows-msvc"
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Setup reproducible build environment
setup_reproducible_env() {
    log_info "Setting up reproducible build environment..."

    # Set SOURCE_DATE_EPOCH for reproducible builds
    export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --pretty=%ct 2>/dev/null || date +%s)}"
    export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$(pwd)=/source"
    export CARGO_TERM_COLOR=never
    export RUST_BACKTRACE=1

    # Create dist directory
    mkdir -p "$PROJECT_ROOT/dist"

    log_success "Reproducible environment configured"
}

# Build WASM component
build_wasm_component() {
    log_info "Building WASM component..."

    cd "$PROJECT_ROOT"

    # Build the component
    cargo component build --release --package saf-component-demo

    # Verify the component was built
    if [[ -f "target/wasm32-wasip1/release/saf_component_demo.wasm" ]]; then
        log_success "WASM component built successfully"

        # Calculate checksum
        local checksum
        checksum=$(sha256sum "target/wasm32-wasip1/release/saf_component_demo.wasm" | cut -d' ' -f1)
        echo "$checksum" > "target/wasm32-wasip1/release/saf_component_demo.wasm.sha256"

        log_info "Component checksum: $checksum"
    else
        log_error "WASM component build failed"
        exit 1
    fi
}

# Build broker for specific target
build_broker() {
    local target="$1"
    log_info "Building broker for $target..."

    cd "$PROJECT_ROOT"

    # Set target-specific flags
    case "$target" in
        "x86_64-pc-windows-msvc")
            export RUSTFLAGS="$RUSTFLAGS -C target-feature=+crt-static"
            ;;
        "x86_64-apple-darwin"|"aarch64-apple-darwin")
            export MACOSX_DEPLOYMENT_TARGET="10.15"
            ;;
    esac

    # Build broker with wasmtime-host feature
    if cargo build --release --target "$target" --package broker --features wasmtime-host,ui; then
        log_success "Broker built for $target"

        # Calculate checksum
        local binary_name="broker"
        if [[ "$target" == *"windows"* ]]; then
            binary_name="broker.exe"
        fi

        local binary_path="target/$target/release/$binary_name"
        if [[ -f "$binary_path" ]]; then
            local checksum
            checksum=$(sha256sum "$binary_path" | cut -d' ' -f1)
            echo "$checksum" > "$binary_path.sha256"
            log_info "Broker checksum for $target: $checksum"
        fi
    else
        log_error "Broker build failed for $target"
        return 1
    fi
}

# Build UI
build_ui() {
    log_info "Building UI..."

    cd "$PROJECT_ROOT/crates/ui"

    if [[ -f "src-tauri/tauri.conf.json" ]]; then
        # Build with Tauri
        if cargo tauri build --no-bundle; then
            log_success "UI built successfully"
        else
            log_error "UI build failed"
            return 1
        fi
    else
        log_warn "Tauri configuration not found, skipping UI build"
    fi
}

# Create packages
create_packages() {
    local target="$1"
    log_info "Creating packages for $target..."

    cd "$PROJECT_ROOT"

    # Create package directory
    local package_dir="dist/secure-app-framework-$target"
    mkdir -p "$package_dir"

    # Copy binaries
    case "$target" in
        "x86_64-unknown-linux-gnu")
            cp "target/$target/release/broker" "$package_dir/" 2>/dev/null || true
            cp "target/wasm32-wasip1/release/saf_component_demo.wasm" "$package_dir/" 2>/dev/null || true
            ;;
        "x86_64-pc-windows-msvc")
            cp "target/$target/release/broker.exe" "$package_dir/" 2>/dev/null || true
            cp "target/wasm32-wasip1/release/saf_component_demo.wasm" "$package_dir/" 2>/dev/null || true
            ;;
        "x86_64-apple-darwin"|"aarch64-apple-darwin")
            cp "target/$target/release/broker" "$package_dir/" 2>/dev/null || true
            cp "target/wasm32-wasip1/release/saf_component_demo.wasm" "$package_dir/" 2>/dev/null || true
            ;;
    esac

    # Copy configuration files
    cp "README.md" "$package_dir/" 2>/dev/null || true
    cp "LICENSE" "$package_dir/" 2>/dev/null || true

    # Create checksums file
    cd "$package_dir"
    find . -type f -not -name "*.sha256" -exec sha256sum {} \; > checksums.sha256
    cd "$PROJECT_ROOT"

    # Create tarball/zip
    case "$target" in
        *"windows"*)
            if command -v zip >/dev/null 2>&1; then
                zip -r "dist/secure-app-framework-$target.zip" "$package_dir"
                log_success "Package created: dist/secure-app-framework-$target.zip"
            fi
            ;;
        *)
            if command -v tar >/dev/null 2>&1; then
                tar -czf "dist/secure-app-framework-$target.tar.gz" "$package_dir"
                log_success "Package created: dist/secure-app-framework-$target.tar.gz"
            fi
            ;;
    esac
}

# Generate SBOM
generate_sbom() {
    log_info "Generating SBOM..."

    cd "$PROJECT_ROOT"

    # CycloneDX SBOM for Cargo
    if command -v cyclonedx >/dev/null 2>&1; then
        cargo install cyclonedx-bom --quiet
        cyclonedx-bom -o "dist/sbom-cyclonedx-cargo.json"
        log_success "Cargo SBOM generated"
    fi

    # Syft SBOM for binary artifacts
    if command -v syft >/dev/null 2>&1; then
        syft dir:. -o cyclonedx-json > "dist/sbom-cyclonedx-syft.json"
        log_success "Binary SBOM generated"
    fi
}

# Sign artifacts (placeholder for actual signing)
sign_artifacts() {
    log_info "Signing artifacts..."

    # This would integrate with cosign, signtool, codesign, etc.
    # For now, just create signature placeholders
    for file in dist/*.{zip,tar.gz,wasm}; do
        if [[ -f "$file" ]]; then
            echo "SIGNATURE_PLACEHOLDER" > "$file.sig"
            log_info "Created signature placeholder for $(basename "$file")"
        fi
    done

    log_success "Artifacts signed (placeholders)"
}

# Main build function
main() {
    local build_targets=("${TARGETS[@]}")
    local skip_wasm=false
    local skip_ui=false
    local skip_package=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --target)
                build_targets=("$2")
                shift 2
                ;;
            --skip-wasm)
                skip_wasm=true
                shift
                ;;
            --skip-ui)
                skip_ui=true
                shift
                ;;
            --skip-package)
                skip_package=true
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --target TARGET    Build for specific target only"
                echo "  --skip-wasm        Skip WASM component build"
                echo "  --skip-ui          Skip UI build"
                echo "  --skip-package     Skip packaging"
                echo "  --help             Show this help"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done

    log_info "Starting Secure App Framework build..."
    log_info "Targets: ${build_targets[*]}"

    setup_reproducible_env

    # Build WASM component
    if [[ "$skip_wasm" != true ]]; then
        build_wasm_component
    fi

    # Build for each target
    for target in "${build_targets[@]}"; do
        log_info "Building for target: $target"

        if build_broker "$target"; then
            if [[ "$skip_package" != true ]]; then
                create_packages "$target"
            fi
        else
            log_warn "Skipping packaging for failed target: $target"
        fi
    done

    # Build UI
    if [[ "$skip_ui" != true ]]; then
        build_ui
    fi

    # Generate SBOM and sign
    generate_sbom
    sign_artifacts

    log_success "Build completed successfully!"
    log_info "Artifacts available in: $PROJECT_ROOT/dist/"

    # List created artifacts
    echo ""
    echo "Created artifacts:"
    find "$PROJECT_ROOT/dist" -type f | sort
}

# Run main function
main "$@"
