#!/bin/bash
# ==============================================================================
# Grainlify Mainnet Deployment Script
# ==============================================================================
# Convenience script for deploying contracts to Stellar mainnet.
#
# IMPORTANT: This script deploys to mainnet where real funds are involved.
# Always test thoroughly on testnet first!
#
# Usage:
#   ./deploy_mainnet.sh [contract_name]
#
# Examples:
#   ./deploy_mainnet.sh                   # Deploy all contracts
#   ./deploy_mainnet.sh grainlify-core    # Deploy specific contract
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source utilities
source "$SCRIPT_DIR/utils.sh"

# Load environment
load_env

# ==============================================================================
# Mainnet-specific Configuration
# ==============================================================================

export NETWORK="mainnet"
export CONFIRM_DEPLOY="true"  # Always require confirmation for mainnet

# ==============================================================================
# Safety Checks
# ==============================================================================

print_banner "Grainlify MAINNET Deployment"

echo -e "${RED}╔══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${RED}║                    ⚠️  MAINNET DEPLOYMENT  ⚠️                     ║${NC}"
echo -e "${RED}║                                                                  ║${NC}"
echo -e "${RED}║  This script deploys to Stellar MAINNET where real XLM funds    ║${NC}"
echo -e "${RED}║  are required. Make sure you have:                               ║${NC}"
echo -e "${RED}║                                                                  ║${NC}"
echo -e "${RED}║  1. Tested thoroughly on testnet                                 ║${NC}"
echo -e "${RED}║  2. Audited the contract code                                    ║${NC}"
echo -e "${RED}║  3. Secured your admin keys properly                             ║${NC}"
echo -e "${RED}║  4. Sufficient XLM for deployment fees                           ║${NC}"
echo -e "${RED}║  5. A rollback plan in case of issues                            ║${NC}"
echo -e "${RED}║                                                                  ║${NC}"
echo -e "${RED}╚══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Require explicit mainnet confirmation
if [[ "${ALLOW_MAINNET_DEPLOY}" != "true" ]]; then
    echo -e "${YELLOW}To enable mainnet deployment, set ALLOW_MAINNET_DEPLOY=true${NC}"
    echo ""
    read -p "Type 'DEPLOY TO MAINNET' to continue: " -r
    
    if [[ "$REPLY" != "DEPLOY TO MAINNET" ]]; then
        log_error "Mainnet deployment cancelled"
        exit 1
    fi
fi

# Verify credentials are set
if [[ -z "$STELLAR_SECRET_KEY" ]]; then
    log_error "STELLAR_SECRET_KEY is required for mainnet deployment"
    log_error "Set it in scripts/config/.env or as an environment variable"
    exit 1
fi

# Verify account has funds
log_info "Verifying account has sufficient funds..."

if [[ -n "$STELLAR_PUBLIC_KEY" ]]; then
    # Use Horizon API to check balance
    BALANCE=$(curl -s "https://horizon.stellar.org/accounts/$STELLAR_PUBLIC_KEY" 2>/dev/null | \
        grep -o '"balance":"[0-9.]*"' | head -1 | grep -o '[0-9.]*' || echo "0")
    
    if [[ -z "$BALANCE" || "$BALANCE" == "0" ]]; then
        log_error "Account has no balance or is not funded"
        log_error "Please fund your account before deploying to mainnet"
        exit 1
    fi
    
    log_info "Account balance: $BALANCE XLM"
    
    # Require minimum balance for deployment
    MIN_BALANCE=100  # Adjust based on actual deployment costs
    if (( $(echo "$BALANCE < $MIN_BALANCE" | bc -l) )); then
        log_warning "Account balance ($BALANCE XLM) is below recommended minimum ($MIN_BALANCE XLM)"
        if ! confirm_action "Continue anyway?"; then
            exit 1
        fi
    fi
fi

# ==============================================================================
# Pre-deployment Checklist
# ==============================================================================

echo ""
echo "Pre-deployment Checklist:"
echo "========================"
echo ""

CHECKLIST_ITEMS=(
    "Tested on testnet successfully"
    "Contract code has been audited"
    "Admin keys are secured (hardware wallet/multisig)"
    "Have sufficient XLM for fees"
    "Verified deployment configuration"
    "Have a rollback plan"
)

for item in "${CHECKLIST_ITEMS[@]}"; do
    read -p "✓ $item [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_error "Please complete all checklist items before mainnet deployment"
        exit 1
    fi
done

echo ""
log_success "Checklist completed"

# ==============================================================================
# Backup existing deployments
# ==============================================================================

backup_deployment "mainnet" "*"
log_info "Backed up existing deployment records"

# ==============================================================================
# Run Deployment
# ==============================================================================

if [[ -n "$1" ]]; then
    "$SCRIPT_DIR/deploy.sh" -n mainnet -c "$1"
else
    "$SCRIPT_DIR/deploy.sh" -n mainnet
fi

# ==============================================================================
# Post-deployment
# ==============================================================================

echo ""
print_separator
log_success "Mainnet deployment complete!"
echo ""
echo "IMPORTANT: Save your deployment records and verify:"
echo "1. Contract functionality on mainnet"
echo "2. Admin access is working"
echo "3. All integrations updated with new contract IDs"
print_separator
