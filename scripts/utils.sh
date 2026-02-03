#!/bin/bash
# ==============================================================================
# Common utilities for Grainlify deployment scripts
# ==============================================================================

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
CONFIG_DIR="$SCRIPT_DIR/config"
DEPLOYMENTS_DIR="$ROOT_DIR/contracts/deployments"

# Default configuration
DEFAULT_NETWORK="testnet"
DEFAULT_LOG_LEVEL="INFO"

# ==============================================================================
# Logging Functions
# ==============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $*"
    if [[ -n "$LOG_FILE" ]]; then
        echo "[INFO] $(date '+%Y-%m-%d %H:%M:%S') - $*" >> "$LOG_FILE"
    fi
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $*"
    if [[ -n "$LOG_FILE" ]]; then
        echo "[SUCCESS] $(date '+%Y-%m-%d %H:%M:%S') - $*" >> "$LOG_FILE"
    fi
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $*"
    if [[ -n "$LOG_FILE" ]]; then
        echo "[WARNING] $(date '+%Y-%m-%d %H:%M:%S') - $*" >> "$LOG_FILE"
    fi
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $*" >&2
    if [[ -n "$LOG_FILE" ]]; then
        echo "[ERROR] $(date '+%Y-%m-%d %H:%M:%S') - $*" >> "$LOG_FILE"
    fi
}

log_debug() {
    if [[ "$LOG_LEVEL" == "DEBUG" ]]; then
        echo -e "${CYAN}[DEBUG]${NC} $(date '+%Y-%m-%d %H:%M:%S') - $*"
        if [[ -n "$LOG_FILE" ]]; then
            echo "[DEBUG] $(date '+%Y-%m-%d %H:%M:%S') - $*" >> "$LOG_FILE"
        fi
    fi
}

# ==============================================================================
# Configuration Functions
# ==============================================================================

load_env() {
    local env_file="${1:-$CONFIG_DIR/.env}"
    
    if [[ -f "$env_file" ]]; then
        log_debug "Loading environment from $env_file"
        set -a
        source "$env_file"
        set +a
    else
        log_warning "Environment file not found: $env_file"
        log_info "Using default configuration. Consider copying .env.example to .env"
    fi
    
    # Set defaults if not specified
    NETWORK="${NETWORK:-$DEFAULT_NETWORK}"
    LOG_LEVEL="${LOG_LEVEL:-$DEFAULT_LOG_LEVEL}"
    VERBOSE="${VERBOSE:-false}"
    DRY_RUN="${DRY_RUN:-false}"
    CONFIRM_DEPLOY="${CONFIRM_DEPLOY:-true}"
}

get_network_config() {
    local network="$1"
    local field="$2"
    local config_file="$CONFIG_DIR/networks.json"
    
    if [[ ! -f "$config_file" ]]; then
        log_error "Network configuration file not found: $config_file"
        return 1
    fi
    
    # Use jq if available, otherwise use a simple grep/sed approach
    if command -v jq &> /dev/null; then
        jq -r ".networks.$network.$field // empty" "$config_file"
    else
        # Fallback: simple parsing (less robust)
        grep -A20 "\"$network\"" "$config_file" | grep "\"$field\"" | head -1 | sed 's/.*: *"\([^"]*\)".*/\1/'
    fi
}

get_contract_config() {
    local contract="$1"
    local field="$2"
    local config_file="$CONFIG_DIR/networks.json"
    
    if command -v jq &> /dev/null; then
        jq -r ".contracts.\"$contract\".$field // empty" "$config_file"
    else
        grep -A10 "\"$contract\"" "$config_file" | grep "\"$field\"" | head -1 | sed 's/.*: *"\([^"]*\)".*/\1/'
    fi
}

# ==============================================================================
# Network Functions
# ==============================================================================

setup_network() {
    local network="${1:-$NETWORK}"
    
    RPC_URL=$(get_network_config "$network" "rpc_url")
    NETWORK_PASSPHRASE=$(get_network_config "$network" "network_passphrase")
    FRIENDBOT_URL=$(get_network_config "$network" "friendbot_url")
    HORIZON_URL=$(get_network_config "$network" "horizon_url")
    EXPLORER_URL=$(get_network_config "$network" "explorer_url")
    
    if [[ -z "$RPC_URL" ]]; then
        log_error "Failed to get RPC URL for network: $network"
        return 1
    fi
    
    log_debug "Network: $network"
    log_debug "RPC URL: $RPC_URL"
    log_debug "Network Passphrase: $NETWORK_PASSPHRASE"
    
    export RPC_URL NETWORK_PASSPHRASE FRIENDBOT_URL HORIZON_URL EXPLORER_URL
}

check_stellar_cli() {
    if ! command -v stellar &> /dev/null; then
        log_error "stellar CLI not found. Please install it with: cargo install --locked stellar-cli"
        return 1
    fi
    
    local version
    version=$(stellar version 2>/dev/null || stellar --version 2>/dev/null || echo "unknown")
    log_debug "stellar CLI version: $version"
}

check_account() {
    local address="$1"
    
    if [[ -z "$address" ]]; then
        log_error "No address provided to check_account"
        return 1
    fi
    
    log_debug "Checking account: $address"
    
    # Try to get account info via Horizon API
    local horizon_url
    horizon_url=$(get_network_config "$NETWORK" "horizon_url")
    
    if [[ -n "$horizon_url" ]]; then
        local response
        response=$(curl -s "$horizon_url/accounts/$address" 2>/dev/null)
        if echo "$response" | grep -q '"id"'; then
            log_debug "Account exists and is funded"
            return 0
        fi
    fi
    
    log_warning "Account not found or not funded: $address"
    return 1
}

fund_account_testnet() {
    local address="$1"
    
    if [[ "$NETWORK" != "testnet" && "$NETWORK" != "local" ]]; then
        log_error "Friendbot is only available on testnet and local networks"
        return 1
    fi
    
    if [[ -z "$FRIENDBOT_URL" || "$FRIENDBOT_URL" == "null" ]]; then
        log_error "Friendbot URL not configured for network: $NETWORK"
        return 1
    fi
    
    log_info "Funding account via Friendbot: $address"
    
    if curl -s "$FRIENDBOT_URL?addr=$address" | grep -q "successful"; then
        log_success "Account funded successfully"
        return 0
    else
        log_warning "Friendbot request may have failed or account already funded"
        return 0
    fi
}

# ==============================================================================
# Contract Functions
# ==============================================================================

build_contract() {
    local contract_path="$1"
    local release="${2:-true}"
    
    log_info "Building contract: $contract_path"
    
    if [[ ! -d "$ROOT_DIR/$contract_path" ]]; then
        log_error "Contract directory not found: $contract_path"
        return 1
    fi
    
    cd "$ROOT_DIR/$contract_path"
    
    local build_cmd="cargo build --target wasm32-unknown-unknown"
    if [[ "$release" == "true" ]]; then
        build_cmd="$build_cmd --release"
    fi
    
    log_debug "Running: $build_cmd"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would run: $build_cmd"
        return 0
    fi
    
    if $build_cmd; then
        log_success "Contract built successfully"
        return 0
    else
        log_error "Contract build failed"
        return 1
    fi
}

get_wasm_path() {
    local contract_name="$1"
    local release="${2:-true}"
    
    local profile="release"
    if [[ "$release" != "true" ]]; then
        profile="debug"
    fi
    
    # Handle different contract names
    local wasm_name="${contract_name//-/_}"
    echo "$ROOT_DIR/target/wasm32-unknown-unknown/$profile/$wasm_name.wasm"
}

install_wasm() {
    local wasm_path="$1"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would upload WASM: $wasm_path" >&2
        echo "dry-run-wasm-hash"
        return 0
    fi
    
    if [[ ! -f "$wasm_path" ]]; then
        log_error "WASM file not found: $wasm_path"
        return 1
    fi
    
    log_info "Uploading WASM: $wasm_path" >&2
    
    local wasm_hash
    wasm_hash=$(stellar contract upload \
        --wasm "$wasm_path" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$NETWORK" 2>&1)
    
    if [[ $? -eq 0 ]]; then
        # Extract just the hash from the output (last line usually)
        wasm_hash=$(echo "$wasm_hash" | tail -1 | tr -d '[:space:]')
        log_success "WASM uploaded. Hash: $wasm_hash" >&2
        echo "$wasm_hash"
        return 0
    else
        log_error "Failed to upload WASM: $wasm_hash"
        return 1
    fi
}

deploy_contract() {
    local wasm_hash="$1"
    local salt="${2:-}"
    
    log_info "Deploying contract with WASM hash: $wasm_hash" >&2
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would deploy contract" >&2
        echo "dry-run-contract-id"
        return 0
    fi
    
    local result
    if [[ -n "$salt" ]]; then
        result=$(stellar contract deploy \
            --wasm-hash "$wasm_hash" \
            --source-account "$STELLAR_SECRET_KEY" \
            --network "$NETWORK" \
            --salt "$salt" 2>&1)
    else
        result=$(stellar contract deploy \
            --wasm-hash "$wasm_hash" \
            --source-account "$STELLAR_SECRET_KEY" \
            --network "$NETWORK" 2>&1)
    fi
    
    if [[ $? -eq 0 ]]; then
        # Extract just the contract ID from the output
        result=$(echo "$result" | tail -1 | tr -d '[:space:]')
        log_success "Contract deployed. ID: $result" >&2
        echo "$result"
        return 0
    else
        log_error "Failed to deploy contract: $result"
        return 1
    fi
}

invoke_contract() {
    local contract_id="$1"
    local function_name="$2"
    shift 2
    local args=("$@")
    
    log_info "Invoking $function_name on contract $contract_id"
    
    log_debug "Args: ${args[*]}"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would invoke: $function_name"
        return 0
    fi
    
    local result
    result=$(stellar contract invoke \
        --id "$contract_id" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$NETWORK" \
        -- "$function_name" "${args[@]}" 2>&1)
    
    if [[ $? -eq 0 ]]; then
        log_success "Function invoked successfully"
        echo "$result"
        return 0
    else
        log_error "Failed to invoke function: $result"
        return 1
    fi
}

get_contract_version() {
    local contract_id="$1"
    
    local result
    # Use --send=no to simulate without submitting (read-only call)
    result=$(stellar contract invoke \
        --id "$contract_id" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$NETWORK" \
        --send=no \
        -- get_version 2>&1) || true
    
    if [[ -n "$result" && "$result" != *"error"* && "$result" != *"Error"* ]]; then
        # Extract numeric value from result
        echo "$result" | grep -oE '[0-9]+' | head -1
    else
        echo "0"
    fi
}

# ==============================================================================
# Deployment Record Functions
# ==============================================================================

init_deployments_dir() {
    mkdir -p "$DEPLOYMENTS_DIR"
    mkdir -p "$DEPLOYMENTS_DIR/backups"
    mkdir -p "$DEPLOYMENTS_DIR/logs"
}

get_deployment_file() {
    local network="$1"
    echo "$DEPLOYMENTS_DIR/${network}_deployments.json"
}

save_deployment() {
    local network="$1"
    local contract_name="$2"
    local contract_id="$3"
    local wasm_hash="$4"
    local version="${5:-1}"
    
    init_deployments_dir
    
    local deployment_file
    deployment_file=$(get_deployment_file "$network")
    
    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    local entry="{
        \"contract_name\": \"$contract_name\",
        \"contract_id\": \"$contract_id\",
        \"wasm_hash\": \"$wasm_hash\",
        \"version\": $version,
        \"deployed_at\": \"$timestamp\",
        \"network\": \"$network\"
    }"
    
    if [[ ! -f "$deployment_file" ]]; then
        echo "{\"deployments\": []}" > "$deployment_file"
    fi
    
    if command -v jq &> /dev/null; then
        local tmp_file="$deployment_file.tmp"
        jq ".deployments += [$entry]" "$deployment_file" > "$tmp_file"
        mv "$tmp_file" "$deployment_file"
    else
        # Simple append (less robust)
        log_warning "jq not available, deployment record may be incomplete"
    fi
    
    log_success "Deployment saved to $deployment_file"
}

get_latest_deployment() {
    local network="$1"
    local contract_name="$2"
    
    local deployment_file
    deployment_file=$(get_deployment_file "$network")
    
    if [[ ! -f "$deployment_file" ]]; then
        return 1
    fi
    
    if command -v jq &> /dev/null; then
        jq -r ".deployments | map(select(.contract_name == \"$contract_name\")) | last" "$deployment_file"
    else
        log_error "jq required to read deployment records"
        return 1
    fi
}

backup_deployment() {
    local network="$1"
    local contract_name="$2"
    
    local deployment_file
    deployment_file=$(get_deployment_file "$network")
    
    if [[ -f "$deployment_file" ]]; then
        local backup_file="$DEPLOYMENTS_DIR/backups/${network}_deployments_$(date +%Y%m%d_%H%M%S).json"
        cp "$deployment_file" "$backup_file"
        log_info "Deployment backup created: $backup_file"
    fi
}

# ==============================================================================
# Confirmation Functions
# ==============================================================================

confirm_action() {
    local message="$1"
    
    if [[ "$CONFIRM_DEPLOY" != "true" ]]; then
        return 0
    fi
    
    echo -e "${YELLOW}$message${NC}"
    read -p "Continue? [y/N] " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        return 0
    else
        log_info "Action cancelled by user"
        return 1
    fi
}

print_separator() {
    echo "=============================================================================="
}

print_banner() {
    local message="$1"
    print_separator
    echo -e "${CYAN}$message${NC}"
    print_separator
}

# ==============================================================================
# Validation Functions
# ==============================================================================

validate_address() {
    local address="$1"
    
    # Basic Stellar address validation (G... format, 56 characters)
    if [[ "$address" =~ ^G[A-Z0-9]{55}$ ]]; then
        return 0
    else
        return 1
    fi
}

validate_contract_id() {
    local contract_id="$1"
    
    # Contract IDs are C... format, 56 characters
    if [[ "$contract_id" =~ ^C[A-Z0-9]{55}$ ]]; then
        return 0
    else
        return 1
    fi
}

validate_wasm_hash() {
    local hash="$1"
    
    # WASM hash is 64 hex characters
    if [[ "$hash" =~ ^[a-f0-9]{64}$ ]]; then
        return 0
    else
        return 1
    fi
}

# ==============================================================================
# Cleanup on Exit
# ==============================================================================

cleanup() {
    cd "$ROOT_DIR"
}

trap cleanup EXIT

# ==============================================================================
# Export Functions
# ==============================================================================

export -f log_info log_success log_warning log_error log_debug
export -f load_env get_network_config get_contract_config setup_network
export -f check_stellar_cli check_account fund_account_testnet
export -f build_contract get_wasm_path install_wasm deploy_contract invoke_contract
export -f save_deployment get_latest_deployment backup_deployment
export -f confirm_action print_separator print_banner
export -f validate_address validate_contract_id validate_wasm_hash
