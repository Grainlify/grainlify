#!/bin/bash
# ==============================================================================
# Grainlify Testnet Deployment Script
# ==============================================================================
# Convenience script for deploying contracts to Stellar testnet.
#
# Usage:
#   ./deploy_testnet.sh [contract_name]
#
# Examples:
#   ./deploy_testnet.sh                   # Deploy all contracts
#   ./deploy_testnet.sh grainlify-core    # Deploy specific contract
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source utilities
source "$SCRIPT_DIR/utils.sh"

# Load environment
load_env

# ==============================================================================
# Testnet-specific Configuration
# ==============================================================================

export NETWORK="testnet"
export CONFIRM_DEPLOY="${CONFIRM_DEPLOY:-true}"

# ==============================================================================
# Main
# ==============================================================================

print_banner "Grainlify Testnet Deployment"

echo "Network: testnet"
echo "RPC URL: https://soroban-testnet.stellar.org:443"
echo ""

# Check if stellar CLI is available
check_stellar_cli || exit 1

# Generate testnet account if not configured
if [[ -z "$STELLAR_SECRET_KEY" ]]; then
    log_info "No STELLAR_SECRET_KEY configured. Generating new testnet account..."
    
    ACCOUNT_NAME="grainlify-testnet-deployer-$(date +%s)"
    
    log_info "Generating key pair..."
    stellar keys generate --global "$ACCOUNT_NAME" --network testnet
    
    STELLAR_PUBLIC_KEY=$(stellar keys public-key "$ACCOUNT_NAME")
    STELLAR_SECRET_KEY=$(stellar keys secret "$ACCOUNT_NAME")
    
    log_success "Generated account: $STELLAR_PUBLIC_KEY"
    
    log_info "Funding account via Friendbot..."
    stellar keys fund "$ACCOUNT_NAME" --network testnet || \
        curl -s "https://friendbot.stellar.org?addr=$STELLAR_PUBLIC_KEY" > /dev/null
    
    log_success "Account funded"
    
    echo ""
    echo "IMPORTANT: Save these credentials!"
    echo "Public Key:  $STELLAR_PUBLIC_KEY"
    echo "Secret Key:  $STELLAR_SECRET_KEY"
    echo "Account:     $ACCOUNT_NAME"
    echo ""
    echo "Add to scripts/config/.env:"
    echo "  STELLAR_PUBLIC_KEY=$STELLAR_PUBLIC_KEY"
    echo "  STELLAR_SECRET_KEY=$STELLAR_SECRET_KEY"
    echo ""
    
    export STELLAR_PUBLIC_KEY STELLAR_SECRET_KEY
fi

# Set admin address
export ADMIN_ADDRESS="${ADMIN_ADDRESS:-$STELLAR_PUBLIC_KEY}"

# Run deployment
if [[ -n "$1" ]]; then
    "$SCRIPT_DIR/deploy.sh" -n testnet -c "$1"
else
    "$SCRIPT_DIR/deploy.sh" -n testnet
fi
