#!/bin/bash
# ==============================================================================
# Grainlify Deployment Status Script
# ==============================================================================
# Shows the current deployment status across all networks.
#
# Usage:
#   ./status.sh [options]
#
# Options:
#   -n, --network <network>     Show status for specific network
#   -c, --contract <contract>   Show status for specific contract
#   -j, --json                  Output as JSON
#   -h, --help                  Show this help message
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

# ==============================================================================
# Configuration
# ==============================================================================

STATUS_NETWORK=""
STATUS_CONTRACT=""
JSON_OUTPUT="false"

# ==============================================================================
# Parse Arguments
# ==============================================================================

while [[ $# -gt 0 ]]; do
    case $1 in
        -n|--network)
            STATUS_NETWORK="$2"
            shift 2
            ;;
        -c|--contract)
            STATUS_CONTRACT="$2"
            shift 2
            ;;
        -j|--json)
            JSON_OUTPUT="true"
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [-n network] [-c contract] [-j] [-h]"
            echo ""
            echo "Options:"
            echo "  -n, --network <network>   Show status for specific network"
            echo "  -c, --contract <contract> Show status for specific contract"
            echo "  -j, --json                Output as JSON"
            echo "  -h, --help                Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# ==============================================================================
# Functions
# ==============================================================================

show_network_status() {
    local network="$1"
    local deployment_file
    deployment_file=$(get_deployment_file "$network")
    
    if [[ "$JSON_OUTPUT" != "true" ]]; then
        echo ""
        echo "Network: $network"
        echo "─────────────────────────────────────────────────────────────────"
    fi
    
    if [[ ! -f "$deployment_file" ]]; then
        if [[ "$JSON_OUTPUT" == "true" ]]; then
            echo "  \"$network\": {\"status\": \"no deployments\"}"
        else
            echo "  No deployments found"
        fi
        return
    fi
    
    if ! command -v jq &> /dev/null; then
        echo "  jq required for detailed status"
        return
    fi
    
    local contracts=("grainlify-core" "program-escrow" "bounty-escrow")
    
    for contract in "${contracts[@]}"; do
        if [[ -n "$STATUS_CONTRACT" && "$STATUS_CONTRACT" != "$contract" ]]; then
            continue
        fi
        
        local deployment
        deployment=$(jq -r ".deployments | map(select(.contract_name == \"$contract\")) | last" "$deployment_file" 2>/dev/null)
        
        if [[ -z "$deployment" || "$deployment" == "null" ]]; then
            if [[ "$JSON_OUTPUT" != "true" ]]; then
                printf "  %-18s │ Not Deployed\n" "$contract"
            fi
            continue
        fi
        
        local contract_id version deployed_at wasm_hash
        contract_id=$(echo "$deployment" | jq -r '.contract_id // "N/A"')
        version=$(echo "$deployment" | jq -r '.version // "N/A"')
        deployed_at=$(echo "$deployment" | jq -r '.deployed_at // "N/A"')
        wasm_hash=$(echo "$deployment" | jq -r '.wasm_hash // "N/A"')
        
        if [[ "$JSON_OUTPUT" == "true" ]]; then
            echo "    \"$contract\": {"
            echo "      \"contract_id\": \"$contract_id\","
            echo "      \"version\": $version,"
            echo "      \"wasm_hash\": \"${wasm_hash:0:16}...\","
            echo "      \"deployed_at\": \"$deployed_at\""
            echo "    },"
        else
            printf "  %-18s │ v%-6s │ %s │ %s\n" \
                "$contract" \
                "$version" \
                "${contract_id:0:15}..." \
                "${deployed_at:0:19}"
        fi
    done
}

# ==============================================================================
# Main
# ==============================================================================

load_env

if [[ "$JSON_OUTPUT" != "true" ]]; then
    print_banner "Grainlify Deployment Status"
    echo ""
    echo "Contract              │ Version │ Contract ID       │ Deployed At"
    echo "══════════════════════╪═════════╪═══════════════════╪════════════════════"
fi

if [[ -n "$STATUS_NETWORK" ]]; then
    show_network_status "$STATUS_NETWORK"
else
    for network in testnet mainnet local; do
        show_network_status "$network"
    done
fi

if [[ "$JSON_OUTPUT" != "true" ]]; then
    echo ""
    echo "─────────────────────────────────────────────────────────────────"
    echo "Use './verify.sh -n <network>' to verify contract status"
    echo ""
fi
