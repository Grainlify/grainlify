#!/bin/bash
# ==============================================================================
# Grainlify Contract Deployment Script
# ==============================================================================
# Deploys smart contracts to Stellar/Soroban network.
#
# Usage:
#   ./deploy.sh [options]
#
# Options:
#   -n, --network <network>     Network to deploy to (testnet, mainnet, local)
#   -c, --contract <contract>   Specific contract to deploy (all by default)
#   -d, --dry-run               Simulate deployment without making changes
#   -y, --yes                   Skip confirmation prompts
#   -v, --verbose               Enable verbose output
#   -h, --help                  Show this help message
#
# Examples:
#   ./deploy.sh -n testnet                    # Deploy all contracts to testnet
#   ./deploy.sh -n testnet -c grainlify-core  # Deploy only grainlify-core
#   ./deploy.sh -n mainnet -y                 # Deploy to mainnet without prompts
# ==============================================================================

set -e

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source utilities
source "$SCRIPT_DIR/utils.sh"

# ==============================================================================
# Default Configuration
# ==============================================================================

DEPLOY_NETWORK="${NETWORK:-testnet}"
DEPLOY_CONTRACT=""
SKIP_CONFIRM="false"

# ==============================================================================
# Help Function
# ==============================================================================

show_help() {
    cat << EOF
Grainlify Contract Deployment Script

Usage: $0 [options]

Options:
  -n, --network <network>     Network to deploy to (testnet, mainnet, local)
                              Default: testnet
  -c, --contract <contract>   Specific contract to deploy. Options:
                              - grainlify-core
                              - program-escrow
                              - bounty-escrow
                              Default: all contracts
  -d, --dry-run               Simulate deployment without making changes
  -y, --yes                   Skip confirmation prompts
  -v, --verbose               Enable verbose output
  -h, --help                  Show this help message

Environment Variables:
  STELLAR_SECRET_KEY          Source account secret key (required)
  STELLAR_PUBLIC_KEY          Source account public key
  ADMIN_ADDRESS               Admin address for contract initialization
  TOKEN_ADDRESS               Token contract address for escrow contracts
  
Examples:
  $0 -n testnet                    # Deploy all contracts to testnet
  $0 -n testnet -c grainlify-core  # Deploy only grainlify-core
  $0 -n mainnet -y                 # Deploy to mainnet without prompts
  $0 -d -n testnet                 # Dry run on testnet

EOF
}

# ==============================================================================
# Parse Arguments
# ==============================================================================

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--network)
                DEPLOY_NETWORK="$2"
                shift 2
                ;;
            -c|--contract)
                DEPLOY_CONTRACT="$2"
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

validate_environment() {
    log_info "Validating deployment environment..."
    
    # Check for required tools
    check_stellar_cli || exit 1
    
    # Check for jq (recommended but not required)
    if ! command -v jq &> /dev/null; then
        log_warning "jq not found. Some features may be limited. Install with: brew install jq"
    fi
    
    # Check for secret key
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        log_error "STELLAR_SECRET_KEY is required. Set it in environment or .env file"
        exit 1
    fi
    
    # Set public key if not provided - try to derive from secret key or use identity
    if [[ -z "$STELLAR_PUBLIC_KEY" ]]; then
        # If using an identity name, get the public key
        if [[ ! "$STELLAR_SECRET_KEY" =~ ^S[A-Z0-9]{55}$ ]]; then
            # It's an identity name, not a raw secret key
            STELLAR_PUBLIC_KEY=$(stellar keys public-key "$STELLAR_SECRET_KEY" 2>/dev/null || echo "")
        fi
        if [[ -z "$STELLAR_PUBLIC_KEY" ]]; then
            log_warning "Could not derive public key. Some checks may be skipped."
        fi
    fi
    
    # Set admin address to public key if not provided
    ADMIN_ADDRESS="${ADMIN_ADDRESS:-$STELLAR_PUBLIC_KEY}"
    
    # Validate network
    case "$DEPLOY_NETWORK" in
        testnet|mainnet|local)
            ;;
        *)
            log_error "Invalid network: $DEPLOY_NETWORK. Must be testnet, mainnet, or local"
            exit 1
            ;;
    esac
    
    log_success "Environment validation passed"
}

validate_contract() {
    local contract="$1"
    
    case "$contract" in
        grainlify-core|program-escrow|bounty-escrow)
            return 0
            ;;
        "")
            return 0
            ;;
        *)
            log_error "Invalid contract: $contract"
            return 1
            ;;
    esac
}

# ==============================================================================
# Deployment Functions
# ==============================================================================

deploy_single_contract() {
    local contract_name="$1"
    
    print_banner "Deploying $contract_name to $DEPLOY_NETWORK"
    
    # Get contract configuration
    local contract_path
    contract_path=$(get_contract_config "$contract_name" "path")
    
    if [[ -z "$contract_path" ]]; then
        log_error "Contract path not found for: $contract_name"
        return 1
    fi
    
    log_info "Contract path: $contract_path"
    
    # Step 1: Build contract
    log_info "Step 1: Building contract..."
    if ! build_contract "$contract_path" "true"; then
        log_error "Failed to build contract"
        return 1
    fi
    
    # Step 2: Get WASM path
    local wasm_path
    wasm_path=$(get_wasm_path "$contract_name" "true")
    
    if [[ ! -f "$wasm_path" && "$DRY_RUN" != "true" ]]; then
        log_error "WASM file not found: $wasm_path"
        return 1
    fi
    
    log_info "WASM path: $wasm_path"
    
    # Step 3: Install WASM
    log_info "Step 2: Installing WASM..."
    local wasm_hash
    wasm_hash=$(install_wasm "$wasm_path")
    
    if [[ -z "$wasm_hash" ]]; then
        log_error "Failed to install WASM"
        return 1
    fi
    
    # Step 4: Deploy contract
    log_info "Step 3: Deploying contract..."
    local contract_id
    contract_id=$(deploy_contract "$wasm_hash")
    
    if [[ -z "$contract_id" ]]; then
        log_error "Failed to deploy contract"
        return 1
    fi
    
    # Step 5: Initialize contract (if required)
    local requires_init
    requires_init=$(get_contract_config "$contract_name" "requires_init")
    
    if [[ "$requires_init" == "true" ]]; then
        log_info "Step 4: Initializing contract..."
        
        case "$contract_name" in
            grainlify-core)
                if ! invoke_contract "$contract_id" "init" "--admin" "$ADMIN_ADDRESS"; then
                    log_warning "Initialization may have failed or contract already initialized"
                fi
                ;;
            program-escrow|bounty-escrow)
                if ! invoke_contract "$contract_id" "init" "--admin" "$ADMIN_ADDRESS" "--token" "${TOKEN_ADDRESS:-CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC}"; then
                    log_warning "Initialization may have failed or contract already initialized"
                fi
                ;;
        esac
    fi
    
    # Step 6: Save deployment record
    log_info "Step 5: Saving deployment record..."
    save_deployment "$DEPLOY_NETWORK" "$contract_name" "$contract_id" "$wasm_hash" "1"
    
    # Print summary
    echo ""
    print_separator
    log_success "Contract deployed successfully!"
    echo ""
    echo "Contract: $contract_name"
    echo "Network:  $DEPLOY_NETWORK"
    echo "ID:       $contract_id"
    echo "WASM:     $wasm_hash"
    echo ""
    
    if [[ -n "$EXPLORER_URL" && "$EXPLORER_URL" != "null" ]]; then
        echo "Explorer: $EXPLORER_URL/contract/$contract_id"
    fi
    
    print_separator
    
    return 0
}

deploy_all_contracts() {
    local contracts=("grainlify-core" "program-escrow" "bounty-escrow")
    local failed=()
    local succeeded=()
    
    print_banner "Deploying all contracts to $DEPLOY_NETWORK"
    
    for contract in "${contracts[@]}"; do
        log_info "Processing contract: $contract"
        
        if deploy_single_contract "$contract"; then
            succeeded+=("$contract")
        else
            failed+=("$contract")
            log_warning "Continuing with remaining contracts..."
        fi
        
        echo ""
    done
    
    # Print final summary
    print_banner "Deployment Summary"
    
    if [[ ${#succeeded[@]} -gt 0 ]]; then
        echo -e "${GREEN}Succeeded:${NC}"
        for contract in "${succeeded[@]}"; do
            echo "  ✓ $contract"
        done
    fi
    
    if [[ ${#failed[@]} -gt 0 ]]; then
        echo -e "${RED}Failed:${NC}"
        for contract in "${failed[@]}"; do
            echo "  ✗ $contract"
        done
        return 1
    fi
    
    return 0
}

# ==============================================================================
# Pre-deployment Checks
# ==============================================================================

pre_deployment_checks() {
    log_info "Running pre-deployment checks..."
    
    # Check account funding
    if [[ -n "$STELLAR_PUBLIC_KEY" ]]; then
        if ! check_account "$STELLAR_PUBLIC_KEY"; then
            if [[ "$DEPLOY_NETWORK" == "testnet" || "$DEPLOY_NETWORK" == "local" ]]; then
                log_info "Attempting to fund account via Friendbot..."
                fund_account_testnet "$STELLAR_PUBLIC_KEY"
            else
                log_error "Account not funded and Friendbot not available on $DEPLOY_NETWORK"
                return 1
            fi
        fi
    fi
    
    log_success "Pre-deployment checks passed"
}

# ==============================================================================
# Main Function
# ==============================================================================

main() {
    # Parse command line arguments
    parse_args "$@"
    
    # Load environment
    load_env
    
    # Override with command line arguments
    NETWORK="$DEPLOY_NETWORK"
    
    # Setup network configuration
    setup_network "$DEPLOY_NETWORK"
    
    # Initialize deployments directory
    init_deployments_dir
    
    # Print header
    print_banner "Grainlify Contract Deployment"
    echo "Network:  $DEPLOY_NETWORK"
    echo "Contract: ${DEPLOY_CONTRACT:-all}"
    echo "Dry Run:  $DRY_RUN"
    echo ""
    
    # Validate environment
    validate_environment
    
    # Validate contract name if specified
    if [[ -n "$DEPLOY_CONTRACT" ]]; then
        validate_contract "$DEPLOY_CONTRACT" || exit 1
    fi
    
    # Confirm deployment
    if [[ "$DEPLOY_NETWORK" == "mainnet" ]]; then
        echo ""
        echo -e "${RED}WARNING: You are about to deploy to MAINNET!${NC}"
        echo "This will use real funds and cannot be undone."
        echo ""
        if ! confirm_action "Are you sure you want to continue?"; then
            exit 0
        fi
    elif [[ "$SKIP_CONFIRM" != "true" ]]; then
        if ! confirm_action "Ready to deploy. Continue?"; then
            exit 0
        fi
    fi
    
    # Run pre-deployment checks
    pre_deployment_checks
    
    # Deploy contract(s)
    if [[ -n "$DEPLOY_CONTRACT" ]]; then
        deploy_single_contract "$DEPLOY_CONTRACT"
    else
        deploy_all_contracts
    fi
    
    log_success "Deployment complete!"
}

# Run main function
main "$@"
