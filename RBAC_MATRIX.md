# RBAC Permission Matrix

## Overview
This document defines the Role-Based Access Control (RBAC) matrix for the Grainlify Program Escrow contract. It maps each role to the operations and permissions they have.

## Role Definitions

### 1. **Admin** (Full Control)
- **Purpose**: System administrator with complete control
- **Scope**: All operations
- **Auto-granted**: To the address provided during contract initialization

**Permissions:**
| Operation | Admin | Operator | Pauser | Viewer |
|-----------|:-----:|:--------:|:------:|:------:|
| Initialize Contract | ✅ | ❌ | ❌ | ❌ |
| Pause Contract | ✅ | ❌ | ❌ | ❌ |
| Unpause Contract | ✅ | ❌ | ❌ | ❌ |
| Grant Roles | ✅ | ❌ | ❌ | ❌ |
| Revoke Roles | ✅ | ❌ | ❌ | ❌ |
| Lock Funds | ✅ | ✅ | ❌ | ❌ |
| Single Payout | ✅ | ✅ | ❌ | ❌ |
| Batch Payout | ✅ | ✅ | ❌ | ❌ |
| Create Release Schedule | ✅ | ✅ | ❌ | ❌ |
| Manage Release | ✅ | ✅ | ❌ | ❌ |
| Update Fee Configuration | ✅ | ❌ | ❌ | ❌ |
| Update Amount Limits | ✅ | ❌ | ❌ | ❌ |
| Manage Whitelist | ✅ | ❌ | ❌ | ❌ |
| View Program Info | ✅ | ✅ | ✅ | ✅ |
| View Balance | ✅ | ✅ | ✅ | ✅ |
| View Payout History | ✅ | ✅ | ✅ | ✅ |

### 2. **Operator** (Day-to-Day Operations)
- **Purpose**: Execute routine operational tasks
- **Scope**: Fund management and payouts
- **Auto-granted**: No (must be explicitly granted by Admin)

**Permissions:**
- Lock funds into programs
- Execute single payouts
- Execute batch payouts
- Create program release schedules
- Manage release operations
- View all program information and history
- **Cannot**: Pause contract, grant roles, configure fees, manage whitelist

### 3. **Pauser** (Emergency Controls)
- **Purpose**: Emergency response capability
- **Scope**: Pause operations only
- **Auto-granted**: No (must be explicitly granted by Admin)

**Permissions:**
- Pause contract (with Admin role also required to unpause)
- View program information
- **Cannot**: Execute payouts, lock funds, grant roles, or unpause

### 4. **Viewer** (Read-Only Access)
- **Purpose**: Audit and monitoring
- **Scope**: View-only access
- **Auto-granted**: No (must be explicitly granted by Admin)

**Permissions:**
- View program information
- View balance information
- View payout history
- **Cannot**: Execute any write operations

---

## Permission Matrix Summary

```
┌─────────────────────────────────┬────────┬──────────┬────────┬────────┐
│ Operation                       │ Admin  │ Operator │ Pauser │ Viewer │
├─────────────────────────────────┼────────┼──────────┼────────┼────────┤
│ Admin/Config Operations         │   ✅   │    ❌    │   ❌   │   ❌   │
│ Fund Operations (Lock/Payout)   │   ✅   │    ✅    │   ❌   │   ❌   │
│ Emergency Pause/Unpause         │   ✅   │    ❌    │   ✅*  │   ❌   │
│ View Operations                 │   ✅   │    ✅    │   ✅   │   ✅   │
└─────────────────────────────────┴────────┴──────────┴────────┴────────┘
* Pauser can pause but only Admin can unpause
```

---

## Role Hierarchy

The roles form a **capability hierarchy** rather than a strict role hierarchy:

```
Admin (Superset of all permissions)
 ├─ Includes Operator capabilities
 │   ├─ Lock funds
 │   ├─ Manage payouts
 │   └─ Create schedules
 ├─ Includes Pauser capabilities
 │   └─ Emergency pause
 └─ Exclusive Admin capabilities
     ├─ Grant/Revoke roles
     ├─ Configure fees
     ├─ Manage whitelist
     └─ Unpause contract

Operator (Operations subset)
 └─ Cannot perform admin or pause operations

Pauser (Emergency subset)
 └─ Can only pause; cannot unpause or perform operations

Viewer (Read-only)
 └─ No write permissions
```

---

## Code Implementation Details

### Role Enforcement Functions
Located in `src/rbac.rs`:

```rust
/// Require exact Admin role
pub fn require_admin(env: &Env, address: &Address)

/// Require Operator or Admin role
pub fn require_operator(env: &Env, address: &Address)

/// Require Pauser or Admin role (for pause operations)
pub fn require_pauser(env: &Env, address: &Address)

/// Check capabilities without panic
pub fn is_admin(env: &Env, address: &Address) -> bool
pub fn is_operator(env: &Env, address: &Address) -> bool
pub fn can_pause(env: &Env, address: &Address) -> bool
```

### Role Storage
- Roles stored as `Map<Address, Symbol>` in contract instance storage
- Key: `RBAC_ROLES` (symbol_short "rbac")
- Value: Serialized role symbol
- Persistent across contract calls

---

## Integration Points

### Pause/Unpause Operations
- **`pause_contract(env, caller)`**: Requires `Pauser` or `Admin` role
- **`unpause_contract(env, caller)`**: Requires `Admin` role only

### Fund Operations
- **`lock_program_funds(...)`**: Requires `Operator` or `Admin` role
- **`single_payout(...)`**: Requires `Operator` or `Admin` role
- **`batch_payout(...)`**: Requires `Operator` or `Admin` role

### Role Management
- **`grant_role(env, address, role)`**: Requires `Admin` role
- **`revoke_role(env, address)`**: Requires `Admin` role
- **`get_role(env, address)`**: No permission required (read-only)

---

## Security Considerations

1. **Admin Auto-Grant**: The initializer address is automatically granted Admin role on contract initialization
2. **Role Revocation**: Revoking an address's role removes all permissions for that address
3. **Multiple Admins**: Multiple addresses can be granted Admin role
4. **No Default Roles**: Only Admin is auto-granted; all other roles require explicit assignment
5. **Immutable Enforcement**: Role checks are enforced at runtime before operations execute
6. **Emergency Pause**: Pauser role enables emergency stopping without ability to modify state

---

## Usage Examples

### Granting an Operator
```rust
// Only Admin can grant roles
ProgramEscrowContract::grant_role(&env, &operator_address, Role::Operator);
```

### Emergency Pause
```rust
// Pauser can pause (no other permissions needed)
ProgramEscrowContract::pause_contract(env, pauser_address);

// Only Admin can unpause
ProgramEscrowContract::unpause_contract(env, admin_address);
```

### Checking Permissions
```rust
// Check if address is operator
if crate::rbac::is_operator(&env, &address) {
    // Can execute operator operations
}

// Require specific role or panic
crate::rbac::require_admin(&env, &address);
```

---

## Testing Coverage

Unit tests validate:
- ✅ Role enum construction and conversion
- ✅ Role parsing from strings
- ✅ Role equality comparison
- ✅ Role symbol conversion
- ✅ Permission enforcement functions
- ✅ Role storage persistence

All tests pass: **10/10 ✅**

---

## Future Enhancements

Potential improvements for future versions:
1. **Time-based Roles**: Roles that expire after a certain period
2. **Delegated Authority**: Allow operators to delegate to sub-operators
3. **Audit Logging**: Enhanced logging of role changes and permission checks
4. **Multi-sig Administration**: Require multiple admins to approve critical operations
5. **Role-specific Limits**: Different payout limits per role
