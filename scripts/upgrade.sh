#!/bin/bash
# ==============================================================================
# Grainlify Contract Upgrade Script
# ==============================================================================
# Upgrades deployed smart contracts with new WASM code.
#
# This script performs:
# 1. Pre-upgrade validation (version check, state backup)
# 2. WASM build and installation
# 3. Contract upgrade execution
# 4. Post-upgrade verification
# 5. State migration (if required)
#
# Usage:
#   ./upgrade.sh [options]
#
# Options:
#   -n, --network <network>     Network to upgrade on (testnet, mainnet)
#   -c, --contract <contract>   Contract to upgrade (required)
#   -i, --id <contract_id>      Contract ID to upgrade (auto-detected if not provided)
#   -w, --wasm <wasm_hash>      Use existing WASM hash instead of building new
#   -m, --migrate               Run migration after upgrade
#   -t, --target-version <v>    Target version for migration
#   -d, --dry-run               Simulate upgrade without making changes
#   -y, --yes                   Skip confirmation prompts
#   -v, --verbose               Enable verbose output
#   -h, --help                  Show this help message
#
# Examples:
#   ./upgrade.sh -n testnet -c grainlify-core
#   ./upgrade.sh -n mainnet -c program-escrow -m -t 20000
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

# ==============================================================================
# Default Configuration
# ==============================================================================

UPGRADE_NETWORK="${NETWORK:-testnet}"
UPGRADE_CONTRACT=""
CONTRACT_ID=""
WASM_HASH=""
RUN_MIGRATION="false"
TARGET_VERSION=""
SKIP_CONFIRM="false"

# ==============================================================================
# Help Function
# ==============================================================================

show_help() {
    cat << EOF
Grainlify Contract Upgrade Script

Usage: $0 [options]

Options:
  -n, --network <network>     Network to upgrade on (testnet, mainnet)
                              Default: testnet
  -c, --contract <contract>   Contract to upgrade. Required. Options:
                              - grainlify-core
                              - program-escrow
                              - bounty-escrow
  -i, --id <contract_id>      Contract ID to upgrade
                              Auto-detected from deployment records if not provided
  -w, --wasm <wasm_hash>      Use existing WASM hash instead of building new
  -m, --migrate               Run migration after upgrade
  -t, --target-version <v>    Target version for migration (numeric, e.g., 20000)
  -d, --dry-run               Simulate upgrade without making changes
  -y, --yes                   Skip confirmation prompts
  -v, --verbose               Enable verbose output
  -h, --help                  Show this help message

Examples:
  $0 -n testnet -c grainlify-core
  $0 -n mainnet -c program-escrow -m -t 20000
  $0 -n testnet -c grainlify-core -w abc123...

EOF
}

# ==============================================================================
# Parse Arguments
# ==============================================================================

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--network)
                UPGRADE_NETWORK="$2"
                shift 2
                ;;
            -c|--contract)
                UPGRADE_CONTRACT="$2"
                shift 2
                ;;
            -i|--id)
                CONTRACT_ID="$2"
                shift 2
                ;;
            -w|--wasm)
                WASM_HASH="$2"
                shift 2
                ;;
            -m|--migrate)
                RUN_MIGRATION="true"
                shift
                ;;
            -t|--target-version)
                TARGET_VERSION="$2"
                shift 2
                ;;
            -d|--dry-run)
                DRY_RUN="true"
                shift
                ;;
            -y|--yes)
                SKIP_CONFIRM="true"
                CONFIRM_DEPLOY="false"
                shift
                ;;
            -v|--verbose)
                VERBOSE="true"
                LOG_LEVEL="DEBUG"
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# ==============================================================================
# Validation Functions
# ==============================================================================

validate_upgrade_params() {
    log_info "Validating upgrade parameters..."
    
    # Contract is required
    if [[ -z "$UPGRADE_CONTRACT" ]]; then
        log_error "Contract name is required. Use -c <contract_name>"
        exit 1
    fi
    
    # Validate contract name
    case "$UPGRADE_CONTRACT" in
        grainlify-core|program-escrow|bounty-escrow)
            ;;
        *)
            log_error "Invalid contract: $UPGRADE_CONTRACT"
            exit 1
            ;;
    esac
    
    # Get contract ID if not provided
    if [[ -z "$CONTRACT_ID" ]]; then
        log_info "Looking up contract ID from deployment records..."
        
        local deployment
        deployment=$(get_latest_deployment "$UPGRADE_NETWORK" "$UPGRADE_CONTRACT")
        
        if [[ -n "$deployment" && "$deployment" != "null" ]]; then
            CONTRACT_ID=$(echo "$deployment" | jq -r '.contract_id // empty')
        fi
        
        if [[ -z "$CONTRACT_ID" ]]; then
            log_error "Contract ID not found. Provide it with -i <contract_id>"
            exit 1
        fi
        
        log_info "Found contract ID: $CONTRACT_ID"
    fi
    
    # Validate contract ID format
    if ! validate_contract_id "$CONTRACT_ID"; then
        log_error "Invalid contract ID format: $CONTRACT_ID"
        exit 1
    fi
    
    # Validate WASM hash if provided
    if [[ -n "$WASM_HASH" ]]; then
        if ! validate_wasm_hash "$WASM_HASH"; then
            log_error "Invalid WASM hash format: $WASM_HASH"
            exit 1
        fi
    fi
    
    log_success "Upgrade parameters validated"
}

# ==============================================================================
# Pre-Upgrade Functions
# ==============================================================================

pre_upgrade_checks() {
    print_banner "Pre-Upgrade Checks"
    
    # Check current version
    log_info "Checking current contract version..."
    
    local current_version
    current_version=$(get_contract_version "$CONTRACT_ID")
    
    log_info "Current version: $current_version"
    
    # Backup current deployment state
    log_info "Creating deployment backup..."
    backup_deployment "$UPGRADE_NETWORK" "$UPGRADE_CONTRACT"
    
    # Verify admin access
    log_info "Verifying admin access..."
    
    local admin_check
    admin_check=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$UPGRADE_NETWORK" \
        --send=no \
        -- get_admin 2>&1) || true
    
    if [[ -n "$admin_check" && "$admin_check" != *"Error"* ]]; then
        log_debug "Admin address: $admin_check"
        
        if [[ "$admin_check" != *"$ADMIN_ADDRESS"* && "$admin_check" != *"$STELLAR_PUBLIC_KEY"* ]]; then
            log_warning "Current account may not be admin. Upgrade may fail."
        fi
    fi
    
    # Save pre-upgrade state
    log_info "Saving pre-upgrade state..."
    save_pre_upgrade_state "$CONTRACT_ID" "$UPGRADE_CONTRACT"
    
    log_success "Pre-upgrade checks passed"
    
    return 0
}

save_pre_upgrade_state() {
    local contract_id="$1"
    local contract_name="$2"
    
    local state_file="$DEPLOYMENTS_DIR/backups/${contract_name}_pre_upgrade_$(date +%Y%m%d_%H%M%S).json"
    
    # Get contract state (version, balances, etc.)
    local state_data="{"
    state_data+="\"contract_id\": \"$contract_id\","
    state_data+="\"contract_name\": \"$contract_name\","
    state_data+="\"network\": \"$UPGRADE_NETWORK\","
    state_data+="\"timestamp\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\","
    
    # Get version
    local version
    version=$(get_contract_version "$contract_id")
    state_data+="\"version\": $version"
    
    state_data+="}"
    
    echo "$state_data" > "$state_file"
    
    log_info "Pre-upgrade state saved to: $state_file"
}

# ==============================================================================
# Upgrade Functions
# ==============================================================================

build_and_install_wasm() {
    if [[ -n "$WASM_HASH" ]]; then
        log_info "Using provided WASM hash: $WASM_HASH"
        return 0
    fi
    
    log_info "Building new WASM..."
    
    local contract_path
    contract_path=$(get_contract_config "$UPGRADE_CONTRACT" "path")
    
    if ! build_contract "$contract_path" "true"; then
        log_error "Failed to build contract"
        return 1
    fi
    
    local wasm_path
    wasm_path=$(get_wasm_path "$UPGRADE_CONTRACT" "true")
    
    log_info "Installing WASM..."
    WASM_HASH=$(install_wasm "$wasm_path")
    
    if [[ -z "$WASM_HASH" ]]; then
        log_error "Failed to install WASM"
        return 1
    fi
    
    log_success "WASM installed: $WASM_HASH"
}

execute_upgrade() {
    print_banner "Executing Upgrade"
    
    log_info "Upgrading contract $CONTRACT_ID with WASM $WASM_HASH"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would upgrade contract"
        return 0
    fi
    
    # Execute the upgrade
    local result
    result=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$UPGRADE_NETWORK" \
        -- upgrade \
        --new_wasm_hash "$WASM_HASH" 2>&1)
    
    if [[ $? -eq 0 ]]; then
        log_success "Upgrade transaction submitted"
        echo "$result"
    else
        log_error "Upgrade failed: $result"
        return 1
    fi
}

# ==============================================================================
# Post-Upgrade Functions
# ==============================================================================

post_upgrade_verification() {
    print_banner "Post-Upgrade Verification"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would verify upgrade"
        return 0
    fi
    
    # Wait for confirmation
    log_info "Waiting for upgrade confirmation..."
    sleep 5
    
    # Verify contract is still accessible
    log_info "Verifying contract accessibility..."
    
    local new_version
    new_version=$(get_contract_version "$CONTRACT_ID")
    
    log_info "Post-upgrade version: $new_version"
    
    # Basic health check - try to call a read function
    local health_check
    health_check=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$UPGRADE_NETWORK" \
        --send=no \
        -- get_version 2>&1) || true
    
    if [[ -n "$health_check" && "$health_check" != *"error"* ]]; then
        log_success "Contract health check passed"
    else
        log_warning "Contract health check returned unexpected result"
    fi
    
    # Update deployment records
    save_deployment "$UPGRADE_NETWORK" "$UPGRADE_CONTRACT" "$CONTRACT_ID" "$WASM_HASH" "$new_version"
    
    log_success "Post-upgrade verification complete"
}

# ==============================================================================
# Migration Functions
# ==============================================================================

run_migration() {
    if [[ "$RUN_MIGRATION" != "true" ]]; then
        return 0
    fi
    
    print_banner "Running Migration"
    
    if [[ -z "$TARGET_VERSION" ]]; then
        log_error "Target version required for migration. Use -t <version>"
        return 1
    fi
    
    log_info "Migrating to version: $TARGET_VERSION"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would run migration"
        return 0
    fi
    
    # Generate migration hash (placeholder - in production this would be meaningful)
    local migration_hash="0000000000000000000000000000000000000000000000000000000000000000"
    
    # Execute migration
    local result
    result=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$UPGRADE_NETWORK" \
        -- migrate \
        --target_version "$TARGET_VERSION" \
        --migration_hash "$migration_hash" 2>&1)
    
    if [[ $? -eq 0 ]]; then
        log_success "Migration completed successfully"
        echo "$result"
    else
        log_error "Migration failed: $result"
        return 1
    fi
    
    # Verify migration
    log_info "Verifying migration..."
    
    local migration_state
    migration_state=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$UPGRADE_NETWORK" \
        --send=no \
        -- get_migration_state 2>&1) || true
    
    if [[ -n "$migration_state" ]]; then
        log_info "Migration state: $migration_state"
    fi
}

# ==============================================================================
# Main Function
# ==============================================================================

main() {
    parse_args "$@"
    
    load_env
    
    NETWORK="$UPGRADE_NETWORK"
    setup_network "$UPGRADE_NETWORK"
    init_deployments_dir
    
    print_banner "Grainlify Contract Upgrade"
    echo "Network:  $UPGRADE_NETWORK"
    echo "Contract: $UPGRADE_CONTRACT"
    echo "Migrate:  $RUN_MIGRATION"
    echo "Dry Run:  $DRY_RUN"
    echo ""
    
    # Validate environment
    check_stellar_cli || exit 1
    
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        log_error "STELLAR_SECRET_KEY is required"
        exit 1
    fi
    
    ADMIN_ADDRESS="${ADMIN_ADDRESS:-$STELLAR_PUBLIC_KEY}"
    
    # Validate parameters
    validate_upgrade_params
    
    # Safety confirmation for mainnet
    if [[ "$UPGRADE_NETWORK" == "mainnet" ]]; then
        echo ""
        echo -e "${RED}WARNING: You are about to upgrade a contract on MAINNET!${NC}"
        echo "Contract ID: $CONTRACT_ID"
        echo ""
        if ! confirm_action "Are you sure you want to continue?"; then
            exit 0
        fi
    elif [[ "$SKIP_CONFIRM" != "true" ]]; then
        if ! confirm_action "Ready to upgrade. Continue?"; then
            exit 0
        fi
    fi
    
    # Pre-upgrade checks
    pre_upgrade_checks
    
    # Build and install WASM
    build_and_install_wasm || exit 1
    
    # Execute upgrade
    execute_upgrade || exit 1
    
    # Post-upgrade verification
    post_upgrade_verification
    
    # Run migration if requested
    run_migration || exit 1
    
    # Final summary
    print_banner "Upgrade Complete"
    echo "Contract:    $UPGRADE_CONTRACT"
    echo "Contract ID: $CONTRACT_ID"
    echo "WASM Hash:   $WASM_HASH"
    echo "Network:     $UPGRADE_NETWORK"
    
    if [[ -n "$EXPLORER_URL" && "$EXPLORER_URL" != "null" ]]; then
        echo "Explorer:    $EXPLORER_URL/contract/$CONTRACT_ID"
    fi
    
    print_separator
    
    log_success "Upgrade complete!"
}

main "$@"
