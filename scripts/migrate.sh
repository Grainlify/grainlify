#!/bin/bash
# ==============================================================================
# Grainlify Contract Migration Script
# ==============================================================================
# Handles data structure migrations between contract versions.
#
# Migration Types:
# - State migration: Update storage schema between versions
# - Data migration: Transform existing data to new format
# - Event migration: Update event schemas
#
# Usage:
#   ./migrate.sh [options]
#
# Options:
#   -n, --network <network>       Network to migrate on
#   -c, --contract <contract>     Contract to migrate
#   -i, --id <contract_id>        Contract ID to migrate
#   -f, --from-version <v>        Source version (auto-detected if not provided)
#   -t, --to-version <v>          Target version (required)
#   -s, --strategy <strategy>     Migration strategy (batch, incremental, full)
#   --batch-size <size>           Batch size for batch migration (default: 100)
#   -d, --dry-run                 Simulate migration
#   -v, --verbose                 Enable verbose output
#   -h, --help                    Show this help message
#
# Examples:
#   ./migrate.sh -n testnet -c grainlify-core -t 20000
#   ./migrate.sh -n mainnet -c program-escrow -f 10000 -t 20000 -s batch
# ==============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/utils.sh"

# ==============================================================================
# Default Configuration
# ==============================================================================

MIGRATE_NETWORK="${NETWORK:-testnet}"
MIGRATE_CONTRACT=""
CONTRACT_ID=""
FROM_VERSION=""
TO_VERSION=""
MIGRATION_STRATEGY="full"
BATCH_SIZE=100
SKIP_CONFIRM="false"

# ==============================================================================
# Help Function
# ==============================================================================

show_help() {
    cat << EOF
Grainlify Contract Migration Script

Usage: $0 [options]

Options:
  -n, --network <network>       Network to migrate on (testnet, mainnet)
  -c, --contract <contract>     Contract to migrate (required)
  -i, --id <contract_id>        Contract ID to migrate
  -f, --from-version <v>        Source version (auto-detected if not provided)
  -t, --to-version <v>          Target version (required)
  -s, --strategy <strategy>     Migration strategy:
                                - full: All-at-once migration
                                - batch: Process in batches
                                - incremental: Step-by-step version increments
  --batch-size <size>           Batch size for batch migration (default: 100)
  -d, --dry-run                 Simulate migration without changes
  -y, --yes                     Skip confirmation prompts
  -v, --verbose                 Enable verbose output
  -h, --help                    Show this help message

Migration Functions:
  The contract must implement these functions:
  - get_version() -> u32
  - migrate(target_version: u32, migration_hash: BytesN<32>) -> Result
  - get_migration_state() -> MigrationState (optional)

Examples:
  $0 -n testnet -c grainlify-core -t 20000
  $0 -n mainnet -c program-escrow -f 10000 -t 20000 -s batch

EOF
}

# ==============================================================================
# Parse Arguments
# ==============================================================================

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -n|--network)
                MIGRATE_NETWORK="$2"
                shift 2
                ;;
            -c|--contract)
                MIGRATE_CONTRACT="$2"
                shift 2
                ;;
            -i|--id)
                CONTRACT_ID="$2"
                shift 2
                ;;
            -f|--from-version)
                FROM_VERSION="$2"
                shift 2
                ;;
            -t|--to-version)
                TO_VERSION="$2"
                shift 2
                ;;
            -s|--strategy)
                MIGRATION_STRATEGY="$2"
                shift 2
                ;;
            --batch-size)
                BATCH_SIZE="$2"
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
# Validation
# ==============================================================================

validate_migration_params() {
    log_info "Validating migration parameters..."
    
    if [[ -z "$MIGRATE_CONTRACT" ]]; then
        log_error "Contract name is required. Use -c <contract_name>"
        exit 1
    fi
    
    if [[ -z "$TO_VERSION" ]]; then
        log_error "Target version is required. Use -t <version>"
        exit 1
    fi
    
    # Validate version is numeric
    if ! [[ "$TO_VERSION" =~ ^[0-9]+$ ]]; then
        log_error "Target version must be numeric (e.g., 20000 for v2.0.0)"
        exit 1
    fi
    
    # Validate strategy
    case "$MIGRATION_STRATEGY" in
        full|batch|incremental)
            ;;
        *)
            log_error "Invalid strategy: $MIGRATION_STRATEGY. Must be: full, batch, incremental"
            exit 1
            ;;
    esac
    
    # Get contract ID if not provided
    if [[ -z "$CONTRACT_ID" ]]; then
        local deployment
        deployment=$(get_latest_deployment "$MIGRATE_NETWORK" "$MIGRATE_CONTRACT")
        
        if [[ -n "$deployment" && "$deployment" != "null" ]]; then
            CONTRACT_ID=$(echo "$deployment" | jq -r '.contract_id // empty')
        fi
        
        if [[ -z "$CONTRACT_ID" ]]; then
            log_error "Contract ID not found. Provide it with -i <contract_id>"
            exit 1
        fi
    fi
    
    # Get current version if not provided
    if [[ -z "$FROM_VERSION" ]]; then
        FROM_VERSION=$(get_contract_version "$CONTRACT_ID")
        log_info "Detected current version: $FROM_VERSION"
    fi
    
    # Validate version progression
    if [[ "$FROM_VERSION" -ge "$TO_VERSION" ]]; then
        log_error "Target version ($TO_VERSION) must be greater than current version ($FROM_VERSION)"
        exit 1
    fi
    
    log_success "Migration parameters validated"
}

# ==============================================================================
# Migration Planning
# ==============================================================================

generate_migration_plan() {
    print_banner "Migration Plan"
    
    echo "Contract:        $MIGRATE_CONTRACT"
    echo "Contract ID:     $CONTRACT_ID"
    echo "Network:         $MIGRATE_NETWORK"
    echo "From Version:    $FROM_VERSION"
    echo "To Version:      $TO_VERSION"
    echo "Strategy:        $MIGRATION_STRATEGY"
    echo ""
    
    # Determine migration path
    local migration_steps=()
    
    case "$MIGRATION_STRATEGY" in
        full)
            migration_steps+=("$FROM_VERSION -> $TO_VERSION (direct)")
            ;;
        incremental)
            # Generate step-by-step migration path
            local current=$FROM_VERSION
            local major_from=$((FROM_VERSION / 10000))
            local major_to=$((TO_VERSION / 10000))
            
            for ((major = major_from; major < major_to; major++)); do
                local next=$(((major + 1) * 10000))
                migration_steps+=("$current -> $next")
                current=$next
            done
            
            if [[ $current -lt $TO_VERSION ]]; then
                migration_steps+=("$current -> $TO_VERSION")
            fi
            ;;
        batch)
            migration_steps+=("$FROM_VERSION -> $TO_VERSION (batch, size=$BATCH_SIZE)")
            ;;
    esac
    
    echo "Migration Steps:"
    for step in "${migration_steps[@]}"; do
        echo "  • $step"
    done
    echo ""
    
    # Calculate estimated time
    local estimated_time="< 1 minute"
    if [[ "$MIGRATION_STRATEGY" == "batch" ]]; then
        estimated_time="Depends on data volume"
    elif [[ ${#migration_steps[@]} -gt 1 ]]; then
        estimated_time="${#migration_steps[@]}-$((${#migration_steps[@]} * 2)) minutes"
    fi
    
    echo "Estimated Time: $estimated_time"
    echo ""
    
    # Export for use in migration
    MIGRATION_STEPS=("${migration_steps[@]}")
}

# ==============================================================================
# Pre-Migration
# ==============================================================================

pre_migration_backup() {
    print_banner "Pre-Migration Backup"
    
    log_info "Creating pre-migration backup..."
    
    local backup_dir="$DEPLOYMENTS_DIR/backups/migrations"
    mkdir -p "$backup_dir"
    
    local backup_file="$backup_dir/${MIGRATE_CONTRACT}_${FROM_VERSION}_to_${TO_VERSION}_$(date +%Y%m%d_%H%M%S).json"
    
    local backup_data="{"
    backup_data+="\"contract_id\": \"$CONTRACT_ID\","
    backup_data+="\"contract_name\": \"$MIGRATE_CONTRACT\","
    backup_data+="\"network\": \"$MIGRATE_NETWORK\","
    backup_data+="\"from_version\": $FROM_VERSION,"
    backup_data+="\"to_version\": $TO_VERSION,"
    backup_data+="\"strategy\": \"$MIGRATION_STRATEGY\","
    backup_data+="\"timestamp\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\","
    backup_data+="\"status\": \"pending\""
    backup_data+="}"
    
    echo "$backup_data" > "$backup_file"
    
    MIGRATION_BACKUP_FILE="$backup_file"
    
    log_success "Backup created: $backup_file"
}

# ==============================================================================
# Migration Execution
# ==============================================================================

execute_full_migration() {
    log_info "Executing full migration from $FROM_VERSION to $TO_VERSION..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would execute full migration"
        return 0
    fi
    
    # Generate migration hash
    local migration_hash
    migration_hash=$(printf '%064d' "$TO_VERSION")
    
    local result
    result=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$MIGRATE_NETWORK" \
        -- migrate \
        --target_version "$TO_VERSION" \
        --migration_hash "$migration_hash" 2>&1)
    
    if [[ $? -eq 0 ]]; then
        log_success "Full migration completed"
        return 0
    else
        log_error "Full migration failed: $result"
        return 1
    fi
}

execute_incremental_migration() {
    log_info "Executing incremental migration..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would execute incremental migration"
        return 0
    fi
    
    local current=$FROM_VERSION
    local step_count=1
    local total_steps=${#MIGRATION_STEPS[@]}
    
    for step in "${MIGRATION_STEPS[@]}"; do
        log_info "Step $step_count/$total_steps: $step"
        
        # Extract target version from step
        local target
        target=$(echo "$step" | grep -oE '[0-9]+$' | head -1)
        
        if [[ -z "$target" ]]; then
            target=$TO_VERSION
        fi
        
        local migration_hash
        migration_hash=$(printf '%064d' "$target")
        
        local result
        result=$(stellar contract invoke \
            --id "$CONTRACT_ID" \
            --source-account "$STELLAR_SECRET_KEY" \
            --network "$MIGRATE_NETWORK" \
            -- migrate \
            --target_version "$target" \
            --migration_hash "$migration_hash" 2>&1)
        
        if [[ $? -ne 0 ]]; then
            log_error "Migration step failed: $result"
            return 1
        fi
        
        log_success "Step $step_count completed"
        
        # Wait between steps
        if [[ $step_count -lt $total_steps ]]; then
            log_info "Waiting 5 seconds before next step..."
            sleep 5
        fi
        
        ((step_count++))
    done
    
    log_success "Incremental migration completed"
}

execute_batch_migration() {
    log_info "Executing batch migration with batch size $BATCH_SIZE..."
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would execute batch migration"
        return 0
    fi
    
    # For contracts that support batch migration
    # This would typically iterate over stored data and migrate in batches
    
    log_info "Note: Batch migration depends on contract implementation"
    log_info "Falling back to full migration..."
    
    execute_full_migration
}

run_migration() {
    print_banner "Executing Migration"
    
    case "$MIGRATION_STRATEGY" in
        full)
            execute_full_migration
            ;;
        incremental)
            execute_incremental_migration
            ;;
        batch)
            execute_batch_migration
            ;;
    esac
}

# ==============================================================================
# Post-Migration Verification
# ==============================================================================

verify_migration() {
    print_banner "Verifying Migration"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would verify migration"
        return 0
    fi
    
    # Wait for confirmation
    log_info "Waiting for migration confirmation..."
    sleep 5
    
    # Verify new version
    log_info "Verifying contract version..."
    
    local actual_version
    actual_version=$(get_contract_version "$CONTRACT_ID")
    
    if [[ "$actual_version" == "$TO_VERSION" ]]; then
        log_success "Version verified: $actual_version"
    else
        log_error "Version mismatch! Expected: $TO_VERSION, Got: $actual_version"
        return 1
    fi
    
    # Check migration state if available
    log_info "Checking migration state..."
    
    local migration_state
    migration_state=$(stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source-account "$STELLAR_SECRET_KEY" \
        --network "$MIGRATE_NETWORK" \
        --send=no \
        -- get_migration_state 2>&1) || true
    
    if [[ -n "$migration_state" && "$migration_state" != *"error"* ]]; then
        log_info "Migration state: $migration_state"
    else
        log_debug "Migration state not available or function not implemented"
    fi
    
    # Update backup with success status
    if [[ -n "$MIGRATION_BACKUP_FILE" && -f "$MIGRATION_BACKUP_FILE" ]]; then
        local tmp_file="$MIGRATION_BACKUP_FILE.tmp"
        if command -v jq &> /dev/null; then
            jq '.status = "completed"' "$MIGRATION_BACKUP_FILE" > "$tmp_file"
            mv "$tmp_file" "$MIGRATION_BACKUP_FILE"
        fi
    fi
    
    log_success "Migration verification complete"
}

# ==============================================================================
# Main Function
# ==============================================================================

main() {
    parse_args "$@"
    
    load_env
    
    NETWORK="$MIGRATE_NETWORK"
    setup_network "$MIGRATE_NETWORK"
    init_deployments_dir
    
    print_banner "Grainlify Contract Migration"
    
    # Validate environment
    check_stellar_cli || exit 1
    
    if [[ -z "$STELLAR_SECRET_KEY" ]]; then
        log_error "STELLAR_SECRET_KEY is required"
        exit 1
    fi
    
    # Validate parameters
    validate_migration_params
    
    # Generate and display migration plan
    generate_migration_plan
    
    # Safety confirmation
    if [[ "$MIGRATE_NETWORK" == "mainnet" ]]; then
        echo -e "${RED}WARNING: You are about to migrate a contract on MAINNET!${NC}"
        echo "This operation modifies contract state and may be irreversible."
        echo ""
        if ! confirm_action "Are you sure you want to continue?"; then
            exit 0
        fi
    elif [[ "$SKIP_CONFIRM" != "true" ]]; then
        if ! confirm_action "Ready to migrate. Continue?"; then
            exit 0
        fi
    fi
    
    # Pre-migration backup
    pre_migration_backup
    
    # Execute migration
    run_migration || {
        log_error "Migration failed!"
        log_info "Check backup file: $MIGRATION_BACKUP_FILE"
        exit 1
    }
    
    # Verify migration
    verify_migration || {
        log_error "Migration verification failed!"
        log_info "Consider running rollback"
        exit 1
    }
    
    # Update deployment records
    save_deployment "$MIGRATE_NETWORK" "$MIGRATE_CONTRACT" "$CONTRACT_ID" "" "$TO_VERSION"
    
    # Final summary
    print_banner "Migration Complete"
    echo "Contract:     $MIGRATE_CONTRACT"
    echo "Contract ID:  $CONTRACT_ID"
    echo "Network:      $MIGRATE_NETWORK"
    echo "Old Version:  $FROM_VERSION"
    echo "New Version:  $TO_VERSION"
    echo "Strategy:     $MIGRATION_STRATEGY"
    print_separator
    
    log_success "Migration complete!"
}

main "$@"
