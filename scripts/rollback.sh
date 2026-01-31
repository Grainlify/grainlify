#!/bin/bash
# ==============================================================================
# Grainlify Contract Rollback Script
# ==============================================================================
# Rolls back deployed contracts to a previous version.
#
# Rollback Scenarios:
# - Failed upgrade: Restore previous WASM
# - Failed migration: Restore previous state
# - Emergency: Quick rollback to known good version
#
# Usage:
#   ./rollback.sh [options]
#
# Options:
#   -n, --network <network>     Network to rollback on
#   -c, --contract <contract>   Contract to rollback
#   -i, --id <contract_id>      Contract ID to rollback
#   -w, --wasm <wasm_hash>      Specific WASM hash to rollback to
#   -v, --version <version>     Specific version to rollback to
#   --list                      List available rollback points
#   --emergency                 Skip all confirmations (dangerous!)
#   -d, --dry-run               Simulate rollback
#   -h, --help                  Show this help message
#
# Examples:
#   ./rollback.sh -n testnet -c grainlify-core --list
#   ./rollback.sh -n testnet -c grainlify-core -w abc123...
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

# ==============================================================================
# Default Configuration
# ==============================================================================

ROLLBACK_NETWORK="${NETWORK:-testnet}"
ROLLBACK_CONTRACT=""
CONTRACT_ID=""
ROLLBACK_WASM=""
ROLLBACK_VERSION=""
LIST_ONLY="false"
EMERGENCY_MODE="false"
SKIP_CONFIRM="false"

# ==============================================================================
# Help Function
# ==============================================================================

show_help() {
    cat << EOF
Grainlify Contract Rollback Script

Usage: $0 [options]

Options:
  -n, --network <network>     Network to rollback on (testnet, mainnet)
  -c, --contract <contract>   Contract to rollback (required)
  -i, --id <contract_id>      Contract ID to rollback
  -w, --wasm <wasm_hash>      Specific WASM hash to rollback to
  -v, --version <version>     Specific version to rollback to
  --list                      List available rollback points
  --emergency                 Skip all confirmations (DANGEROUS!)
  -d, --dry-run               Simulate rollback without changes
  -y, --yes                   Skip confirmation prompts
  -h, --help                  Show this help message

Rollback Points:
  The script can rollback to:
  - A specific WASM hash (from deployment history)
  - A specific version (from deployment history)
  - The previous deployment (default if neither specified)

Safety Features:
  - Automatic pre-rollback backup
  - Post-rollback verification
  - Rollback audit logging

Examples:
  $0 -n testnet -c grainlify-core --list
  $0 -n testnet -c grainlify-core  # Rollback to previous version
  $0 -n mainnet -c program-escrow -w abc123...

EOF
}

# ==============================================================================
# Parse Arguments
# ==============================================================================

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--network)
                ROLLBACK_NETWORK="$2"
                shift 2
                ;;
            -c|--contract)
                ROLLBACK_CONTRACT="$2"
                shift 2
                ;;
            -i|--id)
                CONTRACT_ID="$2"
                shift 2
                ;;
            -w|--wasm)
                ROLLBACK_WASM="$2"
                shift 2
                ;;
            -v|--version)
                ROLLBACK_VERSION="$2"
                shift 2
                ;;
            --list)
                LIST_ONLY="true"
                shift
                ;;
            --emergency)
                EMERGENCY_MODE="true"
                SKIP_CONFIRM="true"
                CONFIRM_DEPLOY="false"
                shift
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
# Rollback History Functions
# ==============================================================================

list_rollback_points() {
    print_banner "Available Rollback Points"
    
    echo "Contract: $ROLLBACK_CONTRACT"
    echo "Network:  $ROLLBACK_NETWORK"
    echo ""
    
    local deployment_file
    deployment_file=$(get_deployment_file "$ROLLBACK_NETWORK")
    
    if [[ ! -f "$deployment_file" ]]; then
        log_warning "No deployment history found"
        return 1
    fi
    
    if ! command -v jq &> /dev/null; then
        log_error "jq is required to list rollback points"
        return 1
    fi
    
    echo "Deployment History:"
    echo "-------------------"
    
    local deployments
    deployments=$(jq -r ".deployments | map(select(.contract_name == \"$ROLLBACK_CONTRACT\")) | reverse" "$deployment_file")
    
    local count
    count=$(echo "$deployments" | jq 'length')
    
    if [[ "$count" == "0" ]]; then
        log_warning "No deployments found for $ROLLBACK_CONTRACT"
        return 1
    fi
    
    echo "$deployments" | jq -r '.[] | "  Version: \(.version // "N/A") | WASM: \(.wasm_hash[0:16])... | Date: \(.deployed_at)"'
    
    echo ""
    echo "Backup History:"
    echo "---------------"
    
    local backup_dir="$DEPLOYMENTS_DIR/backups"
    if [[ -d "$backup_dir" ]]; then
        ls -la "$backup_dir" | grep "$ROLLBACK_CONTRACT" | head -10 || echo "  No backups found"
    else
        echo "  No backups directory"
    fi
    
    echo ""
}

get_previous_deployment() {
    local deployment_file
    deployment_file=$(get_deployment_file "$ROLLBACK_NETWORK")
    
    if [[ ! -f "$deployment_file" ]]; then
        return 1
    fi
    
    # Get the second-to-last deployment for this contract
    local deployments
    deployments=$(jq -r ".deployments | map(select(.contract_name == \"$ROLLBACK_CONTRACT\"))" "$deployment_file")
    
    local count
    count=$(echo "$deployments" | jq 'length')
    
    if [[ "$count" -lt 2 ]]; then
        log_warning "No previous deployment found to rollback to"
        return 1
    fi
    
    # Get second to last deployment
    echo "$deployments" | jq ".[$((count - 2))]"
}

# ==============================================================================
# Rollback Functions
# ==============================================================================

pre_rollback_backup() {
    print_banner "Pre-Rollback Backup"
    
    log_info "Creating pre-rollback backup..."
    
    local backup_dir="$DEPLOYMENTS_DIR/backups/rollbacks"
    mkdir -p "$backup_dir"
    
    local backup_file="$backup_dir/${ROLLBACK_CONTRACT}_pre_rollback_$(date +%Y%m%d_%H%M%S).json"
    
    # Get current state
    local current_version
    current_version=$(get_contract_version "$CONTRACT_ID")
    
    local backup_data="{"
    backup_data+="\"contract_id\": \"$CONTRACT_ID\","
    backup_data+="\"contract_name\": \"$ROLLBACK_CONTRACT\","
    backup_data+="\"network\": \"$ROLLBACK_NETWORK\","
    backup_data+="\"current_version\": $current_version,"
    backup_data+="\"rollback_target_wasm\": \"$ROLLBACK_WASM\","
    backup_data+="\"rollback_target_version\": \"${ROLLBACK_VERSION:-unknown}\","
    backup_data+="\"timestamp\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\","
    backup_data+="\"status\": \"pending\""
    backup_data+="}"
    
    echo "$backup_data" > "$backup_file"
    
    ROLLBACK_BACKUP_FILE="$backup_file"
    
    log_success "Pre-rollback backup created: $backup_file"
}

execute_rollback() {
    print_banner "Executing Rollback"
    
    if [[ -z "$ROLLBACK_WASM" ]]; then
        log_error "No WASM hash specified for rollback"
        return 1
    fi
    
    log_info "Rolling back contract $CONTRACT_ID to WASM $ROLLBACK_WASM"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would rollback contract"
        return 0
    fi
    
    # Execute the rollback (which is essentially an upgrade to an older WASM)
    local result
    result=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$ROLLBACK_NETWORK" \
        -- upgrade \
        --new_wasm_hash "$ROLLBACK_WASM" 2>&1)
    
    if [[ $? -eq 0 ]]; then
        log_success "Rollback transaction submitted"
        echo "$result"
        return 0
    else
        log_error "Rollback failed: $result"
        return 1
    fi
}

verify_rollback() {
    print_banner "Verifying Rollback"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would verify rollback"
        return 0
    fi
    
    log_info "Waiting for rollback confirmation..."
    sleep 5
    
    # Verify contract is accessible
    log_info "Verifying contract accessibility..."
    
    local version
    version=$(get_contract_version "$CONTRACT_ID")
    
    log_info "Post-rollback version: $version"
    
    if [[ -n "$ROLLBACK_VERSION" && "$version" != "$ROLLBACK_VERSION" ]]; then
        log_warning "Version mismatch: expected $ROLLBACK_VERSION, got $version"
    fi
    
    # Run quick verification
    "$SCRIPT_DIR/verify.sh" -n "$ROLLBACK_NETWORK" -i "$CONTRACT_ID" --quick || {
        log_warning "Quick verification had issues"
    }
    
    # Update backup status
    if [[ -n "$ROLLBACK_BACKUP_FILE" && -f "$ROLLBACK_BACKUP_FILE" ]]; then
        local tmp_file="$ROLLBACK_BACKUP_FILE.tmp"
        if command -v jq &> /dev/null; then
            jq '.status = "completed"' "$ROLLBACK_BACKUP_FILE" > "$tmp_file"
            mv "$tmp_file" "$ROLLBACK_BACKUP_FILE"
        fi
    fi
    
    log_success "Rollback verification complete"
}

# ==============================================================================
# Main Function
# ==============================================================================

main() {
    parse_args "$@"
    
    load_env
    
    NETWORK="$ROLLBACK_NETWORK"
    setup_network "$ROLLBACK_NETWORK"
    init_deployments_dir
    
    # Contract is required
    if [[ -z "$ROLLBACK_CONTRACT" ]]; then
        log_error "Contract name is required. Use -c <contract_name>"
        show_help
        exit 1
    fi
    
    # List only mode
    if [[ "$LIST_ONLY" == "true" ]]; then
        list_rollback_points
        exit 0
    fi
    
    print_banner "Grainlify Contract Rollback"
    echo "Network:  $ROLLBACK_NETWORK"
    echo "Contract: $ROLLBACK_CONTRACT"
    echo "Emergency: $EMERGENCY_MODE"
    echo ""
    
    check_stellar_cli || exit 1
    
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        log_error "STELLAR_SECRET_KEY is required"
        exit 1
    fi
    
    # Get contract ID if not provided
    if [[ -z "$CONTRACT_ID" ]]; then
        local deployment
        deployment=$(get_latest_deployment "$ROLLBACK_NETWORK" "$ROLLBACK_CONTRACT")
        
        if [[ -n "$deployment" && "$deployment" != "null" ]]; then
            CONTRACT_ID=$(echo "$deployment" | jq -r '.contract_id // empty')
        fi
        
        if [[ -z "$CONTRACT_ID" ]]; then
            log_error "Contract ID not found. Provide it with -i <contract_id>"
            exit 1
        fi
    fi
    
    log_info "Contract ID: $CONTRACT_ID"
    
    # Determine rollback target
    if [[ -z "$ROLLBACK_WASM" && -z "$ROLLBACK_VERSION" ]]; then
        log_info "No specific rollback target. Finding previous deployment..."
        
        local previous
        previous=$(get_previous_deployment)
        
        if [[ -z "$previous" || "$previous" == "null" ]]; then
            log_error "Could not find previous deployment to rollback to"
            log_info "Use --list to see available rollback points"
            exit 1
        fi
        
        ROLLBACK_WASM=$(echo "$previous" | jq -r '.wasm_hash // empty')
        ROLLBACK_VERSION=$(echo "$previous" | jq -r '.version // empty')
        
        log_info "Rollback target: version $ROLLBACK_VERSION (WASM: ${ROLLBACK_WASM:0:16}...)"
    fi
    
    if [[ -z "$ROLLBACK_WASM" ]]; then
        log_error "No WASM hash available for rollback"
        exit 1
    fi
    
    # Safety confirmations
    if [[ "$EMERGENCY_MODE" == "true" ]]; then
        echo -e "${RED}╔══════════════════════════════════════════════════════════════════╗${NC}"
        echo -e "${RED}║                    ⚠️  EMERGENCY ROLLBACK  ⚠️                      ║${NC}"
        echo -e "${RED}║                                                                  ║${NC}"
        echo -e "${RED}║  Emergency mode enabled - skipping all confirmations!           ║${NC}"
        echo -e "${RED}╚══════════════════════════════════════════════════════════════════╝${NC}"
        echo ""
    elif [[ "$ROLLBACK_NETWORK" == "mainnet" ]]; then
        echo -e "${RED}WARNING: You are about to rollback a contract on MAINNET!${NC}"
        echo "Contract ID: $CONTRACT_ID"
        echo "Rollback to WASM: $ROLLBACK_WASM"
        echo ""
        if ! confirm_action "Are you sure you want to continue?"; then
            exit 0
        fi
    elif [[ "$SKIP_CONFIRM" != "true" ]]; then
        echo "Rollback Details:"
        echo "  Contract:    $ROLLBACK_CONTRACT"
        echo "  Contract ID: $CONTRACT_ID"
        echo "  Target WASM: ${ROLLBACK_WASM:0:32}..."
        echo "  Target Ver:  ${ROLLBACK_VERSION:-unknown}"
        echo ""
        if ! confirm_action "Ready to rollback. Continue?"; then
            exit 0
        fi
    fi
    
    # Execute rollback process
    pre_rollback_backup
    
    execute_rollback || {
        log_error "Rollback failed!"
        log_info "Check backup file: $ROLLBACK_BACKUP_FILE"
        exit 1
    }
    
    verify_rollback
    
    # Log rollback action
    log_info "Recording rollback in deployment history..."
    save_deployment "$ROLLBACK_NETWORK" "$ROLLBACK_CONTRACT" "$CONTRACT_ID" "$ROLLBACK_WASM" "${ROLLBACK_VERSION:-0}"
    
    # Final summary
    print_banner "Rollback Complete"
    echo "Contract:     $ROLLBACK_CONTRACT"
    echo "Contract ID:  $CONTRACT_ID"
    echo "Network:      $ROLLBACK_NETWORK"
    echo "Rolled to:    ${ROLLBACK_VERSION:-N/A} (WASM: ${ROLLBACK_WASM:0:16}...)"
    print_separator
    
    log_success "Rollback complete!"
    
    if [[ "$ROLLBACK_NETWORK" == "mainnet" ]]; then
        echo ""
        echo -e "${YELLOW}IMPORTANT: Verify contract functionality manually!${NC}"
        echo "1. Test critical functions"
        echo "2. Verify state integrity"
        echo "3. Update any dependent systems"
    fi
}

main "$@"
