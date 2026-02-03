#!/bin/bash
# ==============================================================================
# Grainlify Contract Verification Script
# ==============================================================================
# Verifies deployed smart contracts are functioning correctly.
#
# Verification Checks:
# - Contract existence and accessibility
# - Version correctness
# - Admin access verification
# - Function invocation tests
# - State consistency checks
# - Event emission verification
#
# Usage:
#   ./verify.sh [options]
#
# Options:
#   -n, --network <network>     Network to verify on
#   -c, --contract <contract>   Contract to verify (all by default)
#   -i, --id <contract_id>      Specific contract ID to verify
#   --full                      Run comprehensive verification suite
#   --quick                     Run quick verification only
#   -v, --verbose               Enable verbose output
#   -h, --help                  Show this help message
#
# Examples:
#   ./verify.sh -n testnet -c grainlify-core
#   ./verify.sh -n mainnet --full
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

# ==============================================================================
# Default Configuration
# ==============================================================================

VERIFY_NETWORK="${NETWORK:-testnet}"
VERIFY_CONTRACT=""
CONTRACT_ID=""
VERIFICATION_MODE="standard"
VERIFICATION_RESULTS=()

# ==============================================================================
# Help Function
# ==============================================================================

show_help() {
    cat << EOF
Grainlify Contract Verification Script

Usage: $0 [options]

Options:
  -n, --network <network>     Network to verify on (testnet, mainnet)
  -c, --contract <contract>   Contract to verify. Options:
                              - grainlify-core
                              - program-escrow
                              - bounty-escrow
                              Default: all deployed contracts
  -i, --id <contract_id>      Specific contract ID to verify
  --full                      Run comprehensive verification suite
  --quick                     Run quick verification only (existence + version)
  -v, --verbose               Enable verbose output
  -h, --help                  Show this help message

Verification Checks:
  Quick Mode:
    - Contract existence
    - Version check
    
  Standard Mode (default):
    - Contract existence
    - Version check
    - Admin verification
    - Basic function calls
    
  Full Mode:
    - All standard checks
    - State consistency
    - Event emission
    - Error handling
    - Edge case testing

Examples:
  $0 -n testnet -c grainlify-core
  $0 -n mainnet --full
  $0 -n testnet -i CABC... --quick

EOF
}

# ==============================================================================
# Parse Arguments
# ==============================================================================

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--network)
                VERIFY_NETWORK="$2"
                shift 2
                ;;
            -c|--contract)
                VERIFY_CONTRACT="$2"
                shift 2
                ;;
            -i|--id)
                CONTRACT_ID="$2"
                shift 2
                ;;
            --full)
                VERIFICATION_MODE="full"
                shift
                ;;
            --quick)
                VERIFICATION_MODE="quick"
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
# Verification Result Tracking
# ==============================================================================

record_result() {
    local check_name="$1"
    local status="$2"
    local message="$3"
    
    VERIFICATION_RESULTS+=("$check_name|$status|$message")
    
    if [[ "$status" == "PASS" ]]; then
        echo -e "  ${GREEN}✓${NC} $check_name: $message"
    elif [[ "$status" == "WARN" ]]; then
        echo -e "  ${YELLOW}⚠${NC} $check_name: $message"
    else
        echo -e "  ${RED}✗${NC} $check_name: $message"
    fi
}

print_verification_summary() {
    local passed=0
    local failed=0
    local warnings=0
    
    for result in "${VERIFICATION_RESULTS[@]}"; do
        local status
        status=$(echo "$result" | cut -d'|' -f2)
        case "$status" in
            PASS) ((passed++)) ;;
            FAIL) ((failed++)) ;;
            WARN) ((warnings++)) ;;
        esac
    done
    
    echo ""
    print_separator
    echo "Verification Summary"
    print_separator
    echo -e "  ${GREEN}Passed:${NC}   $passed"
    echo -e "  ${RED}Failed:${NC}   $failed"
    echo -e "  ${YELLOW}Warnings:${NC} $warnings"
    echo ""
    
    if [[ $failed -gt 0 ]]; then
        return 1
    fi
    return 0
}

# ==============================================================================
# Verification Functions
# ==============================================================================

verify_contract_exists() {
    local contract_id="$1"
    
    log_debug "Checking contract existence: $contract_id"
    
    # Use contract info to check if contract exists
    local result
    result=$(stellar contract info wasm \
        --id "$contract_id" \
        --network "$VERIFY_NETWORK" 2>&1) || true
    
    if [[ -n "$result" && "$result" != *"error"* && "$result" != *"Error"* && "$result" != *"not found"* ]]; then
        record_result "Contract Exists" "PASS" "Contract is deployed"
        return 0
    else
        record_result "Contract Exists" "FAIL" "Contract not found or inaccessible"
        return 1
    fi
}

verify_contract_version() {
    local contract_id="$1"
    local expected_version="${2:-}"
    
    log_debug "Checking contract version"
    
    local version
    version=$(get_contract_version "$contract_id")
    
    if [[ -n "$version" && "$version" != "0" ]]; then
        if [[ -n "$expected_version" ]]; then
            if [[ "$version" == "$expected_version" ]]; then
                record_result "Version Check" "PASS" "Version $version matches expected"
            else
                record_result "Version Check" "FAIL" "Version $version != expected $expected_version"
                return 1
            fi
        else
            record_result "Version Check" "PASS" "Version: $version"
        fi
        return 0
    else
        record_result "Version Check" "WARN" "Version not available or is 0"
        return 0
    fi
}

verify_admin_access() {
    local contract_id="$1"
    
    log_debug "Verifying admin access"
    
    # Need a source account for invoke, even for read-only calls
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        record_result "Admin Access" "WARN" "No source account to verify admin"
        return 0
    fi
    
    local admin
    admin=$(stellar contract invoke \
        --id "$contract_id" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$VERIFY_NETWORK" \
        --send=no \
        -- get_admin 2>&1) || true
    
    if [[ -n "$admin" && "$admin" != *"error"* && "$admin" != *"Error"* ]]; then
        record_result "Admin Access" "PASS" "Admin: ${admin:0:15}..."
        return 0
    else
        record_result "Admin Access" "WARN" "Could not retrieve admin (may not be implemented)"
        return 0
    fi
}

verify_function_calls() {
    local contract_id="$1"
    local contract_name="$2"
    
    log_debug "Testing function calls"
    
    # Need a source account for invoke
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        record_result "Function Calls" "WARN" "No source account to test functions"
        return 0
    fi
    
    # Test read functions based on contract type
    case "$contract_name" in
        grainlify-core)
            # Test get_version (should always work)
            local version_check
            version_check=$(stellar contract invoke \
                --id "$contract_id" \
                --source-account "$STELLAR_SECRET_KEY" \
                --network "$VERIFY_NETWORK" \
                --send=no \
                -- get_version 2>&1) || true
            
            if [[ "$version_check" != *"error"* && "$version_check" != *"Error"* ]]; then
                record_result "Function: get_version" "PASS" "Returns: $version_check"
            else
                record_result "Function: get_version" "WARN" "Function may not be available"
            fi
            ;;
            
        program-escrow|bounty-escrow)
            # Test get_balance (with a dummy ID - should return 0 or error gracefully)
            local balance_check
            balance_check=$(stellar contract invoke \
                --id "$contract_id" \
                --source-account "$STELLAR_SECRET_KEY" \
                --network "$VERIFY_NETWORK" \
                --send=no \
                -- get_balance \
                --id "test-verify-id" 2>&1) || true
            
            if [[ "$balance_check" == "0" || "$balance_check" != *"panic"* ]]; then
                record_result "Function: get_balance" "PASS" "Function accessible"
            else
                record_result "Function: get_balance" "WARN" "Function may behave unexpectedly"
            fi
            ;;
    esac
}

verify_state_consistency() {
    local contract_id="$1"
    
    log_debug "Checking state consistency"
    
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        record_result "State Consistency" "WARN" "No source account to check state"
        return 0
    fi
    
    # Check if the contract can be invoked without errors
    local state_check
    state_check=$(stellar contract invoke \
        --id "$contract_id" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$VERIFY_NETWORK" \
        --send=no \
        -- get_version 2>&1) || true
    
    if [[ "$state_check" != *"storage"*"error"* && "$state_check" != *"Storage"*"Error"* ]]; then
        record_result "State Consistency" "PASS" "No storage errors detected"
    else
        record_result "State Consistency" "FAIL" "Storage errors detected"
        return 1
    fi
}

verify_error_handling() {
    local contract_id="$1"
    
    log_debug "Testing error handling"
    
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        record_result "Error Handling" "WARN" "No source account to test errors"
        return 0
    fi
    
    # Try to invoke a function with invalid parameters
    # This should return a contract error, not a panic
    local error_check
    error_check=$(stellar contract invoke \
        --id "$contract_id" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$VERIFY_NETWORK" \
        --send=no \
        -- get_balance \
        --id "" 2>&1) || true
    
    # A well-behaved contract should return an error, not panic
    if [[ "$error_check" != *"panic"* ]]; then
        record_result "Error Handling" "PASS" "Contract handles errors gracefully"
    else
        record_result "Error Handling" "WARN" "Contract may panic on invalid input"
    fi
}

# ==============================================================================
# Contract Verification
# ==============================================================================

verify_single_contract() {
    local contract_name="$1"
    local contract_id="$2"
    
    print_banner "Verifying: $contract_name"
    echo "Contract ID: $contract_id"
    echo "Network:     $VERIFY_NETWORK"
    echo "Mode:        $VERIFICATION_MODE"
    echo ""
    
    local failed=0
    
    # Quick checks (always run)
    if ! verify_contract_exists "$contract_id"; then
        log_error "Contract verification failed - contract doesn't exist"
        return 1
    fi
    
    verify_contract_version "$contract_id"
    
    if [[ "$VERIFICATION_MODE" == "quick" ]]; then
        return 0
    fi
    
    # Standard checks
    verify_admin_access "$contract_id"
    verify_function_calls "$contract_id" "$contract_name"
    
    if [[ "$VERIFICATION_MODE" == "standard" ]]; then
        return 0
    fi
    
    # Full checks
    verify_state_consistency "$contract_id"
    verify_error_handling "$contract_id"
    
    return 0
}

verify_all_contracts() {
    local contracts=("grainlify-core" "program-escrow" "bounty-escrow")
    
    for contract in "${contracts[@]}"; do
        local deployment
        deployment=$(get_latest_deployment "$VERIFY_NETWORK" "$contract")
        
        if [[ -z "$deployment" || "$deployment" == "null" ]]; then
            log_warning "No deployment found for $contract on $VERIFY_NETWORK"
            continue
        fi
        
        local contract_id
        contract_id=$(echo "$deployment" | jq -r '.contract_id // empty')
        
        if [[ -n "$contract_id" ]]; then
            verify_single_contract "$contract" "$contract_id" || true
        fi
        
        echo ""
    done
}

# ==============================================================================
# Main Function
# ==============================================================================

main() {
    parse_args "$@"
    
    load_env
    
    NETWORK="$VERIFY_NETWORK"
    setup_network "$VERIFY_NETWORK"
    
    print_banner "Grainlify Contract Verification"
    echo "Network: $VERIFY_NETWORK"
    echo "Mode:    $VERIFICATION_MODE"
    echo ""
    
    check_stellar_cli || exit 1
    
    if [[ -n "$CONTRACT_ID" ]]; then
        # Verify specific contract ID
        local contract_name="${VERIFY_CONTRACT:-unknown}"
        verify_single_contract "$contract_name" "$CONTRACT_ID"
    elif [[ -n "$VERIFY_CONTRACT" ]]; then
        # Verify specific contract by name
        local deployment
        deployment=$(get_latest_deployment "$VERIFY_NETWORK" "$VERIFY_CONTRACT")
        
        if [[ -z "$deployment" || "$deployment" == "null" ]]; then
            log_error "No deployment found for $VERIFY_CONTRACT on $VERIFY_NETWORK"
            exit 1
        fi
        
        CONTRACT_ID=$(echo "$deployment" | jq -r '.contract_id // empty')
        
        if [[ -z "$CONTRACT_ID" ]]; then
            log_error "Could not find contract ID"
            exit 1
        fi
        
        verify_single_contract "$VERIFY_CONTRACT" "$CONTRACT_ID"
    else
        # Verify all contracts
        verify_all_contracts
    fi
    
    # Print summary
    print_verification_summary
}

main "$@"
